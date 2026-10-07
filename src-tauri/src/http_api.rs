//! Optional loopback adapter. No state or audio execution is owned here.
use crate::http_accept::{close_gracefully, reply_with, ConnectionPool};
use audiorouter_protocol::{JsonRpcRequest, JsonRpcResponse};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, TcpListener, TcpStream};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};

const MAX_BODY: usize = 4 * 1024 * 1024;
const MAX_HEADERS: usize = 16 * 1024;
/// Forward one request to the backend with its correlation ID (P2-3).
pub type Forward = dyn Fn(&JsonRpcRequest, &str) -> Result<JsonRpcResponse, String> + Send + Sync;

pub struct HttpApi {
    pub port: u16,
    pub lan: Option<Ipv4Addr>,
    pub token: String,
    /// Dropping the pool stops the listeners and workers.
    _pool: ConnectionPool,
}
impl HttpApi {
    #[cfg(test)]
    pub fn start(port: u16, forward: Arc<Forward>) -> Result<Self, String> {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).map_err(|_| "Cannot securely generate API token")?;
        Self::start_with_token(
            port,
            bytes.iter().map(|b| format!("{b:02x}")).collect(),
            forward,
        )
    }
    #[cfg(test)]
    pub fn start_with_token(
        port: u16,
        token: String,
        forward: Arc<Forward>,
    ) -> Result<Self, String> {
        Self::start_on(port, token, None, forward)
    }
    /// Loopback is always served. `lan` adds one listener on that exact
    /// private or link-local address of this PC (HTTP-09); peers on it must
    /// themselves be private or link-local. Never binds every interface.
    pub fn start_on(
        port: u16,
        token: String,
        lan: Option<Ipv4Addr>,
        forward: Arc<Forward>,
    ) -> Result<Self, String> {
        if token.len() != 64 || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("Invalid API token".into());
        }
        if let Some(address) = lan {
            if !lan_address_allowed(address) {
                return Err(format!("{address} is not a private local-network address. Choose this PC's home or office network address."));
            }
        }
        // Listeners stay blocking: each accept thread waits in the kernel
        // until a connection arrives, with no idle wake-ups or polling delay.
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port))
            .map_err(|_| format!("Cannot open localhost port {port}. Choose another port or stop the application using it."))?;
        let port = listener
            .local_addr()
            .map_err(|_| "Cannot read API port")?
            .port();
        let mut listeners = vec![(listener, false)];
        if let Some(address) = lan {
            let network = TcpListener::bind((address, port))
                .map_err(|_| format!("Cannot open port {port} on {address}. Check that this PC still has that address, or choose another port."))?;
            listeners.push((network, true));
        }
        let mut hosts = vec![format!("127.0.0.1:{port}")];
        hosts.extend(lan.map(|address| format!("{address}:{port}")));
        let describe = forward(
            &rpc("system.describe", None),
            &audiorouter_protocol::diagnostics::new_correlation_id(),
        )?
        .result
        .ok_or("Backend discovery unavailable; reconnect before starting API")?;
        let schema = openapi(&describe, port, lan);
        let rate = Mutex::new((Instant::now(), 40f64));
        let secret = token.clone();
        let pool = ConnectionPool::start(
            listeners,
            peer_allowed,
            Arc::new(move |stream: &mut TcpStream, stop: &AtomicBool| {
                let _ = handle(stream, &hosts, &secret, &schema, &forward, stop, &rate);
            }),
        )?;
        Ok(Self {
            port,
            lan,
            token,
            _pool: pool,
        })
    }
}

fn rpc(method: &str, params: Option<Value>) -> JsonRpcRequest {
    JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!("localhost-http")),
        method: method.into(),
        params,
    }
}
/// Addresses a local-network listener may use: RFC 1918 private ranges and
/// IPv4 link-local (a direct cable). Public, CGNAT and VPN-overlay addresses
/// are refused so a listener cannot face the internet by mistake.
pub fn lan_address_allowed(address: Ipv4Addr) -> bool {
    address.is_private() || address.is_link_local()
}
/// The loopback listener serves only this PC; the network listener serves
/// this PC and private/link-local peers.
fn peer_allowed(peer: IpAddr, network: bool) -> bool {
    let peer = match peer {
        IpAddr::V4(peer) => peer,
        IpAddr::V6(peer) if peer.is_loopback() => return true,
        IpAddr::V6(peer) => match peer.to_ipv4_mapped() {
            Some(peer) => peer,
            None => return false,
        },
    };
    peer.is_loopback() || (network && lan_address_allowed(peer))
}
/// Exact Host check: one of the listener addresses with the active port.
/// A browser Origin, when sent, must be the same Host.
fn host_allowed(host: Option<&str>, origin: Option<&str>, hosts: &[String]) -> bool {
    let Some(host) = host.filter(|host| hosts.iter().any(|allowed| allowed == host)) else {
        return false;
    };
    origin.is_none_or(|origin| origin == format!("http://{host}"))
}
/// Methods only the desktop window may call: approving a recording folder is
/// the user's own file-root decision (REC-07, SEC authorization UX), so a
/// token holder on localhost cannot redirect recordings.
pub const DESKTOP_ONLY_METHODS: &[&str] = &["recordings.setRoot", "devices.setAccess"];

