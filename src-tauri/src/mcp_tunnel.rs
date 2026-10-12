//! User initiated supervisor for OpenAI's Secure MCP Tunnel runtime.
//! No tunnel is started at app launch and no remote resource is created here.

use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::Mutex,
    time::Duration,
};

use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

const CONFIG_FILE: &str = "mcp-tunnel-config.dpapi";
const HEALTH_URL_FILE: &str = "mcp-tunnel-health.url";
const TUNNEL_RUNTIME_VERSION: &str = "0.0.16";

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelStatus {
    pub status: &'static str,
    pub configured: bool,
    pub tunnel_id: Option<String>,
    pub runtime_available: bool,
}

#[derive(Deserialize, Serialize)]
struct TunnelConfig {
    tunnel_id: String,
    runtime_key: String,
}

impl Drop for TunnelConfig {
    fn drop(&mut self) {
        self.runtime_key.zeroize();
    }
}

#[derive(Default)]
struct RuntimeState {
    child: Option<Child>,
    phase: &'static str,
}

#[derive(Default)]
pub struct McpTunnelService(Mutex<RuntimeState>);

impl Drop for McpTunnelService {
    fn drop(&mut self) {
        if let Ok(state) = self.0.get_mut() {
            stop_child(&mut state.child);
        }
    }
}

fn root() -> Result<PathBuf, String> {
    let root = std::env::var_os("LOCALAPPDATA").ok_or("Local app storage is unavailable.")?;
    let root = PathBuf::from(root);
    if !root.is_absolute() {
        return Err("Local app storage path is invalid.".into());
    }
    Ok(root.join("AudioRouter"))
}

