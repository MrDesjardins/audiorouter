//! AudioRouter cable driver helper (spec 17 §6, WP-07).
//!
//! The binary runs elevated and is the only AudioRouter component that
//! changes the driver store or creates the `ROOT\AudioRouterVirtual` device
//! (VDEV-08). This library holds everything that can be tested without
//! Windows driver installation: argument validation, package inspection,
//! version rules, the atomic state file, the bounded log, result JSON, exit
//! codes and the command flows (idempotency, rollback, waits). The Windows
//! side is behind [`Platform`]; `windows_platform` implements it with
//! SetupAPI, WinTrust and MMDevice.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[cfg(windows)]
pub mod windows_platform;

/// Hardware ID of the one root-enumerated AudioRouter cable device.
pub const HARDWARE_ID: &str = "ROOT\\AudioRouterVirtual";
pub const INF_NAME: &str = "audioroutervirtual.inf";
pub const SYS_NAME: &str = "audioroutervirtual.sys";
pub const CAT_NAME: &str = "audioroutervirtual.cat";
pub const PACKAGE_JSON: &str = "package.json";
pub const MAX_CABLES: u8 = 8;
pub const DEFAULT_CABLE_COUNT: u8 = 2;
pub const STATE_SCHEMA: u32 = 1;
pub const MAX_STATE_BYTES: u64 = 64 * 1024;
pub const MAX_INF_BYTES: u64 = 512 * 1024;
pub const MAX_LOG_BYTES: u64 = 1024 * 1024;
pub const MAX_NAME_CHARS: usize = 48;
/// Bounded wait for endpoints after install/update/restart (17 §6).
pub const ENDPOINT_WAIT_MS: u64 = 30_000;
pub const ENDPOINT_POLL_MS: u64 = 250;
/// Exact signer of Microsoft-signed (attestation/WHQL) driver catalogs.
pub const MICROSOFT_DRIVER_SIGNER: &str = "Microsoft Windows Hardware Compatibility Publisher";

/// Exit codes (17 §6).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ExitKind {
    Success,
    InvalidRequest,
    VerificationFailed,
    WindowsRefused,
    Timeout,
    RestartRequired,
}

impl ExitKind {
    pub fn code(self) -> i32 {
        match self {
            Self::Success => 0,
            Self::InvalidRequest => 1,
            Self::VerificationFailed => 2,
            Self::WindowsRefused => 3,
            Self::Timeout => 4,
            Self::RestartRequired => 5,
        }
    }
}

/// A failed step with its exit class and, for Windows refusals, the HRESULT.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Failure {
    pub kind: ExitKind,
    pub message: String,
    pub hresult: Option<i32>,
}

impl Failure {
    pub fn invalid(message: impl Into<String>) -> Self {
        Self {
            kind: ExitKind::InvalidRequest,
            message: message.into(),
            hresult: None,
        }
    }
    pub fn verification(message: impl Into<String>) -> Self {
        Self {
            kind: ExitKind::VerificationFailed,
            message: message.into(),
            hresult: None,
        }
    }
    pub fn windows(message: impl Into<String>, hresult: i32) -> Self {
        Self {
            kind: ExitKind::WindowsRefused,
            message: message.into(),
            hresult: Some(hresult),
        }
    }
    pub fn timeout(message: impl Into<String>) -> Self {
        Self {
            kind: ExitKind::Timeout,
            message: message.into(),
            hresult: None,
        }
    }
}

// ---------------------------------------------------------------- requests

/// Registry parameters the helper may write (17 §5.5). CableCount is set
/// only through `set-cables`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Parameter {
    MinPeriodFrames,
    DefaultPeriodFrames,
    MaxLeaseMs,
}

impl Parameter {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "MinPeriodFrames" => Some(Self::MinPeriodFrames),
            "DefaultPeriodFrames" => Some(Self::DefaultPeriodFrames),
            "MaxLeaseMs" => Some(Self::MaxLeaseMs),
            _ => None,
        }
    }
    pub fn registry_name(self) -> &'static str {
        match self {
            Self::MinPeriodFrames => "MinPeriodFrames",
            Self::DefaultPeriodFrames => "DefaultPeriodFrames",
            Self::MaxLeaseMs => "MaxLeaseMs",
        }
    }
    /// Same inclusive ranges the driver enforces (bridgeio.h AR_CONFIG_*).
    pub fn range(self) -> (u32, u32) {
        match self {
            Self::MinPeriodFrames => (64, 480),
            Self::DefaultPeriodFrames => (128, 960),
            Self::MaxLeaseMs => (500, 60_000),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Request {
    Status,
    Install {
        package: PathBuf,
    },
    Update {
        package: PathBuf,
    },
    Repair {
        package: PathBuf,
    },
    SetCables {
        count: u8,
    },
    Configure {
        names: BTreeMap<u8, String>,
        params: BTreeMap<Parameter, u32>,
    },
    Remove,
}

impl Request {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Status => "status",
            Self::Install { .. } => "install",
            Self::Update { .. } => "update",
            Self::Repair { .. } => "repair",
            Self::SetCables { .. } => "set-cables",
            Self::Configure { .. } => "configure",
            Self::Remove => "remove",
        }
    }
}

/// Parsed command line: the request plus where to also write the result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Invocation {
    pub request: Request,
    pub result_path: Option<PathBuf>,
}

pub const USAGE: &str = "usage: audiorouter-driver-helper <status | install --package <dir> | \
update --package <dir> | repair --package <dir> | set-cables --count <1-8> | \
configure [--name <1-8>=<text>]... [--param <Name>=<value>]... | remove> [--result <file.json>]";