pub fn operation_path(name: &str) -> String {
    format!("/api/v1/{}", name.replace('.', "/"))
}
pub fn openapi(describe: &Value, port: u16, lan: Option<Ipv4Addr>) -> Value {
    let mut paths = serde_json::Map::new();
    for method in describe["methods"].as_array().into_iter().flatten() {
        let Some(name) = method["name"].as_str() else {
            continue;
        };
        if DESKTOP_ONLY_METHODS.contains(&name) {
            continue;
        }
        paths.insert(operation_path(name), json!({"post": {
            "operationId": name, "tags": [name.split('.').next().unwrap_or("API")],
            "summary": method["description"],
            "description": format!("Permission: {}. Side effect: {}. Backend validation remains authoritative.", method["permission"], method["sideEffect"]),
            "requestBody": {"required": true, "content": {"application/json": {"schema": method["inputSchema"]}}},
            "responses": {"200": {"description": "Backend result; inspect activation for durable graph commits", "content": {"application/json": {"schema": method["outputSchema"]}}}, "400": {"description": "Invalid request"}, "403": {"description": "Permission denied"}, "409": {"description": "Revision conflict"}, "429": {"description": "Rate limited"}, "503": {"description": "Backend unavailable"}},
            "security": [{"bearerAuth": []}]
        }}));
    }
    for (path, name) in [
        ("capabilities", "system.describe"),
        ("sessions", "sessions.list"),
        ("status", "status.get"),
    ] {
        let schema = describe["methods"]
            .as_array()
            .and_then(|methods| methods.iter().find(|method| method["name"] == name))
            .map(|method| method["outputSchema"].clone())
            .unwrap_or(json!({}));
        paths.insert(format!("/api/v1/{path}"), json!({"get": {"operationId": format!("http.{path}"), "security": [{"bearerAuth": []}], "responses": {"200": {"description": "Backend result", "content": {"application/json": {"schema": schema}}}}}}));
    }
    let active_get = describe["methods"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|method| method["name"] == "sessions.active.get")
        .map(|method| method["outputSchema"].clone())
        .unwrap_or(json!({}));
    let active_set = describe["methods"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|method| method["name"] == "sessions.active.set");
    let active_path = paths
        .entry("/api/v1/sessions/active")
        .or_insert_with(|| json!({}));
    active_path["get"] = json!({ "operationId": "http.sessions.active", "responses": { "200": { "description": "Currently selected editing session; audio is not started", "content": { "application/json": { "schema": active_get } } } }, "security": [{"bearerAuth": []}] });
    active_path["put"] = json!({ "operationId": "http.sessions.activate", "summary": "Select the editing session without starting audio", "requestBody": { "required": true, "content": { "application/json": { "schema": active_set.map(|method| method["inputSchema"].clone()).unwrap_or(json!({})) } } }, "responses": { "200": { "description": "Selected session", "content": { "application/json": { "schema": active_set.map(|method| method["outputSchema"].clone()).unwrap_or(json!({})) } } }, "400": { "description": "Unknown session"}, "403": { "description": "Permission denied"} }, "security": [{"bearerAuth": []}] });
    json!({"openapi": "3.1.0", "info": {"title": "AudioRouter local API", "version": "1.0.0"}, "servers": std::iter::once(format!("http://127.0.0.1:{port}")).chain(lan.map(|address| format!("http://{address}:{port}"))).map(|url| json!({"url": url})).collect::<Vec<_>>(), "paths": paths, "components": {"securitySchemes": {"bearerAuth": {"type": "http", "scheme": "bearer"}}}})
}

fn reply(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
) -> std::io::Result<()> {
    reply_with_id(stream, status, content_type, body, None)
}