fn check_local_path(path: &Path) -> Result<(), String> {
    use std::os::windows::fs::MetadataExt;
    if !path.is_absolute() {
        return Err("AudioRouter tunnel storage path is invalid.".into());
    }
    for ancestor in path.ancestors() {
        match std::fs::symlink_metadata(ancestor) {
            Ok(meta) if meta.file_attributes() & 0x400 != 0 => {
                return Err("AudioRouter tunnel storage cannot use redirected paths.".into())
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err("Cannot inspect AudioRouter tunnel storage.".into()),
        }
    }
    Ok(())
}

fn config_path() -> Result<PathBuf, String> {
    Ok(root()?.join(CONFIG_FILE))
}

fn health_path() -> Result<PathBuf, String> {
    Ok(root()?.join(HEALTH_URL_FILE))
}

fn valid_tunnel_id(value: &str) -> bool {
    let Some(id) = value.strip_prefix("tunnel_") else {
        return false;
    };
    let chars = |value: &str| {
        value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    };
    if id.len() == 32 {
        return chars(id);
    }
    id.len() == 37 && id.as_bytes().get(4) == Some(&b'_') && chars(&id[..4]) && chars(&id[5..])
}

fn valid_runtime_key(value: &str) -> bool {
    (20..=2048).contains(&value.len()) && value.bytes().all(|byte| (0x21..=0x7e).contains(&byte))
}

fn read_config() -> Option<TunnelConfig> {
    let path = config_path().ok()?;
    check_local_path(&path).ok()?;
    let mut bytes = crate::api_token::load_secret(&path).ok()?;
    let decoded = serde_json::from_slice(&bytes);
    bytes.zeroize();
    let config: TunnelConfig = decoded.ok()?;
    (valid_tunnel_id(&config.tunnel_id) && valid_runtime_key(&config.runtime_key)).then_some(config)
}

fn save_config(tunnel_id: String, runtime_key: String) -> Result<(), String> {
    let path = config_path()?;
    check_local_path(&path)?;
    let config = TunnelConfig {
        tunnel_id,
        runtime_key,
    };
    let mut bytes = serde_json::to_vec(&config).map_err(|_| "Cannot save tunnel settings.")?;
    let result = crate::api_token::save_secret(&path, &bytes);
    bytes.zeroize();
    result
}

pub fn configure(tunnel_id: String, mut runtime_key: String) -> Result<(), String> {
    let tunnel_id = tunnel_id.trim().to_owned();
    if !valid_tunnel_id(&tunnel_id) {
        runtime_key.clear();
        return Err("Enter the tunnel ID shown on the OpenAI Platform Tunnels page.".into());
    }
    if !valid_runtime_key(&runtime_key) {
        runtime_key.zeroize();
        return Err(
            "Enter a valid restricted runtime key with Tunnels Read and Use access.".into(),
        );
    }
    save_config(tunnel_id, runtime_key)
}

pub fn status(runtime_executable: &Path) -> TunnelStatus {
    let config = read_config();
    let runtime_available = runtime_executable.is_file();
    let has_key = config.is_some();
    let mut result = TunnelStatus {
        status: "notConfigured",
        configured: config.is_some() && has_key,
        tunnel_id: config.as_ref().map(|value| value.tunnel_id.clone()),
        runtime_available,
    };
    if config.is_none() {
        return result;
    }
    if !has_key {
        result.status = "needsSetup";
        return result;
    }
    result.status = "disconnected";
    result
}

pub fn start(
    service: &McpTunnelService,
    runtime_executable: &Path,
    cli: &Path,
    database: &Path,
    pipe: &str,
) -> Result<TunnelStatus, String> {
    if !runtime_executable.is_file() {
        return Err(format!(
            "The bundled OpenAI tunnel runtime v{TUNNEL_RUNTIME_VERSION} is missing."
        ));
    }
    if !cli.is_file() || !database.is_absolute() || pipe.is_empty() {
        return Err("AudioRouter's local MCP connection is unavailable.".into());
    }
    let mut config = read_config().ok_or("Enter your OpenAI tunnel details first.")?;

    let mut state = service
        .0
        .lock()
        .map_err(|_| "Tunnel status is unavailable.")?;
    if let Some(child) = state.child.as_mut() {
        if child
            .try_wait()
            .map_err(|_| "Cannot check the tunnel process.")?
            .is_none()
        {
            return Ok(TunnelStatus {
                status: if ready() { "connected" } else { "connecting" },
                configured: true,
                tunnel_id: Some(config.tunnel_id.clone()),
                runtime_available: true,
            });
        }
    }
    stop_child(&mut state.child);

    let health_url = health_path()?;
    check_local_path(&health_url)?;
    if health_url.exists() {
        std::fs::remove_file(&health_url).map_err(|_| "Cannot reset tunnel health status.")?;
    }
    let command = format!(
        "{} mcp serve --client-id audiorouter-chatgpt --database {}",
        quote_command(cli),
        quote_command(database)
    );
    let spawn_result = Command::new(runtime_executable)
        .arg("run")
        .arg(format!("--control-plane.tunnel-id={}", config.tunnel_id))
        .arg("--control-plane.api-key=env:CONTROL_PLANE_API_KEY")
        .arg(format!("--mcp.command={command}"))
        .arg("--health.listen-addr=127.0.0.1:0")
        .arg(format!("--health.url-file={}", health_url.display()))
        .arg("--log.level=info")
        .env("CONTROL_PLANE_API_KEY", &config.runtime_key)
        .env_remove("OPENAI_API_KEY")
        .env("AUDIOROUTER_TUNNEL_CHILD", "1")
        .env("AUDIOROUTER_CONTROL_PIPE", pipe)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    config.runtime_key.zeroize();
    let child = spawn_result.map_err(|_| "Could not start the OpenAI tunnel runtime.")?;
    state.child = Some(child);
    state.phase = "connecting";
    Ok(TunnelStatus {
        status: state.phase,
        configured: true,
        tunnel_id: Some(config.tunnel_id.clone()),
        runtime_available: true,
    })
}

pub fn current_status(service: &McpTunnelService, runtime_executable: &Path) -> TunnelStatus {
    let mut status = status(runtime_executable);
    if !status.configured {
        return status;
    }
    let Ok(mut state) = service.0.lock() else {
        status.status = "actionNeeded";
        return status;
    };
    let Some(child) = state.child.as_mut() else {
        return status;
    };
    match child.try_wait() {
        Ok(None) => {
            status.status = if ready() {
                state.phase = "connected";
                "connected"
            } else {
                state.phase = "connecting";
                "connecting"
            };
        }
        Ok(Some(_)) => {
            state.child = None;
            state.phase = "actionNeeded";
            status.status = state.phase;
        }
        Err(_) => status.status = "actionNeeded",
    }
    status
}

pub fn disconnect(service: &McpTunnelService) -> Result<(), String> {
    let mut state = service
        .0
        .lock()
        .map_err(|_| "Tunnel status is unavailable.")?;
    stop_child(&mut state.child);
    state.phase = "disconnected";
    if let Ok(path) = health_path() {
        let _ = std::fs::remove_file(path);
    }
    Ok(())
}

pub fn forget(service: &McpTunnelService) -> Result<(), String> {
    disconnect(service)?;
    let path = config_path()?;
    check_local_path(&path)?;
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err("Could not remove the saved OpenAI tunnel details.".into()),
    }
}