pub fn parse_arguments(arguments: &[String]) -> Result<Invocation, Failure> {
    let mut iter = arguments.iter();
    let command = iter.next().ok_or_else(|| Failure::invalid(USAGE))?;
    let mut package: Option<PathBuf> = None;
    let mut count: Option<u8> = None;
    let mut names = BTreeMap::new();
    let mut params = BTreeMap::new();
    let mut result_path = None;
    while let Some(flag) = iter.next() {
        let value = iter
            .next()
            .ok_or_else(|| Failure::invalid(format!("{flag} needs a value")))?;
        match flag.as_str() {
            "--package" if package.is_none() => package = Some(PathBuf::from(value)),
            "--count" if count.is_none() => count = Some(parse_cable_count(value)?),
            "--name" => {
                let (index, text) = parse_name_assignment(value)?;
                if names.insert(index, text).is_some() {
                    return Err(Failure::invalid(format!("cable {} named twice", index + 1)));
                }
            }
            "--param" => {
                let (parameter, number) = parse_parameter_assignment(value)?;
                if params.insert(parameter, number).is_some() {
                    return Err(Failure::invalid(format!(
                        "{} given twice",
                        parameter.registry_name()
                    )));
                }
            }
            "--result" if result_path.is_none() => result_path = Some(PathBuf::from(value)),
            other => {
                return Err(Failure::invalid(format!(
                    "unexpected argument {other}\n{USAGE}"
                )))
            }
        }
    }
    let needs_package = matches!(command.as_str(), "install" | "update" | "repair");
    if needs_package != package.is_some() {
        return Err(Failure::invalid(format!(
            "{command}: --package is {}\n{USAGE}",
            if needs_package {
                "required"
            } else {
                "not accepted"
            }
        )));
    }
    if (command == "set-cables") != count.is_some() {
        return Err(Failure::invalid(format!(
            "--count belongs to set-cables only\n{USAGE}"
        )));
    }
    if command != "configure" && (!names.is_empty() || !params.is_empty()) {
        return Err(Failure::invalid(format!(
            "--name/--param belong to configure only\n{USAGE}"
        )));
    }
    let request = match command.as_str() {
        "status" => Request::Status,
        "install" => Request::Install {
            package: package.unwrap(),
        },
        "update" => Request::Update {
            package: package.unwrap(),
        },
        "repair" => Request::Repair {
            package: package.unwrap(),
        },
        "set-cables" => Request::SetCables {
            count: count.unwrap(),
        },
        "configure" => {
            if names.is_empty() && params.is_empty() {
                return Err(Failure::invalid(
                    "configure needs at least one --name or --param",
                ));
            }
            validate_period_pair(&params)?;
            Request::Configure { names, params }
        }
        "remove" => Request::Remove,
        other => {
            return Err(Failure::invalid(format!(
                "unknown command {other}\n{USAGE}"
            )))
        }
    };
    Ok(Invocation {
        request,
        result_path,
    })
}

fn parse_cable_count(text: &str) -> Result<u8, Failure> {
    match text.parse::<u8>() {
        Ok(count) if (1..=MAX_CABLES).contains(&count) => Ok(count),
        _ => Err(Failure::invalid(format!(
            "--count must be 1-{MAX_CABLES}, not {text}"
        ))),
    }
}

/// `--name 2=Discord` → (1, "Discord"). Cable numbers are 1-based for users.
fn parse_name_assignment(text: &str) -> Result<(u8, String), Failure> {
    let (index, name) = text
        .split_once('=')
        .ok_or_else(|| Failure::invalid(format!("--name expects <1-8>=<text>, not {text}")))?;
    let index = parse_cable_count(index)
        .map_err(|_| Failure::invalid(format!("--name cable must be 1-{MAX_CABLES}")))?;
    validate_cable_name(name)?;
    Ok((index - 1, name.to_owned()))
}

/// Display names: 1-48 printable characters, no control characters, and no
/// parentheses, because Windows composes "<name> (<cable interface>)" and
/// the helper recognises its endpoints by that suffix.
pub fn validate_cable_name(name: &str) -> Result<(), Failure> {
    let count = name.chars().count();
    if count == 0 || count > MAX_NAME_CHARS {
        return Err(Failure::invalid(format!(
            "cable names are 1-{MAX_NAME_CHARS} characters"
        )));
    }
    if name.trim() != name {
        return Err(Failure::invalid(
            "cable names cannot start or end with spaces",
        ));
    }
    if name.chars().any(|c| c.is_control() || c == '(' || c == ')') {
        return Err(Failure::invalid(
            "cable names cannot contain control characters or parentheses",
        ));
    }
    Ok(())
}

fn parse_parameter_assignment(text: &str) -> Result<(Parameter, u32), Failure> {
    let (name, value) = text
        .split_once('=')
        .ok_or_else(|| Failure::invalid(format!("--param expects <Name>=<value>, not {text}")))?;
    let parameter = Parameter::parse(name).ok_or_else(|| {
        Failure::invalid(format!(
            "unknown parameter {name}; use MinPeriodFrames, DefaultPeriodFrames or MaxLeaseMs"
        ))
    })?;
    let (low, high) = parameter.range();
    match value.parse::<u32>() {
        Ok(number) if (low..=high).contains(&number) => Ok((parameter, number)),
        _ => Err(Failure::invalid(format!(
            "{name} must be {low}-{high}, not {value}"
        ))),
    }
}

fn validate_period_pair(params: &BTreeMap<Parameter, u32>) -> Result<(), Failure> {
    if let (Some(min), Some(default)) = (
        params.get(&Parameter::MinPeriodFrames),
        params.get(&Parameter::DefaultPeriodFrames),
    ) {
        if default < min {
            return Err(Failure::invalid(
                "DefaultPeriodFrames cannot be below MinPeriodFrames",
            ));
        }
    }
    Ok(())
}

// ----------------------------------------------------------------- package

/// A four-part driver version from the INF `DriverVer` line.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct DriverVersion(pub [u16; 4]);

impl DriverVersion {
    pub fn parse(text: &str) -> Option<Self> {
        let mut parts = [0_u16; 4];
        let mut count = 0;
        for part in text.trim().split('.') {
            if count == 4 || part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            parts[count] = part.parse().ok()?;
            count += 1;
        }
        (count == 4).then_some(Self(parts))
    }
}

impl std::fmt::Display for DriverVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let [a, b, c, d] = self.0;
        write!(f, "{a}.{b}.{c}.{d}")
    }
}