/// `request_id` is echoed as `X-Request-Id`; it is always a validated
/// correlation ID (`[A-Za-z0-9-]`, at most 32 characters), so it cannot
/// split or inject header lines.
fn reply_with_id(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
    request_id: Option<&str>,
) -> std::io::Result<()> {
    let request_id = audiorouter_protocol::diagnostics::valid_correlation_id(request_id)
        .map(|id| format!("X-Request-Id: {id}\r\n"))
        .unwrap_or_default();
    reply_with(stream, status, content_type, &request_id, body)
}
fn fail(stream: &mut TcpStream, status: u16, message: &str) -> std::io::Result<()> {
    reply(
        stream,
        status,
        "application/json",
        json!({"error": {"message": message}})
            .to_string()
            .as_bytes(),
    )?;
    close_gracefully(
        stream,
        Duration::from_millis(20),
        Duration::from_millis(50),
        MAX_BODY,
    );
    Ok(())
}
fn token_matches(actual: &str, token: &str) -> bool {
    let expected = format!("Bearer {token}");
    actual.len() == expected.len()
        && actual
            .as_bytes()
            .iter()
            .zip(expected.as_bytes())
            .fold(0u8, |diff, (a, b)| diff | (a ^ b))
            == 0
}
fn handle(
    stream: &mut TcpStream,
    hosts: &[String],
    token: &str,
    schema: &Value,
    forward: &Arc<Forward>,
    stop: &AtomicBool,
    rate: &Mutex<(Instant, f64)>,
) -> std::io::Result<()> {
    // Listeners are blocking, but keep accepted sockets explicitly blocking:
    // clients send headers/body in separate packets, and bounded blocking
    // reads must wait for them instead of treating WouldBlock as disconnect.
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let started = Instant::now();
    let mut bytes = Vec::with_capacity(1024);
    while !bytes.ends_with(b"\r\n\r\n") {
        if bytes.len() >= MAX_HEADERS || started.elapsed() > Duration::from_secs(2) {
            return fail(stream, 400, "Request headers too large or too slow");
        }
        let mut byte = [0];
        stream.read_exact(&mut byte)?;
        bytes.push(byte[0]);
    }
    let Ok(headers) = std::str::from_utf8(&bytes) else {
        return fail(stream, 400, "Invalid HTTP headers");
    };
    let mut lines = headers.split("\r\n");
    let first = lines
        .next()
        .unwrap_or_default()
        .split(' ')
        .collect::<Vec<_>>();
    if first.len() != 3 || first[2] != "HTTP/1.1" {
        return fail(stream, 400, "HTTP/1.1 required");
    }
    let (verb, path) = (first[0], first[1]);
    let mut fields = std::collections::HashMap::new();
    for line in lines.filter(|line| !line.is_empty()) {
        let Some((key, value)) = line.split_once(':') else {
            return fail(stream, 400, "Invalid header");
        };
        if key.trim() != key
            || fields
                .insert(key.to_ascii_lowercase(), value.trim())
                .is_some()
        {
            return fail(stream, 400, "Duplicate or invalid header");
        }
    }
    if !host_allowed(
        fields.get("host").copied(),
        fields.get("origin").copied(),
        hosts,
    ) {
        return fail(
            stream,
            403,
            "Only this API's exact Host and same-origin requests are allowed",
        );
    }
    if fields.contains_key("transfer-encoding") || path.contains('?') {
        return fail(
            stream,
            400,
            "Chunked bodies and query strings are unsupported; use JSON parameters",
        );
    }
    if verb == "GET" {
        match path {
            "/docs" | "/docs/" => return reply(stream, 200, "text/html; charset=utf-8", b"<!doctype html><html><head><meta charset='utf-8'><title>AudioRouter API</title><link rel='stylesheet' href='/swagger-ui.css'></head><body><p><a href='/licenses/swagger-ui.txt'>Bundled Swagger UI licenses</a></p><div id='swagger-ui'></div><script src='/swagger-ui-bundle.js'></script><script src='/swagger-init.js'></script></body></html>"),
            "/swagger-ui.css" => return reply(stream, 200, "text/css", include_bytes!("../http-assets/swagger-ui.css")),
            "/swagger-ui-bundle.js" => return reply(stream, 200, "text/javascript", include_bytes!("../http-assets/swagger-ui-bundle.js")),
            "/swagger-init.js" => return reply(stream, 200, "text/javascript", b"SwaggerUIBundle({url:'/openapi.json',dom_id:'#swagger-ui',validatorUrl:null,persistAuthorization:false,queryConfigEnabled:false});"),
            "/openapi.json" => return reply(stream, 200, "application/json", schema.to_string().as_bytes()),
            "/licenses/swagger-ui.txt" => return reply(stream, 200, "text/plain; charset=utf-8", concat!(include_str!("../http-assets/LICENSE"), "\n", include_str!("../http-assets/NOTICE"), "\n", include_str!("../http-assets/swagger-ui-bundle.js.LICENSE.txt")).as_bytes()),
            _ => {}
        }
    }
    if !fields
        .get("authorization")
        .is_some_and(|actual| token_matches(actual, token))
    {
        return fail(
            stream,
            401,
            "Enter the bearer token from AudioRouter's API tab",
        );
    }
    let method = match (verb, path) {
        ("GET", "/api/v1/capabilities") => "system.describe".into(),
        ("GET", "/api/v1/sessions") => "sessions.list".into(),
        ("GET", "/api/v1/sessions/active") => "sessions.active.get".into(),
        ("PUT", "/api/v1/sessions/active") => "sessions.active.set".into(),
        ("GET", "/api/v1/status") => "status.get".into(),
        ("POST", path) => {
            let Some(method) = path
                .strip_prefix("/api/v1/")
                .map(|name| name.replace('/', "."))
            else {
                return fail(stream, 404, "Unknown API resource");
            };
            if !audiorouter_domain::API_METHODS
                .iter()
                .any(|spec| spec.name == method)
            {
                return fail(stream, 404, "Unknown API operation");
            }
            if DESKTOP_ONLY_METHODS.contains(&method.as_str()) {
                return fail(
                    stream,
                    403,
                    "Choose the recording folder in the AudioRouter window",
                );
            }
            method
        }
        _ => return fail(stream, 405, "Use the documented HTTP method/resource"),
    };
    let length = match fields.get("content-length") {
        None => 0,
        Some(value) => match value.parse::<usize>() {
            Ok(length) => length,
            Err(_) => return fail(stream, 400, "Invalid Content-Length"),
        },
    };
    if length > MAX_BODY {
        return fail(stream, 413, "JSON body exceeds 4 MiB");
    }
    let params = if verb == "POST" || verb == "PUT" {
        if fields
            .get("content-type")
            .is_none_or(|value| value.split(';').next() != Some("application/json"))
        {
            return fail(stream, 400, "Content-Type must be application/json");
        }
        if let Some(expect) = fields.get("expect") {
            if !expect.eq_ignore_ascii_case("100-continue") {
                return fail(stream, 400, "Unsupported Expect header");
            }
            // Authenticate and bound Content-Length before allowing the body.
            stream.write_all(b"HTTP/1.1 100 Continue\r\n\r\n")?;
            stream.flush()?;
        }
        let mut body = vec![0u8; length];
        let mut offset = 0;
        while offset < length {
            if started.elapsed() > Duration::from_secs(2) {
                return fail(stream, 400, "Request body deadline exceeded");
            }
            let count = stream.read(&mut body[offset..])?;
            if count == 0 {
                return fail(stream, 400, "Incomplete body");
            }
            offset += count;
        }
        match serde_json::from_slice::<Value>(&body) {
            Ok(value) if value.is_object() => Some(value),
            _ => return fail(stream, 400, "Expected a bounded JSON parameter object"),
        }
    } else {
        if length != 0 {
            return fail(stream, 400, "GET cannot have a body");
        }
        None
    };
    if stop.load(Ordering::Acquire) {
        return fail(stream, 503, "API stopped");
    }
    // One correlation ID per HTTP request: the caller's `X-Request-Id` when
    // it is a valid ID, otherwise a fresh one. It is forwarded to the backend,
    // logged by the shell and backend, and returned in the response.
    let request_id = request_correlation_id(fields.get("x-request-id").copied());
    {
        let mut budget = rate.lock().unwrap_or_else(|error| error.into_inner());
        budget.1 = (budget.1 + budget.0.elapsed().as_secs_f64() * 20.0).min(40.0);
        budget.0 = Instant::now();
        let cost = if audiorouter_domain::API_METHODS.iter().any(|spec| {
            spec.name == method && spec.side_effect == audiorouter_domain::SideEffectClass::ReadOnly
        }) {
            0.1
        } else {
            1.0
        };
        if budget.1 < cost {
            return fail(stream, 429, "Local API rate exceeded; retry later");
        }
        budget.1 -= cost;
    }
    match forward(&rpc(&method, params), &request_id) {
        Ok(response) => match response.error {
            Some(error) => {
                let kind = error
                    .data
                    .as_ref()
                    .and_then(|data| data["code"].as_str())
                    .unwrap_or_default();
                let status = match kind {
                    "permissionDenied" | "clientRevoked" => 403,
                    "revisionConflict" => 409,
                    "rateLimited" => 429,
                    _ => 400,
                };
                reply_with_id(
                    stream,
                    status,
                    "application/json",
                    json!({"error": error}).to_string().as_bytes(),
                    Some(&request_id),
                )
            }
            None => reply_with_id(
                stream,
                200,
                "application/json",
                response
                    .result
                    .unwrap_or(Value::Null)
                    .to_string()
                    .as_bytes(),
                Some(&request_id),
            ),
        },
        Err(_) => reply_with_id(
            stream,
            503,
            "application/json",
            json!({"error": {"message": "Backend unavailable; reconnect in AudioRouter"}})
                .to_string()
                .as_bytes(),
            Some(&request_id),
        ),
    }
}

