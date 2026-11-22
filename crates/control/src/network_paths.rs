//! Network path refusal for every client that is not the desktop window.
//!
//! Opening a UNC path (`\\host\share\...`) or a mapped network drive makes
//! Windows connect to that host and offer the user's credentials (NTLM)
//! before any later check runs. A token holder, a script or an assistant,
//! which a prompt could mislead, must not be able to cause that. The person
//! at the AudioRouter window may still choose a network folder there.
//!
//! The backend owns this check (AGENTS.md: authorization lives in the
//! backend). Adapters such as the HTTP API may repeat it, never replace it.

use serde_json::Value;

impl crate::ControlPlane {
    /// The first parameter naming a network location the caller may not
    /// use. One exception: inside a whole-session `candidate` (`graph.plan`,
    /// a preview Start), a location the targeted session (its `sessionId`,
    /// or the open session) already holds may come back unchanged. Anywhere
    /// else (`plugins.inspect path`, `presetPath`, ...) every network
    /// location is refused, even one the session holds. The stored session
    /// is read only when a network value is present, so ordinary requests
    /// pay one walk of their parameters.
    pub(crate) fn new_network_location(&self, params: &Value) -> Option<String> {
        network_location_parameter(params)?;
        let session_id = params
            .get("sessionId")
            .and_then(Value::as_str)
            .map(audiorouter_domain::EntityId::new)
            .or_else(|| self.active_session_id.clone());
        let known = session_id
            .and_then(|id| self.store.session(&id))
            .and_then(|session| serde_json::to_value(session).ok())
            .map(|session| network_locations(&session))
            .unwrap_or_default();
        let mut found = None;
        visit_locations(
            params,
            &mut String::new(),
            false,
            &mut |pointer: &str, text: &str| {
                let resent = pointer.starts_with("/candidate/") && known.iter().any(|k| k == text);
                if is_network_path(text) && !resent {
                    found = Some(pointer.to_owned());
                    return true;
                }
                false
            },
        );
        found
    }
}

/// True when `text` names a network location:
/// - a UNC path in any slash spelling (`\\host\share`, `//host`, `\/host`);
/// - a namespace path that reaches UNC (`\\?\UNC\...`, `\\.\UNC\...`,
///   `\??\UNC\...`, and `GLOBALROOT`, which can name the network redirector);
/// - on Windows, a drive letter (plain or `\\?\X:`) that maps to a network
///   share or to no drive at all.
///
/// Local device-namespace paths stay local: verbatim drive paths that
/// `fs::canonicalize` returns (`\\?\C:\...`), pipes and the fixed bridge
/// device (`\\.\AudioRouterVirtualBridge`). The check reads only the
/// spelling and the drive type; it never opens the path.
pub fn is_network_path(text: &str) -> bool {
    let bytes = text.as_bytes();
    let separator = |byte: u8| byte == b'\\' || byte == b'/';
    // Namespace prefixes Win32 passes through: `\\?\`, `\\.\` and the NT
    // object-manager form `\??\`.
    let namespace = bytes.len() >= 4
        && separator(bytes[0])
        && separator(bytes[3])
        && ((separator(bytes[1]) && matches!(bytes[2], b'?' | b'.'))
            || (bytes[1] == b'?' && bytes[2] == b'?'));
    if namespace {
        let rest = &bytes[4..];
        let starts_with = |prefix: &[u8]| {
            rest.len() >= prefix.len() && rest[..prefix.len()].eq_ignore_ascii_case(prefix)
        };
        if starts_with(b"UNC\\") || starts_with(b"UNC/") || starts_with(b"GLOBALROOT") {
            return true;
        }
        return drive_is_remote(rest);
    }
    if bytes.len() >= 2 && separator(bytes[0]) && separator(bytes[1]) {
        return true;
    }
    drive_is_remote(bytes)
}