/// Facts read from a driver package directory before any Windows call.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageInfo {
    pub dir: PathBuf,
    pub inf: PathBuf,
    pub sys: PathBuf,
    pub cat: PathBuf,
    pub version: DriverVersion,
    /// `package.json` "signed": "unsigned" | "test" | "microsoft".
    pub declared_signing: String,
}

/// Which certificate signed the catalog, as found by WinVerifyTrust.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Signer {
    Microsoft,
    /// Any other trusted signer (the WDK/AudioRouter test certificate in the VM).
    Other(String),
}

/// Accept the verified signer under the rules of 17 §9.1.
pub fn signer_allowed(signer: &Signer, allow_test: bool) -> Result<(), Failure> {
    match signer {
        Signer::Microsoft => Ok(()),
        Signer::Other(name) if allow_test => {
            let _ = name;
            Ok(())
        }
        Signer::Other(name) => Err(Failure::verification(format!(
            "the driver catalog is signed by \"{name}\", not {MICROSOFT_DRIVER_SIGNER}; \
             test-signed packages are accepted only by a debug helper with AUDIOROUTER_ALLOW_TEST_DRIVER=1"
        ))),
    }
}

/// The package must be the `driver` folder beside the helper (17 §6). Only
/// a test-enabled debug helper may use another absolute directory (VM).
pub fn resolve_package_dir(
    requested: &Path,
    helper_dir: &Path,
    allow_test: bool,
) -> Result<PathBuf, Failure> {
    if !requested.is_absolute() {
        return Err(Failure::invalid("--package must be an absolute path"));
    }
    let requested = std::fs::canonicalize(requested)
        .map_err(|error| Failure::invalid(format!("package folder not found: {error}")))?;
    if allow_test {
        return Ok(requested);
    }
    let expected = std::fs::canonicalize(helper_dir.join("driver")).map_err(|_| {
        Failure::invalid("the AudioRouter install has no driver folder beside the helper")
    })?;
    if requested != expected {
        return Err(Failure::invalid(
            "--package must be the driver folder of this AudioRouter installation",
        ));
    }
    Ok(requested)
}

fn read_bounded(path: &Path, limit: u64) -> Result<String, Failure> {
    let metadata = std::fs::symlink_metadata(path).map_err(|_| {
        Failure::verification(format!(
            "package is incomplete: {} is missing",
            file_label(path)
        ))
    })?;
    if !metadata.is_file() || is_reparse(&metadata) {
        return Err(Failure::verification(format!(
            "{} must be a regular file",
            file_label(path)
        )));
    }
    if metadata.len() > limit {
        return Err(Failure::verification(format!(
            "{} is larger than {limit} bytes",
            file_label(path)
        )));
    }
    let bytes = std::fs::read(path)
        .map_err(|error| Failure::verification(format!("{}: {error}", file_label(path))))?;
    decode_text(&bytes).ok_or_else(|| {
        Failure::verification(format!("{} is not UTF-8/UTF-16 text", file_label(path)))
    })
}

fn file_label(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// INF files produced by StampInf may be UTF-16LE with a BOM.
pub fn decode_text(bytes: &[u8]) -> Option<String> {
    if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        if rest.len() % 2 != 0 {
            return None;
        }
        let units: Vec<u16> = rest
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        return String::from_utf16(&units).ok();
    }
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    String::from_utf8(bytes.to_vec()).ok()
}

#[cfg(windows)]
fn is_reparse(metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    metadata.file_attributes() & 0x400 != 0 || metadata.file_type().is_symlink()
}

#[cfg(not(windows))]
fn is_reparse(metadata: &std::fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

/// Read and check the package's identity before any signature or Windows
/// step: the INF must be AudioRouter's (provider, hardware ID, catalog),
/// carry a DriverVer, and agree with package.json.
pub fn inspect_package(dir: &Path) -> Result<PackageInfo, Failure> {
    let inf = dir.join(INF_NAME);
    let sys = dir.join(SYS_NAME);
    let cat = dir.join(CAT_NAME);
    let text = read_bounded(&inf, MAX_INF_BYTES)?;
    for path in [&sys, &cat] {
        let metadata = std::fs::symlink_metadata(path).map_err(|_| {
            Failure::verification(format!(
                "package is incomplete: {} is missing",
                file_label(path)
            ))
        })?;
        if !metadata.is_file() || is_reparse(&metadata) || metadata.len() == 0 {
            return Err(Failure::verification(format!(
                "{} must be a non-empty regular file",
                file_label(path)
            )));
        }
    }
    let version = inf_identity(&text)?;
    let manifest = read_bounded(&dir.join(PACKAGE_JSON), 16 * 1024)?;
    let manifest: serde_json::Value = serde_json::from_str(&manifest)
        .map_err(|error| Failure::verification(format!("package.json is invalid: {error}")))?;
    let declared = manifest
        .get("driverVersion")
        .and_then(|v| v.as_str())
        .and_then(DriverVersion::parse);
    if declared != Some(version) {
        return Err(Failure::verification(format!(
            "package.json driverVersion does not match the INF DriverVer {version}"
        )));
    }
    let declared_signing = manifest
        .get("signed")
        .and_then(|v| v.as_str())
        .filter(|v| matches!(*v, "unsigned" | "test" | "microsoft"))
        .ok_or_else(|| {
            Failure::verification("package.json \"signed\" must be unsigned, test or microsoft")
        })?
        .to_owned();
    if declared_signing == "unsigned" {
        return Err(Failure::verification(
            "the package is unsigned; it cannot be installed",
        ));
    }
    Ok(PackageInfo {
        dir: dir.to_path_buf(),
        inf,
        sys,
        cat,
        version,
        declared_signing,
    })
}

/// Check the INF is AudioRouter's and return its DriverVer version.
pub fn inf_identity(text: &str) -> Result<DriverVersion, Failure> {
    let mut provider_ok = false;
    let mut provider_string_ok = false;
    let mut catalog_ok = false;
    let mut version = None;
    for raw in text.lines() {
        let line = raw.split(';').next().unwrap_or("").trim();
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let (key, value) = (key.trim(), value.trim());
        if key.eq_ignore_ascii_case("Provider") {
            provider_ok = value.eq_ignore_ascii_case("%ProviderName%");
        } else if key.eq_ignore_ascii_case("ProviderName") {
            provider_string_ok = value == "\"AudioRouter Project\"";
        } else if key.eq_ignore_ascii_case("CatalogFile") {
            catalog_ok = value.eq_ignore_ascii_case(CAT_NAME);
        } else if key.eq_ignore_ascii_case("DriverVer") {
            version = value.split(',').nth(1).and_then(DriverVersion::parse);
        }
    }
    let hardware_id_ok = text.lines().any(|line| {
        let line = line.split(';').next().unwrap_or("");
        line.contains('=')
            && line
                .split(',')
                .skip(1)
                .any(|id| id.trim().eq_ignore_ascii_case(HARDWARE_ID))
    });
    if !(provider_ok && provider_string_ok) {
        return Err(Failure::verification(
            "the INF is not an AudioRouter package (provider)",
        ));
    }
    if !hardware_id_ok {
        return Err(Failure::verification(format!(
            "the INF does not install {HARDWARE_ID}"
        )));
    }
    if text.to_ascii_uppercase().contains("SWD\\AUDIOROUTER") {
        return Err(Failure::verification(
            "the INF must not match a software-device instance",
        ));
    }
    if !catalog_ok {
        return Err(Failure::verification(format!(
            "the INF must name {CAT_NAME}"
        )));
    }
    version.ok_or_else(|| Failure::verification("the INF has no valid DriverVer"))
}

// ------------------------------------------------------------------- state

/// `%ProgramData%\AudioRouter\driver\state.json` (17 §6).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    pub schema: u32,
    pub package_version: DriverVersion,
    pub oem_inf: String,
    pub device_instance_id: String,
    pub installed_at: String,
    #[serde(default)]
    pub cable_names: BTreeMap<u8, String>,
}

