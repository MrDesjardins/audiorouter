//! The bundled sound library (Soundboard, SND-08): listing, reading and
//! loading `lib:` sounds for playback.
//!
//! The library ships beside the program as `sound-library/` (a `library.json`
//! manifest plus FLAC files). A sound is reachable only through an id in that
//! manifest, so a request can never name an arbitrary file, and each file is
//! checked against its manifest SHA-256 before it is decoded or returned.

use super::*;
use audiorouter_domain::{SOUNDBOARD_SLOTS, SOUND_LIBRARY_PREFIX};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

const LIBRARY_DIR: &str = "sound-library";
const MANIFEST_FILE: &str = "library.json";
const MAX_MANIFEST_BYTES: u64 = 2 * 1024 * 1024;
const MAX_LIBRARY_SOUND_BYTES: u64 = 2 * 1024 * 1024;
/// Total decoded audio one board may hold (about 256 MB, owner decision
/// 2026-10-29).
const MAX_BOARD_DECODED_BYTES: usize = 256 * 1024 * 1024;

/// Where the library lives: the override variable, else `sound-library` or
/// `resources/sound-library` beside the program or in a parent folder (the
/// repository checkout during development).
fn library_root() -> Option<PathBuf> {
    let valid = |path: &PathBuf| path.join(MANIFEST_FILE).is_file();
    if let Some(path) = std::env::var_os("AUDIOROUTER_SOUND_LIBRARY").map(PathBuf::from) {
        return valid(&path).then_some(path);
    }
    let mut directory = std::env::current_exe().ok()?.parent()?.to_path_buf();
    for _ in 0..6 {
        for candidate in [
            directory.join(LIBRARY_DIR),
            directory.join("resources").join(LIBRARY_DIR),
        ] {
            if valid(&candidate) {
                return Some(candidate);
            }
        }
        if !directory.pop() {
            break;
        }
    }
    None
}

fn read_manifest(root: &std::path::Path) -> Result<Value, ControlError> {
    let path = root.join(MANIFEST_FILE);
    let too_big = std::fs::metadata(&path).map_or(true, |meta| meta.len() > MAX_MANIFEST_BYTES);
    if too_big {
        return Err(ControlError::InvalidRequest(
            "the sound library manifest is missing or too large".into(),
        ));
    }
    let text = std::fs::read_to_string(&path).map_err(|_| {
        ControlError::InvalidRequest("the sound library manifest could not be read".into())
    })?;
    serde_json::from_str(&text).map_err(|_| {
        ControlError::InvalidRequest("the sound library manifest is not valid JSON".into())
    })
}

fn encode_base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut text = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for group in bytes.chunks(3) {
        let value = (u32::from(group[0]) << 16)
            | (u32::from(*group.get(1).unwrap_or(&0)) << 8)
            | u32::from(*group.get(2).unwrap_or(&0));
        for index in 0..4usize {
            if index <= group.len() {
                text.push(ALPHABET[((value >> (18 - 6 * index)) & 63) as usize] as char);
            } else {
                text.push('=');
            }
        }
    }
    text
}