/// True when `bytes` starts with a drive letter whose root is a network
/// share (DRIVE_REMOTE) or does not exist (DRIVE_NO_ROOT_DIR).
fn drive_is_remote(bytes: &[u8]) -> bool {
    if !(bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':') {
        return false;
    }
    #[cfg(windows)]
    {
        let root = [u16::from(bytes[0]), u16::from(b':'), u16::from(b'\\'), 0];
        // SAFETY: `root` is a NUL-terminated drive root that stays alive for
        // this metadata-only call; GetDriveTypeW retains no pointer.
        let kind = unsafe {
            windows::Win32::Storage::FileSystem::GetDriveTypeW(windows::core::PCWSTR(root.as_ptr()))
        };
        // DRIVE_NO_ROOT_DIR (1) or DRIVE_REMOTE (4).
        matches!(kind, 1 | 4)
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// A parameter name that holds a file or folder location: `path`,
/// `presetPath`, `directory`, `shellPath`, `root`, `newPath`,
/// `renderMappingPath` and so on. Matching by name, not by a list of
/// methods, means a new path parameter is covered without an edit here.
///
/// Deliberately broad (`paths`, `directories`, `presetFile`, `roots`,
/// `location`): a false match only refuses a UNC-looking string under an
/// odd name, while a missed one opens an SMB connection.
fn is_location_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    [
        "path", "director", "folder", "file", "root", "location", "dll",
    ]
    .iter()
    .any(|word| key.contains(word))
}

/// The first parameter (as a JSON pointer) that names a network location,
/// searching nested objects and arrays (a node's `path` inside a graph
/// candidate or an imported session counts). Bounded by the decoded request
/// limits the dispatcher already enforces.
pub fn network_location_parameter(params: &Value) -> Option<String> {
    let mut found = None;
    visit_locations(
        params,
        &mut String::new(),
        false,
        &mut |pointer: &str, text: &str| {
            if is_network_path(text) {
                found = Some(pointer.to_owned());
                return true;
            }
            false
        },
    );
    found
}

/// Every network location stored under a location key in `value`.
fn network_locations(value: &Value) -> Vec<String> {
    let mut locations = Vec::new();
    visit_locations(
        value,
        &mut String::new(),
        false,
        &mut |_: &str, text: &str| {
            if is_network_path(text) {
                locations.push(text.to_owned());
            }
            false
        },
    );
    locations
}

/// Call `found(pointer, text)` for each string under a location key, at any
/// depth, until it returns true. Bounded by the decoded request limits the
/// dispatcher already enforces.
fn visit_locations(
    value: &Value,
    pointer: &mut String,
    location_key: bool,
    found: &mut dyn FnMut(&str, &str) -> bool,
) -> bool {
    match value {
        Value::String(text) if location_key => found(pointer, text),
        Value::Object(map) => {
            for (key, child) in map {
                if visit_child(child, pointer, key, is_location_key(key), &mut *found) {
                    return true;
                }
            }
            false
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                let segment = index.to_string();
                if visit_child(child, pointer, &segment, location_key, &mut *found) {
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}

fn visit_child(
    value: &Value,
    pointer: &mut String,
    segment: &str,
    location_key: bool,
    found: &mut dyn FnMut(&str, &str) -> bool,
) -> bool {
    let length = pointer.len();
    pointer.push('/');
    // RFC 6901 escaping, so `fieldPath` names the right member.
    pointer.push_str(&segment.replace('~', "~0").replace('/', "~1"));
    let stop = visit_locations(value, pointer, location_key, found);
    pointer.truncate(length);
    stop
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn every_unc_spelling_is_a_network_path() {
        for text in [
            r"\\host\share\preset.fxp",
            "//host/share/preset.fxp",
            r"\/host\share",
            r"/\host\share",
            r"\\?\UNC\host\share\x",
            r"\\?\unc\host\share\x",
            r"\\.\UNC\host\share\x",
            "//?/UNC/host/share",
            r"\\?\GLOBALROOT\Device\Mup\host\share",
            r"\??\UNC\host\share\x",
            r"\??\GLOBALROOT\Device\Mup\host\share",
        ] {
            assert!(is_network_path(text), "{text}");
        }
        // Local device-namespace paths: the bridge device, a pipe, and the
        // verbatim form `fs::canonicalize` returns for local files.
        for text in [
            "",
            "C",
            "relative\\x",
            "/single/slash",
            r"\single",
            r"\\.\AudioRouterVirtualBridge",
            r"\\.\pipe\audiorouter-control",
        ] {
            assert!(!is_network_path(text), "{text}");
        }
        #[cfg(windows)]
        {
            let system = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
            assert!(!is_network_path(&format!(r"{system}\Windows")));
            assert!(!is_network_path(&format!(r"\\?\{system}\Windows")));
        }
    }

    #[test]
    fn location_parameters_are_found_by_name_at_any_depth() {
        assert_eq!(
            network_location_parameter(&json!({ "presetPath": r"\\h\s\x.fxp" })).as_deref(),
            Some("/presetPath")
        );
        assert_eq!(
            network_location_parameter(&json!({
                "session": { "nodes": [ { "parameters": { "path": "//h/s/a.wav" } } ] }
            }))
            .as_deref(),
            Some("/session/nodes/0/parameters/path")
        );
        assert_eq!(
            network_location_parameter(&json!({ "directory": "//h/s", "root": "x" })).as_deref(),
            Some("/directory")
        );
        for key in [
            "directories",
            "presetFile",
            "searchRoots",
            "pluginDll",
            "paths",
        ] {
            assert_eq!(
                network_location_parameter(&Value::Object(
                    [(key.to_owned(), json!([r"\\h\s"]))].into_iter().collect()
                )),
                Some(format!("/{key}/0")),
                "{key}"
            );
        }
        assert_eq!(
            network_location_parameter(&json!({ "a/b": { "path": "//h/s" } })).as_deref(),
            Some("/a~1b/path")
        );
        // A network-looking value under a key that is not a location (a
        // display name, a note) is only text.
        assert_eq!(
            network_location_parameter(&json!({ "name": r"\\not a path", "comment": "//x" })),
            None
        );
        assert_eq!(
            network_location_parameter(&json!({ "path": r"C:\Plugins\fx.dll" })),
            None
        );
    }

    #[test]
    fn the_network_locations_a_session_holds_are_collected() {
        // The candidate exemption itself is tested through the dispatcher:
        // `network_locations_are_refused_for_clients_other_than_the_window`.
        let stored = json!({ "nodes": [
            { "parameters": { "path": r"\\nas\music\intro.wav" } },
            { "parameters": { "path": r"C:\local.wav" } },
            { "name": r"\\not\a\location" }
        ] });
        assert_eq!(
            network_locations(&stored),
            vec![r"\\nas\music\intro.wav".to_owned()]
        );
    }
}