impl State {
    pub fn validate(&self) -> Result<(), Failure> {
        let oem_ok = is_oem_inf_name(&self.oem_inf);
        if self.schema != STATE_SCHEMA
            || !oem_ok
            || self.device_instance_id.is_empty()
            || self.device_instance_id.len() > 200
        {
            return Err(Failure::invalid(
                "the helper state file is invalid; repair or remove from an elevated prompt",
            ));
        }
        for (index, name) in &self.cable_names {
            if *index >= MAX_CABLES {
                return Err(Failure::invalid(
                    "the helper state file names an unknown cable",
                ));
            }
            validate_cable_name(name)?;
        }
        Ok(())
    }
}

/// `oem<digits>.inf`, the only package names the helper ever removes.
pub fn is_oem_inf_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    name.is_ascii()
        && (8..=16).contains(&name.len())
        && lower.starts_with("oem")
        && lower.ends_with(".inf")
        && lower[3..lower.len() - 4]
            .bytes()
            .all(|b| b.is_ascii_digit())
}

pub struct StateStore {
    dir: PathBuf,
}

impl StateStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }
    pub fn path(&self) -> PathBuf {
        self.dir.join("state.json")
    }
    pub fn load(&self) -> Result<Option<State>, Failure> {
        let path = self.path();
        match std::fs::symlink_metadata(&path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(Failure::invalid(format!(
                    "cannot read helper state: {error}"
                )))
            }
            Ok(metadata)
                if !metadata.is_file()
                    || is_reparse(&metadata)
                    || metadata.len() > MAX_STATE_BYTES =>
            {
                return Err(Failure::invalid(
                    "the helper state file is not a regular bounded file",
                ));
            }
            Ok(_) => {}
        }
        let text = std::fs::read_to_string(&path)
            .map_err(|error| Failure::invalid(format!("cannot read helper state: {error}")))?;
        let state: State = serde_json::from_str(&text).map_err(|_| {
            Failure::invalid(
                "the helper state file is invalid; repair or remove from an elevated prompt",
            )
        })?;
        state.validate()?;
        Ok(Some(state))
    }
    /// Atomic replace: write a unique temporary file, then rename over.
    pub fn save(&self, state: &State) -> Result<(), Failure> {
        state.validate()?;
        std::fs::create_dir_all(&self.dir).map_err(|error| {
            Failure::invalid(format!("cannot create the helper state folder: {error}"))
        })?;
        let temporary = self.dir.join(format!(".state.{}.tmp", std::process::id()));
        let text = serde_json::to_string_pretty(state).expect("state serializes");
        let result =
            std::fs::write(&temporary, text).and_then(|_| std::fs::rename(&temporary, self.path()));
        if let Err(error) = result {
            let _ = std::fs::remove_file(&temporary);
            return Err(Failure::invalid(format!(
                "cannot write the helper state: {error}"
            )));
        }
        Ok(())
    }
    pub fn delete(&self) -> Result<(), Failure> {
        match std::fs::remove_file(self.path()) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(Failure::invalid(format!(
                "cannot delete the helper state: {error}"
            ))),
        }
    }
}

/// Append one line to `helper.log`, rotating once at 1 MB (17 §6). Lines
/// hold command names, versions, results and HRESULTs; no audio, no user
/// paths beyond the package directory.
pub fn append_log(dir: &Path, line: &str) {
    let path = dir.join("helper.log");
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    if std::fs::metadata(&path)
        .map(|m| m.len() >= MAX_LOG_BYTES)
        .unwrap_or(false)
    {
        let _ = std::fs::rename(&path, dir.join("helper.log.1"));
    }
    let bounded: String = line
        .chars()
        .filter(|c| !c.is_control())
        .take(2_000)
        .collect();
    use std::io::Write;
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let _ = writeln!(file, "{bounded}");
    }
}

// ---------------------------------------------------------------- platform

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Direction {
    /// "AudioRouter Cable X Input": apps play into it (WaveRT render).
    Input,
    /// "AudioRouter Cable X Output": apps record from it (WaveRT capture).
    Output,
}

/// One present AudioRouter endpoint.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CableEndpoint {
    pub cable: u8,
    pub direction: Direction,
    pub endpoint_id: String,
    pub friendly_name: String,
}