fn quote_command(value: &Path) -> String {
    // tunnel-client's Windows stdio command parser treats backslashes as
    // escapes; its supported Windows examples use forward slashes.
    let value = value.to_string_lossy().replace('\\', "/");
    format!("\"{}\"", value.replace('"', "\\\""))
}

fn stop_child(child: &mut Option<Child>) {
    if let Some(mut process) = child.take() {
        let _ = process.kill();
        let _ = process.wait();
    }
}

fn ready() -> bool {
    let Ok(path) = health_path() else {
        return false;
    };
    let Ok(metadata) = std::fs::symlink_metadata(&path) else {
        return false;
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > 512 {
        return false;
    }
    let Ok(url) = std::fs::read_to_string(path) else {
        return false;
    };
    let Some(address) = loopback_health_address(url.trim()) else {
        return false;
    };
    let Ok(mut stream) = TcpStream::connect_timeout(&address, Duration::from_millis(250)) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(250)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(250)));
    if stream
        .write_all(b"GET /readyz HTTP/1.0\r\nHost: 127.0.0.1\r\n\r\n")
        .is_err()
    {
        return false;
    }
    let mut response = [0u8; 256];
    let Ok(count) = stream.read(&mut response) else {
        return false;
    };
    response[..count].starts_with(b"HTTP/1.1 200") || response[..count].starts_with(b"HTTP/1.0 200")
}

fn loopback_health_address(url: &str) -> Option<SocketAddr> {
    let address = url.strip_prefix("http://127.0.0.1:")?;
    let port = address.split('/').next()?.parse::<u16>().ok()?;
    (port != 0).then_some(SocketAddr::from(([127, 0, 0, 1], port)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_supported_tunnel_ids_and_runtime_keys() {
        assert!(valid_tunnel_id("tunnel_0123456789abcdef0123456789abcdef"));
        assert!(valid_tunnel_id(
            "tunnel_ab12_0123456789abcdef0123456789abcdef"
        ));
        assert!(!valid_tunnel_id(
            "https://example.test/tunnel_0123456789abcdef0123456789abcdef"
        ));
        assert!(!valid_tunnel_id("tunnel_0123456789ABCDEF0123456789abcdef"));
        assert!(!valid_runtime_key("short"));
        assert!(!valid_runtime_key(&"x".repeat(20).replace('x', "\n")));
        assert!(valid_runtime_key(&"k".repeat(64)));
    }

    #[test]
    fn readiness_url_is_loopback_only() {
        assert_eq!(
            loopback_health_address("http://127.0.0.1:1234"),
            Some(([127, 0, 0, 1], 1234).into())
        );
        assert!(loopback_health_address("http://localhost:1234").is_none());
        assert!(loopback_health_address("http://192.168.1.2:1234").is_none());
        assert!(loopback_health_address("https://127.0.0.1:1234").is_none());
    }
}