/// The caller's `X-Request-Id` when it is a valid correlation ID; anything
/// else is replaced (never echoed or logged).
fn request_correlation_id(header: Option<&str>) -> String {
    audiorouter_protocol::diagnostics::valid_correlation_id(header)
        .map(str::to_owned)
        .unwrap_or_else(audiorouter_protocol::diagnostics::new_correlation_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http_accept::{QUEUE, WORKERS};
    use audiorouter_control::ControlPlane;

    fn exchange(port: u16, request: &str) -> String {
        let mut stream = TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        stream.write_all(request.as_bytes()).unwrap();
        let mut result = String::new();
        stream.read_to_string(&mut result).unwrap();
        result
    }
    fn call(api: &HttpApi, path: &str, body: Option<Value>) -> (u16, Value) {
        let body = body.map(|body| body.to_string());
        let result = exchange(api.port, &format!("{} {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}", if body.is_some() { "POST" } else { "GET" }, api.port, api.token, body.as_ref().map_or(0, String::len), body.unwrap_or_default()));
        let (headers, body) = result.split_once("\r\n\r\n").unwrap();
        (
            headers.split(' ').nth(1).unwrap().parse().unwrap(),
            serde_json::from_str(body).unwrap(),
        )
    }
    fn fixture() -> (HttpApi, Arc<Forward>) {
        let (sender, receiver) = std::sync::mpsc::sync_channel::<(
            JsonRpcRequest,
            std::sync::mpsc::SyncSender<JsonRpcResponse>,
        )>(8);
        std::thread::spawn(move || {
            // The backend owns Windows COM objects on its original thread.
            let mut plane = ControlPlane::new("http-test");
            plane
                .insert_session(crate::default_desktop_session())
                .unwrap();
            for (request, reply) in receiver {
                let _ = reply.send(plane.dispatch(request));
            }
        });
        let forward: Arc<Forward> = Arc::new(move |request: &JsonRpcRequest, _request_id: &str| {
            let (reply, response) = std::sync::mpsc::sync_channel(1);
            sender
                .send((request.clone(), reply))
                .map_err(|_| "Backend closed")?;
            response
                .recv_timeout(Duration::from_secs(5))
                .map_err(|_| "Backend timeout".into())
        });
        (HttpApi::start(0, forward.clone()).unwrap(), forward)
    }
    #[test]
    fn http_plan_commit_changes_the_same_backend_and_reports_conflicts() {
        let (api, plane) = fixture();
        let (status, mut session) = call(
            &api,
            "/api/v1/sessions/get",
            Some(json!({"sessionId": "desktop-session"})),
        );
        assert_eq!(status, 200);
        session["name"] = json!("Changed over HTTP");
        let (status, plan) = call(
            &api,
            "/api/v1/graph/plan",
            Some(json!({"sessionId":"desktop-session", "baseRevision":0, "candidate":session})),
        );
        assert_eq!(status, 200, "{plan}");
        let (status, result) = call(
            &api,
            "/api/v1/graph/commit",
            Some(json!({"planId":plan["planId"],"baseRevision":0,"idempotencyKey":"http-edit"})),
        );
        assert_eq!(status, 200, "{result}");
        let read = plane(
            &rpc("sessions.get", Some(json!({"sessionId":"desktop-session"}))),
            "TEST",
        )
        .unwrap()
        .result
        .unwrap();
        assert_eq!(read["name"], "Changed over HTTP");
        assert_eq!(read["revision"], 1);
        let (status, error) = call(
            &api,
            "/api/v1/graph/plan",
            Some(json!({"sessionId":"desktop-session","baseRevision":0,"candidate":read})),
        );
        assert_eq!(status, 409, "{error}");
    }
    #[test]
    fn streamdeck_style_requests_work_by_name_in_one_call() {
        let (api, plane) = fixture();
        // A key press: toggle a node by its name, no IDs or revisions.
        let (status, toggled) = call(
            &api,
            "/api/v1/nodes/toggle",
            Some(json!({"node":"neutral GAIN","target":"bypass","idempotencyKey":"deck-1"})),
        );
        assert_eq!(status, 200, "{toggled}");
        assert_eq!(toggled["value"], true);
        let session = plane(
            &rpc("sessions.get", Some(json!({"sessionId":"desktop-session"}))),
            "TEST",
        )
        .unwrap()
        .result
        .unwrap();
        assert_eq!(
            session["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .find(|node| node["name"] == "Neutral gain")
                .unwrap()["bypass"],
            true
        );
        // Displays read a plain summary and compact levels.
        let (status, summary) = call(&api, "/api/v1/sessions/summary", Some(json!({})));
        assert_eq!(status, 200, "{summary}");
        assert_eq!(summary["sessionId"], "desktop-session");
        let (status, levels) = call(&api, "/api/v1/meters/levels", Some(json!({})));
        assert_eq!(status, 200, "{levels}");
        assert_eq!(levels["playing"], false);
        // The recording folder is read-only here; only the window approves one.
        let (status, root) = call(&api, "/api/v1/recordings/getRoot", Some(json!({})));
        assert_eq!(status, 200, "{root}");
        let (status, refused) = call(
            &api,
            "/api/v1/recordings/setRoot",
            Some(json!({"root":"C:\\Temp","create":true,"idempotencyKey":"deck-root"})),
        );
        assert_eq!(status, 403, "{refused}");
        // A token holder cannot allow itself to open audio devices.
        let (status, refused) = call(
            &api,
            "/api/v1/devices/setAccess",
            Some(json!({"allowed":true,"idempotencyKey":"deck-devices"})),
        );
        assert_eq!(status, 403, "{refused}");
        let (status, access) = call(&api, "/api/v1/devices/getAccess", Some(json!({})));
        assert_eq!(status, 200, "{access}");
        // Mistakes come back as 400 with a readable reason.
        let (status, error) = call(
            &api,
            "/api/v1/nodes/toggle",
            Some(json!({"node":"Guitar","target":"bypass","idempotencyKey":"deck-2"})),
        );
        assert_eq!(status, 400, "{error}");
        assert!(error.to_string().contains("no node named"), "{error}");
    }
    #[test]
    fn http_rejects_token_origin_host_and_oversized_or_malformed_input() {
        let (api, _) = fixture();
        for headers in [
            format!("Host: 127.0.0.1:{}", api.port),
            format!(
                "Host: 127.0.0.1:{}\r\nAuthorization: Bearer wrong",
                api.port
            ),
        ] {
            assert!(exchange(
                api.port,
                &format!("GET /api/v1/status HTTP/1.1\r\n{headers}\r\n\r\n")
            )
            .starts_with("HTTP/1.1 401"));
        }
        for headers in [
            format!("Host: attacker.test:{}", api.port),
            format!(
                "Host: 127.0.0.1:{}\r\nOrigin: https://attacker.test",
                api.port
            ),
        ] {
            assert!(exchange(
                api.port,
                &format!(
                    "GET /api/v1/status HTTP/1.1\r\n{headers}\r\nAuthorization: Bearer {}\r\n\r\n",
                    api.token
                )
            )
            .starts_with("HTTP/1.1 403"));
        }
        let prefix = format!("POST /api/v1/graph/plan HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\n", api.port, api.token);
        assert!(exchange(
            api.port,
            &format!("{prefix}Content-Length: {}\r\n\r\n", MAX_BODY + 1)
        )
        .starts_with("HTTP/1.1 413"));
        assert!(
            exchange(api.port, &format!("{prefix}Content-Length: 1\r\n\r\nx"))
                .starts_with("HTTP/1.1 400")
        );
        assert!(exchange(
            api.port,
            &format!("{prefix}Transfer-Encoding: chunked\r\n\r\n")
        )
        .starts_with("HTTP/1.1 400"));
        assert!(exchange(
            api.port,
            &format!("{prefix}Content-Length: 0\r\nContent-Length: 0\r\n\r\n")
        )
        .starts_with("HTTP/1.1 400"));
        assert!(HttpApi::start(
            api.port,
            Arc::new(|_: &JsonRpcRequest, _: &str| unreachable!())
        )
        .err()
        .unwrap()
        .contains("Choose another port"));
    }

    #[test]
    fn network_listener_accepts_only_private_addresses_hosts_and_peers() {
        for (address, allowed) in [
            ("192.168.1.20", true),
            ("10.0.0.5", true),
            ("172.16.0.1", true),
            ("169.254.10.2", true),
            ("8.8.8.8", false),
            ("100.64.0.1", false),
            ("0.0.0.0", false),
            ("127.0.0.1", false),
        ] {
            assert_eq!(
                lan_address_allowed(address.parse().unwrap()),
                allowed,
                "{address}"
            );
        }
        assert!(HttpApi::start_on(
            0,
            "a".repeat(64),
            Some("8.8.8.8".parse().unwrap()),
            Arc::new(|_: &JsonRpcRequest, _: &str| unreachable!())
        )
        .err()
        .unwrap()
        .contains("not a private"));
        assert!(peer_allowed("127.0.0.1".parse().unwrap(), false));
        assert!(!peer_allowed("192.168.1.30".parse().unwrap(), false));
        assert!(peer_allowed("192.168.1.30".parse().unwrap(), true));
        assert!(!peer_allowed("203.0.113.9".parse().unwrap(), true));
        assert!(peer_allowed("::ffff:192.168.1.30".parse().unwrap(), true));
        assert!(!peer_allowed("fe80::1".parse().unwrap(), true));
        let hosts = [
            "127.0.0.1:17891".to_string(),
            "192.168.1.20:17891".to_string(),
        ];
        assert!(host_allowed(Some("192.168.1.20:17891"), None, &hosts));
        assert!(host_allowed(
            Some("192.168.1.20:17891"),
            Some("http://192.168.1.20:17891"),
            &hosts
        ));
        assert!(!host_allowed(
            Some("192.168.1.20:17891"),
            Some("http://127.0.0.1:17891"),
            &hosts
        ));
        assert!(!host_allowed(Some("192.168.1.21:17891"), None, &hosts));
        assert!(!host_allowed(Some("audiorouter.local:17891"), None, &hosts));
        assert!(!host_allowed(None, None, &hosts));
    }

    /// Uses this PC's own private address when it has one; a PC with no
    /// connected private network has nothing to bind and skips.
    #[test]
    fn network_listener_serves_its_address_beside_loopback() {
        let Some(lan) = crate::lan_addresses::list()
            .unwrap()
            .first()
            .map(|entry| entry.address)
        else {
            eprintln!("no private IPv4 address on this PC; skipped");
            return;
        };
        let schema = ControlPlane::new("http-lan").describe();
        let api = HttpApi::start_on(
            0,
            "c".repeat(64),
            Some(lan),
            Arc::new(move |request: &JsonRpcRequest, _request_id: &str| {
                if request.method == "system.describe" {
                    return Ok(JsonRpcResponse::success(request.id.clone(), schema.clone()));
                }
                Ok(ControlPlane::new("http-lan").dispatch(request.clone()))
            }),
        )
        .unwrap();
        assert_eq!(api.lan, Some(lan));
        assert_eq!(call(&api, "/api/v1/status", None).0, 200);
        let port = api.port;
        let over_network = |host: String| {
            let mut stream = TcpStream::connect((lan, port)).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            write!(
                stream,
                "GET /api/v1/status HTTP/1.1\r\nHost: {host}\r\nAuthorization: Bearer {}\r\n\r\n",
                api.token
            )
            .unwrap();
            let mut result = String::new();
            stream.read_to_string(&mut result).unwrap();
            result
        };
        assert!(over_network(format!("{lan}:{port}")).starts_with("HTTP/1.1 200"));
        assert!(over_network(format!("attacker.test:{port}")).starts_with("HTTP/1.1 403"));
        let (_, schema) = call(&api, "/openapi.json", None);
        assert_eq!(schema["servers"][1]["url"], format!("http://{lan}:{port}"));
    }

    #[test]
    fn headers_and_body_can_arrive_in_separate_packets() {
        let (api, _) = fixture();
        let mut stream = TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, api.port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        write!(stream, "POST /api/v1/status/get HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n", api.port, api.token).unwrap();
        std::thread::sleep(Duration::from_millis(50));
        stream.write_all(b"{").unwrap();
        std::thread::sleep(Duration::from_millis(50));
        stream.write_all(b"}").unwrap();
        let mut result = String::new();
        stream.read_to_string(&mut result).unwrap();
        assert!(result.starts_with("HTTP/1.1 200"), "{result}");
    }
    /// A backend that answers discovery and blocks every other call until
    /// `release` is set, reporting each blocked call on `entered`.
    fn blocking_backend(
        entered: std::sync::mpsc::Sender<()>,
        release: Arc<(Mutex<bool>, std::sync::Condvar)>,
    ) -> Arc<Forward> {
        let schema = ControlPlane::new("http-busy").describe();
        Arc::new(move |request: &JsonRpcRequest, _request_id: &str| {
            if request.method == "system.describe" {
                return Ok(JsonRpcResponse::success(request.id.clone(), schema.clone()));
            }
            let _ = entered.send(());
            let (open, wake) = &*release;
            let mut open = open.lock().unwrap();
            while !*open {
                open = wake.wait(open).unwrap();
            }
            Ok(ControlPlane::new("http-busy").dispatch(request.clone()))
        })
    }
    fn connect(port: u16) -> TcpStream {
        let stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        stream
    }

    #[test]
    fn full_queue_answers_503_with_retry_after_instead_of_dropping() {
        let (entered, entered_calls) = std::sync::mpsc::channel();
        let release = Arc::new((Mutex::new(false), std::sync::Condvar::new()));
        let api = HttpApi::start(0, blocking_backend(entered, release.clone())).unwrap();
        // Occupy every worker inside the backend call.
        let mut busy = Vec::new();
        for _ in 0..WORKERS {
            let mut stream = connect(api.port);
            write!(
                stream,
                "GET /api/v1/status HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAuthorization: Bearer {}\r\n\r\n",
                api.port, api.token
            )
            .unwrap();
            busy.push(stream);
        }
        for _ in 0..WORKERS {
            entered_calls.recv_timeout(Duration::from_secs(5)).unwrap();
        }
        // Fill the queue; nobody can take these sockets yet.
        let queued = (0..QUEUE).map(|_| connect(api.port)).collect::<Vec<_>>();
        // The next connection is answered at once rather than closed silently.
        let started = Instant::now();
        let mut overflow = connect(api.port);
        let mut response = String::new();
        overflow.read_to_string(&mut response).unwrap();
        assert!(started.elapsed() < Duration::from_secs(1));
        let (headers, body) = response.split_once("\r\n\r\n").unwrap();
        assert!(
            headers.starts_with("HTTP/1.1 503 Service Unavailable\r\n"),
            "{headers}"
        );
        assert!(headers.contains("\r\nRetry-After: 1\r\n"), "{headers}");
        assert!(headers.contains("\r\nConnection: close\r\n"), "{headers}");
        assert!(
            headers.contains("\r\nContent-Type: application/json\r\n"),
            "{headers}"
        );
        let body: Value = serde_json::from_str(body).unwrap();
        assert!(body["error"]["message"].as_str().unwrap().contains("busy"));
        // Releasing the backend completes the held requests normally.
        *release.0.lock().unwrap() = true;
        release.1.notify_all();
        for mut stream in busy {
            let mut response = String::new();
            stream.read_to_string(&mut response).unwrap();
            assert!(response.starts_with("HTTP/1.1 200"), "{response}");
        }
        drop(queued);
        let started = Instant::now();
        drop(api);
        assert!(started.elapsed() < Duration::from_secs(3));
    }

    #[test]
    fn openapi_covers_every_method_and_docs_are_offline() {
        let (api, plane) = fixture();
        let (_, schema) = call(&api, "/openapi.json", None);
        for spec in audiorouter_domain::API_METHODS {
            if DESKTOP_ONLY_METHODS.contains(&spec.name) {
                assert!(schema["paths"][operation_path(spec.name)].is_null());
                continue;
            }
            assert_eq!(
                schema["paths"][operation_path(spec.name)]["post"]["operationId"],
                spec.name
            );
            assert_eq!(
                schema["paths"][operation_path(spec.name)]["post"]["requestBody"]["content"]
                    ["application/json"]["schema"],
                plane(&rpc("system.describe", None), "TEST")
                    .unwrap()
                    .result
                    .unwrap()["methods"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|method| method["name"] == spec.name)
                    .unwrap()["inputSchema"]
            );
        }
        let docs = exchange(
            api.port,
            &format!("GET /docs HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n\r\n", api.port),
        );
        assert!(docs.contains("swagger-ui-bundle.js"));
        assert!(!docs.contains("https://"));
        let init = exchange(
            api.port,
            &format!(
                "GET /swagger-init.js HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n\r\n",
                api.port
            ),
        );
        assert!(init.contains("validatorUrl:null"));
        assert!(init.contains("persistAuthorization:false"));
    }
    #[test]
    fn restart_preserves_token_rotation_revokes_it_and_permissions_are_preserved() {
        let schema = ControlPlane::new("http-observer").describe();
        let api = HttpApi::start(
            0,
            Arc::new(move |request: &JsonRpcRequest, _request_id: &str| {
                if request.method == "system.describe" {
                    return Ok(JsonRpcResponse::success(request.id.clone(), schema.clone()));
                }
                Ok(plane_dispatch_observer(request))
            }),
        )
        .unwrap();
        let (status, error) = call(
            &api,
            "/api/v1/safety/setPrivacyMute",
            Some(json!({"muted":true,"idempotencyKey":"denied"})),
        );
        assert_eq!(status, 403, "{error}");
        let port = api.port;
        let token = api.token.clone();
        drop(api);
        let restarted = HttpApi::start_with_token(
            port,
            token.clone(),
            Arc::new(move |request: &JsonRpcRequest, _request_id: &str| {
                Ok(ControlPlane::new("restart").dispatch(request.clone()))
            }),
        )
        .unwrap();
        assert_eq!(token, restarted.token);
        assert!(exchange(port, &format!("GET /api/v1/status HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer {token}\r\n\r\n")).starts_with("HTTP/1.1 200"));
        drop(restarted);
        let rotated = HttpApi::start_with_token(
            port,
            crate::api_token::generate().unwrap(),
            Arc::new(move |request: &JsonRpcRequest, _request_id: &str| {
                Ok(ControlPlane::new("rotation").dispatch(request.clone()))
            }),
        )
        .unwrap();
        assert_ne!(token, rotated.token);
        assert!(exchange(port, &format!("GET /api/v1/status HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer {token}\r\n\r\n")).starts_with("HTTP/1.1 401"));
        assert_eq!(call(&rotated, "/api/v1/status", None).0, 200);
    }
    #[test]
    fn http_request_ids_are_echoed_validated_and_forwarded() {
        let schema = ControlPlane::new("http-request-id").describe();
        let seen = Arc::new(Mutex::new(Vec::<String>::new()));
        let observed = seen.clone();
        let api = HttpApi::start(
            0,
            Arc::new(move |request: &JsonRpcRequest, request_id: &str| {
                observed.lock().unwrap().push(request_id.to_owned());
                if request.method == "system.describe" {
                    return Ok(JsonRpcResponse::success(request.id.clone(), schema.clone()));
                }
                Ok(ControlPlane::new("http-request-id").dispatch(request.clone()))
            }),
        )
        .unwrap();
        let get = |extra: &str| {
            exchange(
                api.port,
                &format!(
                    "GET /api/v1/status HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAuthorization: Bearer {}\r\n{extra}\r\n",
                    api.port, api.token
                ),
            )
        };
        let header = |response: &str| {
            response
                .lines()
                .find_map(|line| line.strip_prefix("X-Request-Id: "))
                .map(str::to_owned)
        };
        let echoed = get("X-Request-Id: Deck-42\r\n");
        assert!(echoed.starts_with("HTTP/1.1 200"), "{echoed}");
        assert_eq!(header(&echoed).as_deref(), Some("Deck-42"));
        assert_eq!(seen.lock().unwrap().last().unwrap(), "Deck-42");
        let long = "x".repeat(33);
        for hostile in ["has space", long.as_str(), "under_score"] {
            let replaced = get(&format!("X-Request-Id: {hostile}\r\n"));
            let id = header(&replaced).expect("a generated request ID");
            assert_ne!(id, hostile);
            assert!(audiorouter_protocol::diagnostics::is_valid_correlation_id(
                &id
            ));
            assert!(!replaced.contains(hostile), "{replaced}");
            assert_eq!(seen.lock().unwrap().last().unwrap(), &id);
        }
        let generated = header(&get("")).expect("a generated request ID");
        assert_eq!(generated.len(), 8);
        // Requests refused before reaching the backend carry no ID.
        let refused = exchange(
            api.port,
            &format!(
                "GET /api/v1/status HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n\r\n",
                api.port
            ),
        );
        assert!(refused.starts_with("HTTP/1.1 401"));
        assert_eq!(header(&refused), None);
    }

    fn plane_dispatch_observer(request: &JsonRpcRequest) -> JsonRpcResponse {
        let mut plane = ControlPlane::new("observer");
        plane.dispatch_authorized(
            request.clone(),
            &audiorouter_control::ClientGrant::for_role(audiorouter_control::ClientRole::Observer),
        )
    }

    #[test]
    #[cfg(windows)]
    #[ignore = "attended HTTP/browser fixture; disposable pipe and memory state, no audio"]
    fn http_browser_acceptance_fixture() {
        let pipe = format!(r"\\.\pipe\audiorouter-http-browser-{}", std::process::id());
        let server_pipe = pipe.clone();
        std::thread::spawn(move || {
            let mut plane = ControlPlane::new("http-browser");
            let mut session = crate::default_desktop_session();
            session.id = audiorouter_domain::EntityId::new("e2e-session");
            session.name = "HTTP qualification".into();
            plane.insert_session(session).unwrap();
            audiorouter_transport::serve_control_connections_forever_with_grant(
                &server_pipe,
                plane,
                audiorouter_control::ClientGrant::for_role(
                    audiorouter_control::ClientRole::Operator,
                ),
            )
            .unwrap();
        });
        let api = HttpApi::start(17893, crate::http_forward(pipe)).unwrap();
        // Explicit test-only handshake consumed in memory by the browser runner.
        // This disposable grant has no user database or audio; do not persist it.
        println!("HTTP_FIXTURE {} {}", api.port, api.token);
        for _ in 0..1800 {
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}