impl ControlPlane {
    /// The verified FLAC bytes of library sound `id` (`lib:...`).
    pub(crate) fn read_library_sound(id: &str) -> Result<Vec<u8>, ControlError> {
        let not_found =
            || ControlError::InvalidRequest(format!("\"{id}\" is not in the sound library"));
        if !id.starts_with(SOUND_LIBRARY_PREFIX) {
            return Err(not_found());
        }
        let root = library_root().ok_or_else(|| {
            ControlError::InvalidRequest("the sound library is not installed".into())
        })?;
        let manifest = read_manifest(&root)?;
        let entry = manifest
            .get("sounds")
            .and_then(Value::as_array)
            .and_then(|sounds| {
                sounds
                    .iter()
                    .find(|sound| sound.get("id").and_then(Value::as_str) == Some(id))
            })
            .ok_or_else(not_found)?;
        let file = entry
            .get("file")
            .and_then(Value::as_str)
            .filter(|file| {
                !file.is_empty() && !file.contains(['/', '\\', ':']) && !file.starts_with('.')
            })
            .ok_or_else(not_found)?;
        let path = root.join(file);
        if std::fs::metadata(&path).map_or(true, |meta| meta.len() > MAX_LIBRARY_SOUND_BYTES) {
            return Err(not_found());
        }
        let bytes = std::fs::read(&path).map_err(|_| not_found())?;
        let expected = entry.get("sha256").and_then(Value::as_str).unwrap_or("");
        let actual = Sha256::digest(&bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        if !actual.eq_ignore_ascii_case(expected) {
            return Err(ControlError::InvalidRequest(format!(
                "the library sound \"{id}\" failed its integrity check"
            )));
        }
        Ok(bytes)
    }

    pub(crate) fn dispatch_sound_library_list(&self) -> Result<Value, ControlError> {
        let Some(root) = library_root() else {
            return Ok(json!({ "installed": false, "categories": [], "sounds": [] }));
        };
        let manifest = read_manifest(&root)?;
        let categories = manifest
            .get("categories")
            .and_then(Value::as_object)
            .map(|map| {
                map.iter()
                    .map(|(id, title)| json!({ "id": id, "title": title }))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let sounds = manifest
            .get("sounds")
            .and_then(Value::as_array)
            .map(|sounds| {
                sounds
                    .iter()
                    .map(|sound| {
                        let field = |name: &str| sound.get(name).cloned().unwrap_or(Value::Null);
                        let source = sound.get("source");
                        let from = |name: &str| {
                            source
                                .and_then(|source| source.get(name))
                                .cloned()
                                .unwrap_or(Value::Null)
                        };
                        json!({
                            "id": field("id"), "title": field("title"),
                            "category": field("category"), "tags": field("tags"),
                            "durationMs": field("durationMs"),
                            "license": from("license"), "author": from("author"),
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        Ok(json!({ "installed": true, "categories": categories, "sounds": sounds }))
    }

    pub(crate) fn dispatch_sound_library_read(
        &self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let id = params
            .as_ref()
            .and_then(|params| params.get("id"))
            .and_then(Value::as_str)
            .ok_or_else(|| ControlError::InvalidRequest("id is required".into()))?;
        let bytes = Self::read_library_sound(id)?;
        Ok(json!({ "id": id, "format": "flac", "dataBase64": encode_base64(&bytes) }))
    }

    /// Decodes, at the graph rate, every sound the enabled Soundboards of
    /// `session` reference: `lib:` sounds from the library and the user's own
    /// from the media store. A slot whose sound is missing or cannot be
    /// decoded stays empty (the route still starts); a board over the decoded
    /// budget is refused so memory stays bounded.
    pub(crate) fn load_soundboard_media(
        &mut self,
        session: &Session,
        sample_rate_hz: u32,
        media: &mut HashMap<String, Arc<DecodedAudio>>,
    ) -> Result<(), ControlError> {
        for node in session
            .nodes
            .iter()
            .filter(|node| node.enabled && node.kind == NodeKind::Soundboard)
        {
            let mut ids = Vec::<String>::new();
            for slot in 1..=SOUNDBOARD_SLOTS {
                if let Some(id) = node
                    .parameters
                    .get(&format!("slot{slot}MediaId"))
                    .and_then(Value::as_str)
                {
                    ids.push(id.to_owned());
                }
                if let Some(Value::Array(set)) = node.parameters.get(&format!("slot{slot}Set")) {
                    ids.extend(set.iter().filter_map(Value::as_str).map(str::to_owned));
                }
            }
            let mut budget = 0usize;
            for id in ids {
                if media.contains_key(&id) {
                    budget += media[&id].samples.len() * std::mem::size_of::<f32>();
                    continue;
                }
                let loaded = if id.starts_with(SOUND_LIBRARY_PREFIX) {
                    Self::read_library_sound(&id)
                        .ok()
                        .map(|bytes| (bytes, "flac".to_owned()))
                } else {
                    self.storage
                        .as_ref()
                        .and_then(|storage| storage.load_audio_media(&id).ok().flatten())
                        .map(|(_, format, bytes)| (bytes, format))
                };
                let Some((bytes, format)) = loaded else {
                    continue;
                };
                let Ok(decoded) = self.while_servicing_audio(|| {
                    audiorouter_engine::decode_audio_bytes(bytes, &format, sample_rate_hz)
                }) else {
                    continue;
                };
                budget += decoded.samples.len() * std::mem::size_of::<f32>();
                if budget > MAX_BOARD_DECODED_BYTES {
                    return Err(ControlError::NodeNotReady {
                        node_id: node.id.as_str().to_owned(),
                        message: format!(
                            "\"{}\" holds more sound than it can keep ready (limit 256 MB decoded). Remove a long sound.",
                            node.name
                        ),
                    });
                }
                media.insert(id, Arc::new(decoded));
            }
        }
        Ok(())
    }
}