/// The name the INF gives each cable interface, e.g. "AudioRouter Cable B Output".
pub fn interface_name(cable: u8, direction: Direction) -> String {
    let letter = (b'A' + cable) as char;
    match direction {
        Direction::Input => format!("AudioRouter Cable {letter} Input"),
        Direction::Output => format!("AudioRouter Cable {letter} Output"),
    }
}

/// Recognise an AudioRouter endpoint from its friendly name. Windows shows
/// either the cable name, "<description> (<cable name>)", or
/// "<cable name> (AudioRouter Virtual Cable)". Generic Speakers/Line names
/// never identify a cable. Renames of the cable-description form require
/// stable metadata rather than this display-name classifier.
pub fn classify_endpoint_name(friendly: &str) -> Option<(u8, Direction)> {
    let candidate = match friendly.strip_suffix(')') {
        Some(rest) => {
            let (outer, inner) = rest.rsplit_once(" (")?;
            if outer.is_empty() || outer.contains(['(', ')']) || inner.contains(['(', ')']) {
                return None;
            }
            if inner == "AudioRouter Virtual Cable" {
                outer
            } else {
                inner
            }
        }
        None => friendly,
    };
    for cable in 0..MAX_CABLES {
        for direction in [Direction::Input, Direction::Output] {
            if candidate == interface_name(cable, direction) {
                return Some((cable, direction));
            }
        }
    }
    None
}

/// Prefer the driver-provided category description so a user-visible rename
/// cannot erase cable identity. Legacy test packages used the composed name.
pub fn classify_endpoint_properties(
    friendly: &str,
    device_description: &str,
) -> Option<(u8, Direction)> {
    classify_endpoint_name(device_description).or_else(|| classify_endpoint_name(friendly))
}

/// The display name a configured cable name produces (VCAB-02).
pub fn description_for(name: &str, direction: Direction) -> String {
    match direction {
        Direction::Input => format!("AudioRouter {name} Input"),
        Direction::Output => format!("AudioRouter {name} Output"),
    }
}

/// Windows-side operations. Each returns a [`Failure`] carrying the HRESULT
/// when Windows refuses; implementations never touch other drivers.
pub trait Platform {
    /// WinVerifyTrust on the catalog with INF/SYS members; returns the signer.
    /// `allow_test` lets a test-enabled helper fall back to generic
    /// Authenticode policy for a test certificate (17 §9.1).
    fn verify_package(
        &mut self,
        package: &PackageInfo,
        allow_test: bool,
    ) -> Result<Signer, Failure>;
    /// Add the package to the driver store; returns its `oem*.inf` name.
    fn stage_package(&mut self, inf: &Path) -> Result<String, Failure>;
    /// Remove exactly this staged package (never by provider or wildcard).
    fn unstage_package(&mut self, oem_inf: &str) -> Result<(), Failure>;
    /// Instance ID of the `ROOT\AudioRouterVirtual` device, if one exists.
    fn find_device(&mut self) -> Result<Option<String>, Failure>;
    /// Create the root device and install the package on it.
    /// Returns (instance ID, reboot required).
    fn create_device(&mut self, inf: &Path) -> Result<(String, bool), Failure>;
    /// Install `inf` on the existing device (newer DriverVer, or same when
    /// `force`). Returns reboot required.
    fn update_device(&mut self, inf: &Path, force: bool) -> Result<bool, Failure>;
    /// Remove the device instance. Returns reboot required.
    fn remove_device(&mut self, instance_id: &str) -> Result<bool, Failure>;
    fn read_device_dword(&mut self, instance_id: &str, name: &str) -> Result<Option<u32>, Failure>;
    fn write_device_dword(
        &mut self,
        instance_id: &str,
        name: &str,
        value: u32,
    ) -> Result<(), Failure>;
    /// Restart the device so it re-reads its configuration. Returns true if
    /// Windows needs a restart instead (device in use).
    fn restart_device(&mut self, instance_id: &str) -> Result<bool, Failure>;
    /// Version of the driver currently installed on the device.
    fn device_driver_version(
        &mut self,
        instance_id: &str,
    ) -> Result<Option<DriverVersion>, Failure>;
    /// Current default endpoint for each role: (role label, endpoint ID).
    fn default_endpoints(&mut self) -> Vec<(String, String)>;
    /// Present AudioRouter endpoints.
    fn cable_endpoints(&mut self) -> Vec<CableEndpoint>;
    fn set_endpoint_description(
        &mut self,
        endpoint_id: &str,
        description: &str,
    ) -> Result<(), Failure>;
    /// Bridge QUERY: (protocol major, minor, capabilities, enabled cables).
    fn query_driver(&mut self) -> Option<(u16, u16, u32, u32)>;
    fn sleep_ms(&mut self, ms: u64);
    fn now_utc(&mut self) -> String;
}

// ---------------------------------------------------------------- commands

/// Everything a command needs besides the platform.
pub struct Context {
    pub state: StateStore,
    pub log_dir: PathBuf,
    pub helper_dir: PathBuf,
    /// Debug build and AUDIOROUTER_ALLOW_TEST_DRIVER=1 (17 §9.1).
    pub allow_test_driver: bool,
}

/// The JSON result plus the exit class.
#[derive(Clone, Debug)]
pub struct Outcome {
    pub kind: ExitKind,
    pub json: serde_json::Value,
}

pub fn run(request: &Request, platform: &mut dyn Platform, context: &Context) -> Outcome {
    let result = match request {
        Request::Status => status(platform, context),
        Request::Install { package } => install(package, platform, context),
        Request::Update { package } => update(package, platform, context),
        Request::Repair { package } => repair(package, platform, context),
        Request::SetCables { count } => set_cables(*count, platform, context),
        Request::Configure { names, params } => configure(names, params, platform, context),
        Request::Remove => remove(platform, context),
    };
    let outcome = match result {
        Ok(outcome) => outcome,
        Err(failure) => Outcome {
            kind: failure.kind,
            json: serde_json::json!({
                "command": request.name(),
                "ok": false,
                "exitCode": failure.kind.code(),
                "error": failure.message,
                "hresult": failure.hresult.map(|code| format!("0x{:08X}", code as u32)),
            }),
        },
    };
    append_log(
        &context.log_dir,
        &format!(
            "{} {} exit={} {}",
            platform.now_utc(),
            request.name(),
            outcome.kind.code(),
            outcome
                .json
                .get("error")
                .and_then(|e| e.as_str())
                .unwrap_or("ok")
        ),
    );
    outcome
}

fn success(command: &str, restart: bool, mut fields: serde_json::Value) -> Outcome {
    let kind = if restart {
        ExitKind::RestartRequired
    } else {
        ExitKind::Success
    };
    if let Some(map) = fields.as_object_mut() {
        map.insert("command".into(), command.into());
        map.insert("ok".into(), true.into());
        map.insert("exitCode".into(), kind.code().into());
        map.insert("restartRequired".into(), restart.into());
    }
    Outcome { kind, json: fields }
}

fn verified_package(
    requested: &Path,
    platform: &mut dyn Platform,
    context: &Context,
) -> Result<PackageInfo, Failure> {
    let dir = resolve_package_dir(requested, &context.helper_dir, context.allow_test_driver)?;
    let package = inspect_package(&dir)?;
    if package.declared_signing == "test" && !context.allow_test_driver {
        return Err(Failure::verification(
            "this is a test-signed package; only a debug helper with AUDIOROUTER_ALLOW_TEST_DRIVER=1 accepts it",
        ));
    }
    let signer = platform.verify_package(&package, context.allow_test_driver)?;
    signer_allowed(&signer, context.allow_test_driver)?;
    Ok(package)
}

fn configured_count(platform: &mut dyn Platform, instance_id: &str) -> u8 {
    match platform.read_device_dword(instance_id, "CableCount") {
        Ok(Some(value)) if (1..=u32::from(MAX_CABLES)).contains(&value) => value as u8,
        _ => DEFAULT_CABLE_COUNT,
    }
}

/// True when exactly cables 0..count are present in both directions.
pub fn cables_match(endpoints: &[CableEndpoint], count: u8) -> bool {
    let mut seen = std::collections::BTreeSet::new();
    for endpoint in endpoints {
        if endpoint.cable >= count || !seen.insert((endpoint.cable, endpoint.direction)) {
            return false;
        }
    }
    seen.len() == usize::from(count) * 2
}

fn wait_for_cables(platform: &mut dyn Platform, count: u8) -> Result<Vec<CableEndpoint>, Failure> {
    let mut waited = 0;
    loop {
        let endpoints = platform.cable_endpoints();
        if cables_match(&endpoints, count) {
            return Ok(endpoints);
        }
        if waited >= ENDPOINT_WAIT_MS {
            return Err(Failure::timeout(format!(
                "after {} s, {} AudioRouter endpoints are present instead of the {} expected for {count} cables",
                ENDPOINT_WAIT_MS / 1000, endpoints.len(), usize::from(count) * 2
            )));
        }
        platform.sleep_ms(ENDPOINT_POLL_MS);
        waited += ENDPOINT_POLL_MS;
    }
}

fn changed_defaults(before: &[(String, String)], after: &[(String, String)]) -> Vec<String> {
    let mut changed: Vec<String> = after
        .iter()
        .filter(|entry| !before.contains(entry))
        .map(|(role, _)| role.clone())
        .collect();
    for (role, _) in before {
        if !after.iter().any(|(other, _)| other == role) && !changed.contains(role) {
            changed.push(role.clone());
        }
    }
    changed.sort();
    changed
}

/// Apply configured names as endpoint descriptions (17 §5.5 decision).
fn apply_names(
    platform: &mut dyn Platform,
    names: &BTreeMap<u8, String>,
    endpoints: &[CableEndpoint],
) -> Result<(), Failure> {
    for endpoint in endpoints {
        if let Some(name) = names.get(&endpoint.cable) {
            platform.set_endpoint_description(
                &endpoint.endpoint_id,
                &description_for(name, endpoint.direction),
            )?;
        }
    }
    Ok(())
}

fn endpoints_json(endpoints: &[CableEndpoint]) -> serde_json::Value {
    serde_json::to_value(endpoints).unwrap_or_default()
}

pub fn status(platform: &mut dyn Platform, context: &Context) -> Result<Outcome, Failure> {
    let state = context.state.load()?;
    let device = platform.find_device()?;
    let endpoints = platform.cable_endpoints();
    let query = platform.query_driver();
    let installed_version = match &device {
        Some(id) => platform.device_driver_version(id)?,
        None => None,
    };
    let count = match &device {
        Some(id) => configured_count(platform, id),
        None => DEFAULT_CABLE_COUNT,
    };
    let label = classify_status(
        state.is_some(),
        device.is_some(),
        cables_match(&endpoints, count),
        query,
    );
    Ok(success(
        "status",
        false,
        serde_json::json!({
            "state": label,
            "packageVersion": state.as_ref().map(|s| s.package_version.to_string()),
            "installedDriverVersion": installed_version.map(|v| v.to_string()),
            "deviceInstanceId": device,
            "cableCount": count,
            "protocol": query.map(|(major, minor, _, _)| format!("{major}.{minor}")),
            "capabilities": query.map(|(_, _, caps, _)| caps),
            "endpoints": endpoints_json(&endpoints),
        }),
    ))
}

/// The five states of 17 §7.1.
pub fn classify_status(
    has_state: bool,
    has_device: bool,
    endpoints_ok: bool,
    query: Option<(u16, u16, u32, u32)>,
) -> &'static str {
    const SAMPLE_FLOAT64: u32 = 0x20;
    if !has_state && !has_device {
        return "notInstalled";
    }
    if !has_device {
        return "needsRepair";
    }
    match query {
        Some((major, minor, caps, _)) if major != 1 || minor < 1 || caps & SAMPLE_FLOAT64 == 0 => {
            "incompatible"
        }
        Some(_) if endpoints_ok => "installed",
        Some(_) => "needsRepair",
        // Device present but the bridge does not answer: the driver did not
        // start (for example a pending restart or a failed load).
        None => "needsRestart",
    }
}

pub fn install(
    requested: &Path,
    platform: &mut dyn Platform,
    context: &Context,
) -> Result<Outcome, Failure> {
    let package = verified_package(requested, platform, context)?;
    let existing_state = context.state.load()?;
    if let Some(device) = platform.find_device()? {
        // 17 §6: install on an existing device behaves like update/repair;
        // a second install of the same version is a no-op success.
        let installed = platform.device_driver_version(&device)?;
        if installed == Some(package.version)
            && existing_state.as_ref().map(|s| s.package_version) == Some(package.version)
        {
            let count = configured_count(platform, &device);
            let endpoints = platform.cable_endpoints();
            if cables_match(&endpoints, count) {
                return Ok(success(
                    "install",
                    false,
                    serde_json::json!({
                        "alreadyInstalled": true,
                        "packageVersion": package.version.to_string(),
                        "deviceInstanceId": device,
                        "endpoints": endpoints_json(&endpoints),
                    }),
                ));
            }
        }
        return apply_to_existing(
            "install",
            package,
            device,
            existing_state,
            false,
            platform,
            context,
        );
    }
    let defaults_before = platform.default_endpoints();
    let oem = platform.stage_package(&package.inf)?;
    let (device, reboot) = match platform.create_device(&package.inf) {
        Ok(created) => created,
        Err(failure) => {
            // Rollback (17 §6): the package was added but no device exists.
            let rollback = platform.unstage_package(&oem);
            return Err(Failure {
                message: format!(
                    "{}; rollback of {oem}: {}",
                    failure.message,
                    rollback
                        .map(|_| "removed".to_owned())
                        .unwrap_or_else(|r| format!("FAILED: {}", r.message))
                ),
                ..failure
            });
        }
    };
    let state = State {
        schema: STATE_SCHEMA,
        package_version: package.version,
        oem_inf: oem.clone(),
        device_instance_id: device.clone(),
        installed_at: platform.now_utc(),
        cable_names: existing_state.map(|s| s.cable_names).unwrap_or_default(),
    };
    if let Err(failure) = context.state.save(&state) {
        let removed = platform.remove_device(&device);
        let unstaged = platform.unstage_package(&oem);
        return Err(Failure {
            message: format!(
                "{}; rolled back device ({}) and package ({})",
                failure.message,
                if removed.is_ok() { "removed" } else { "FAILED" },
                if unstaged.is_ok() {
                    "removed"
                } else {
                    "FAILED"
                }
            ),
            ..failure
        });
    }
    if reboot {
        return Ok(success(
            "install",
            true,
            serde_json::json!({
                "packageVersion": package.version.to_string(), "oemInf": oem, "deviceInstanceId": device,
            }),
        ));
    }
    let count = configured_count(platform, &device);
    let endpoints = wait_for_cables(platform, count)?;
    apply_names(platform, &state.cable_names, &endpoints)?;
    let defaults_after = platform.default_endpoints();
    Ok(success(
        "install",
        false,
        serde_json::json!({
            "packageVersion": package.version.to_string(),
            "oemInf": oem,
            "deviceInstanceId": device,
            "cableCount": count,
            "endpoints": endpoints_json(&endpoints),
            "defaultsChanged": changed_defaults(&defaults_before, &defaults_after),
        }),
    ))
}

/// Install a package on the existing device: newer → update, same → repair
/// semantics when `force`, older → refused (no downgrade).
fn apply_to_existing(
    command: &str,
    package: PackageInfo,
    device: String,
    existing_state: Option<State>,
    force_same: bool,
    platform: &mut dyn Platform,
    context: &Context,
) -> Result<Outcome, Failure> {
    let installed = platform.device_driver_version(&device)?;
    if let Some(installed) = installed {
        if package.version < installed {
            return Err(Failure::invalid(format!(
                "the installed driver {installed} is newer than this package {}; downgrades are not supported",
                package.version
            )));
        }
    }
    let same = installed == Some(package.version);
    let defaults_before = platform.default_endpoints();
    let oem = platform.stage_package(&package.inf)?;
    let reboot = if same && !force_same {
        false
    } else {
        platform.update_device(&package.inf, same)?
    };
    let previous_oem = existing_state.as_ref().map(|s| s.oem_inf.clone());
    let state = State {
        schema: STATE_SCHEMA,
        package_version: package.version,
        oem_inf: oem.clone(),
        device_instance_id: device.clone(),
        installed_at: platform.now_utc(),
        cable_names: existing_state.map(|s| s.cable_names).unwrap_or_default(),
    };
    context.state.save(&state)?;
    // The superseded package is no longer bound to the device; remove only
    // that exact oem*.inf. Failure is reported, never fatal.
    let superseded = previous_oem
        .filter(|previous| !previous.eq_ignore_ascii_case(&oem))
        .map(|previous| match platform.unstage_package(&previous) {
            Ok(()) => format!("{previous} removed"),
            Err(failure) => format!("{previous} kept: {}", failure.message),
        });
    if reboot {
        return Ok(success(
            command,
            true,
            serde_json::json!({
                "packageVersion": package.version.to_string(), "oemInf": oem, "deviceInstanceId": device,
                "supersededPackage": superseded,
            }),
        ));
    }
    let count = configured_count(platform, &device);
    let endpoints = wait_for_cables(platform, count)?;
    apply_names(platform, &state.cable_names, &endpoints)?;
    let defaults_after = platform.default_endpoints();
    Ok(success(
        command,
        false,
        serde_json::json!({
            "packageVersion": package.version.to_string(),
            "previousDriverVersion": installed.map(|v| v.to_string()),
            "oemInf": oem,
            "deviceInstanceId": device,
            "cableCount": count,
            "endpoints": endpoints_json(&endpoints),
            "defaultsChanged": changed_defaults(&defaults_before, &defaults_after),
            "supersededPackage": superseded,
        }),
    ))
}

pub fn update(
    requested: &Path,
    platform: &mut dyn Platform,
    context: &Context,
) -> Result<Outcome, Failure> {
    let package = verified_package(requested, platform, context)?;
    let device = platform.find_device()?.ok_or_else(|| {
        Failure::invalid("the AudioRouter cable driver is not installed; use install")
    })?;
    let state = context.state.load()?;
    apply_to_existing("update", package, device, state, false, platform, context)
}

pub fn repair(
    requested: &Path,
    platform: &mut dyn Platform,
    context: &Context,
) -> Result<Outcome, Failure> {
    let package = verified_package(requested, platform, context)?;
    let state = context.state.load()?;
    match platform.find_device()? {
        Some(device) => {
            apply_to_existing("repair", package, device, state, true, platform, context)
        }
        None => {
            // The device was deleted (for example in Device Manager): create
            // it again from the same package, keeping configured names.
            let names = state
                .as_ref()
                .map(|s| s.cable_names.clone())
                .unwrap_or_default();
            if state.is_some() {
                context.state.delete()?;
            }
            let outcome = install(requested, platform, context)?;
            if !names.is_empty() {
                if let Some(mut saved) = context.state.load()? {
                    saved.cable_names = names;
                    context.state.save(&saved)?;
                }
            }
            let mut json = outcome.json;
            json["command"] = "repair".into();
            Ok(Outcome {
                kind: outcome.kind,
                json,
            })
        }
    }
}

fn require_device(platform: &mut dyn Platform) -> Result<String, Failure> {
    platform
        .find_device()?
        .ok_or_else(|| Failure::invalid("the AudioRouter cable driver is not installed"))
}

fn restart_and_wait(
    command: &str,
    device: &str,
    count: u8,
    platform: &mut dyn Platform,
    context: &Context,
    extra: serde_json::Value,
) -> Result<Outcome, Failure> {
    if platform.restart_device(device)? {
        let mut fields = extra;
        fields["cableCount"] = count.into();
        return Ok(success(command, true, fields));
    }
    let endpoints = wait_for_cables(platform, count)?;
    let names = context
        .state
        .load()?
        .map(|s| s.cable_names)
        .unwrap_or_default();
    apply_names(platform, &names, &endpoints)?;
    let mut fields = extra;
    fields["cableCount"] = count.into();
    fields["endpoints"] = endpoints_json(&endpoints);
    Ok(success(command, false, fields))
}

pub fn set_cables(
    count: u8,
    platform: &mut dyn Platform,
    context: &Context,
) -> Result<Outcome, Failure> {
    let device = require_device(platform)?;
    let previous = configured_count(platform, &device);
    platform.write_device_dword(&device, "CableCount", u32::from(count))?;
    restart_and_wait(
        "set-cables",
        &device,
        count,
        platform,
        context,
        serde_json::json!({ "previousCableCount": previous }),
    )
}

pub fn configure(
    names: &BTreeMap<u8, String>,
    params: &BTreeMap<Parameter, u32>,
    platform: &mut dyn Platform,
    context: &Context,
) -> Result<Outcome, Failure> {
    let device = require_device(platform)?;
    for name in names.values() {
        validate_cable_name(name)?;
    }
    // Validate the effective period pair against what is already stored.
    let min = match params.get(&Parameter::MinPeriodFrames) {
        Some(value) => Some(*value),
        None => platform.read_device_dword(&device, "MinPeriodFrames")?,
    };
    let default = match params.get(&Parameter::DefaultPeriodFrames) {
        Some(value) => Some(*value),
        None => platform.read_device_dword(&device, "DefaultPeriodFrames")?,
    };
    if let (Some(min), Some(default)) = (min, default) {
        if default < min {
            return Err(Failure::invalid(
                "DefaultPeriodFrames cannot be below MinPeriodFrames",
            ));
        }
    }
    for (parameter, value) in params {
        let (low, high) = parameter.range();
        if !(low..=high).contains(value) {
            return Err(Failure::invalid(format!(
                "{} must be {low}-{high}",
                parameter.registry_name()
            )));
        }
        platform.write_device_dword(&device, parameter.registry_name(), *value)?;
    }
    if !names.is_empty() {
        let mut state = context.state.load()?.ok_or_else(|| {
            Failure::invalid("no helper state; install the driver with the helper first")
        })?;
        state
            .cable_names
            .extend(names.iter().map(|(index, name)| (*index, name.clone())));
        context.state.save(&state)?;
    }
    let count = configured_count(platform, &device);
    if params.is_empty() {
        // Names only: no restart needed, apply to present endpoints now.
        let endpoints = wait_for_cables(platform, count)?;
        apply_names(platform, names, &endpoints)?;
        return Ok(success(
            "configure",
            false,
            serde_json::json!({
                "cableCount": count, "endpoints": endpoints_json(&platform.cable_endpoints()),
            }),
        ));
    }
    let written: BTreeMap<&str, u32> = params
        .iter()
        .map(|(p, v)| (p.registry_name(), *v))
        .collect();
    restart_and_wait(
        "configure",
        &device,
        count,
        platform,
        context,
        serde_json::json!({ "parameters": written }),
    )
}

pub fn remove(platform: &mut dyn Platform, context: &Context) -> Result<Outcome, Failure> {
    let state = context.state.load()?.ok_or_else(|| {
        Failure::invalid("no helper state exists; refusing to remove drivers it did not install")
    })?;
    let mut reboot = false;
    if let Some(device) = platform.find_device()? {
        reboot |= platform.remove_device(&device)?;
    }
    platform.unstage_package(&state.oem_inf)?;
    context.state.delete()?;
    let left = platform.find_device()?;
    if left.is_some() && !reboot {
        return Err(Failure {
            kind: ExitKind::WindowsRefused,
            message: "the AudioRouter device is still present after removal".into(),
            hresult: None,
        });
    }
    Ok(success(
        "remove",
        reboot,
        serde_json::json!({
            "removedPackage": state.oem_inf,
            "packageVersion": state.package_version.to_string(),
        }),
    ))
}

#[cfg(test)]
mod tests;
