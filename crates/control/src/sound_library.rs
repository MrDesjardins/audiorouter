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
/// Total decoded audio all the Soundboards of one route may hold together,
/// so several large boards stay bounded too (review 2026-11-01, P3).
const MAX_SESSION_DECODED_BYTES: usize = 512 * 1024 * 1024;

/// The decoding budgets of [`ControlPlane::load_soundboard_media`].
#[derive(Clone, Copy, Debug)]
pub(crate) struct SoundboardBudgets {
    pub(crate) board_bytes: usize,
    pub(crate) session_bytes: usize,
}

impl Default for SoundboardBudgets {
    fn default() -> Self {
        Self {
            board_bytes: MAX_BOARD_DECODED_BYTES,
            session_bytes: MAX_SESSION_DECODED_BYTES,
        }
    }
}

/// What one Soundboard load did, for tests.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct SoundboardLoadStats {
    /// Times the library manifest was read and parsed (at most once).
    pub(crate) manifest_reads: usize,
    /// Sounds read and decoded successfully on a worker, off the audio
    /// service thread.
    pub(crate) decoded: usize,
}

/// A library sound's file and expected SHA-256, looked up by id.
#[derive(Clone, Debug)]
struct LibraryEntry {
    file: String,
    sha256: String,
}

/// The library manifest's sounds indexed by id. Built once per load.
fn library_index(manifest: &Value) -> HashMap<String, LibraryEntry> {
    manifest
        .get("sounds")
        .and_then(Value::as_array)
        .map(|sounds| {
            sounds
                .iter()
                .filter_map(|sound| {
                    let id = sound.get("id").and_then(Value::as_str)?;
                    let file = sound.get("file").and_then(Value::as_str)?;
                    let sha256 = sound.get("sha256").and_then(Value::as_str).unwrap_or("");
                    Some((
                        id.to_owned(),
                        LibraryEntry {
                            file: file.to_owned(),
                            sha256: sha256.to_owned(),
                        },
                    ))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The bytes of library sound `id` from its manifest entry, refused unless
/// the file name stays inside the library folder, fits the size bound and
/// matches its SHA-256.
fn read_verified_library_file(
    root: &std::path::Path,
    id: &str,
    entry: Option<&LibraryEntry>,
) -> Result<Vec<u8>, ControlError> {
    let not_found =
        || ControlError::InvalidRequest(format!("\"{id}\" is not in the sound library"));
    let entry = entry.ok_or_else(not_found)?;
    let file = Some(entry.file.as_str())
        .filter(|file| {
            !file.is_empty() && !file.contains(['/', '\\', ':']) && !file.starts_with('.')
        })
        .ok_or_else(not_found)?;
    let path = root.join(file);
    if std::fs::metadata(&path).map_or(true, |meta| meta.len() > MAX_LIBRARY_SOUND_BYTES) {
        return Err(not_found());
    }
    let bytes = std::fs::read(&path).map_err(|_| not_found())?;
    let actual = Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    if !actual.eq_ignore_ascii_case(&entry.sha256) {
        return Err(ControlError::InvalidRequest(format!(
            "the library sound \"{id}\" failed its integrity check"
        )));
    }
    Ok(bytes)
}

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
        let index = library_index(&read_manifest(&root)?);
        read_verified_library_file(&root, id, index.get(id))
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
    /// budget, or a route over the session budget, is refused so memory stays
    /// bounded.
    pub(crate) fn load_soundboard_media(
        &mut self,
        session: &Session,
        sample_rate_hz: u32,
        media: &mut HashMap<String, Arc<DecodedAudio>>,
    ) -> Result<(), ControlError> {
        self.load_soundboard_media_with(
            session,
            sample_rate_hz,
            media,
            SoundboardBudgets::default(),
        )
        .map(|_| ())
    }

    /// [`Self::load_soundboard_media`] with explicit budgets. The library
    /// manifest is read and indexed at most once; every file read, hash and
    /// decode runs off this thread while the running audio is serviced, and
    /// each of the user's own sounds is read through a separate worker
    /// connection too (review 2026-11-01, P2-8 and P0-1 remainder).
    pub(crate) fn load_soundboard_media_with(
        &mut self,
        session: &Session,
        sample_rate_hz: u32,
        media: &mut HashMap<String, Arc<DecodedAudio>>,
        budgets: SoundboardBudgets,
    ) -> Result<SoundboardLoadStats, ControlError> {
        let boards = session
            .nodes
            .iter()
            .filter(|node| node.enabled && node.kind == NodeKind::Soundboard)
            .collect::<Vec<_>>();
        let mut stats = SoundboardLoadStats::default();
        let needs_library = boards.iter().any(|node| {
            (1..=SOUNDBOARD_SLOTS).any(|slot| {
                let lib = |value: Option<&Value>| {
                    value
                        .and_then(Value::as_str)
                        .is_some_and(|id| id.starts_with(SOUND_LIBRARY_PREFIX))
                };
                lib(node.parameters.get(&format!("slot{slot}MediaId")))
                    || match node.parameters.get(&format!("slot{slot}Set")) {
                        Some(Value::Array(set)) => set.iter().any(|id| lib(Some(id))),
                        Some(Value::String(_)) => true,
                        _ => false,
                    }
            })
        });
        let library = if needs_library {
            stats.manifest_reads += 1;
            self.while_servicing_audio(|| {
                library_root().and_then(|root| {
                    let manifest = read_manifest(&root).ok()?;
                    let index = library_index(&manifest);
                    Some((root, manifest, index))
                })
            })
        } else {
            None
        };
        let mut session_counted = std::collections::HashSet::<String>::new();
        let mut session_bytes = 0usize;
        for node in boards {
            // (media key, sound id): a category sound is stored under its
            // category key as well as its own id (SND-17).
            let mut ids = Vec::<(String, String)>::new();
            for slot in 1..=SOUNDBOARD_SLOTS {
                if let Some(id) = node
                    .parameters
                    .get(&format!("slot{slot}MediaId"))
                    .and_then(Value::as_str)
                {
                    ids.push((id.to_owned(), id.to_owned()));
                }
                match node.parameters.get(&format!("slot{slot}Set")) {
                    Some(Value::Array(set)) => ids.extend(
                        set.iter()
                            .filter_map(Value::as_str)
                            .map(|id| (id.to_owned(), id.to_owned())),
                    ),
                    Some(Value::String(category)) => {
                        let manifest = library.as_ref().map(|(_, manifest, _)| manifest);
                        for id in category_sounds(manifest, category) {
                            ids.push((
                                audiorouter_engine::soundboard_category_media_key(category, &id),
                                id,
                            ));
                        }
                    }
                    _ => {}
                }
            }
            // Each decoded sound counts once against the board's budget.
            let mut counted = std::collections::HashSet::<String>::new();
            let mut budget = 0usize;
            let mut count = |id: &str, audio: &DecodedAudio, budget: &mut usize| {
                if counted.insert(id.to_owned()) {
                    *budget += audio.samples.len() * std::mem::size_of::<f32>();
                }
            };
            for (key, id) in ids {
                let audio = match media.get(&id).cloned() {
                    Some(audio) => audio,
                    None => {
                        let library = library
                            .as_ref()
                            .map(|(root, _, index)| (root.as_path(), index));
                        match self.decode_board_sound(&id, sample_rate_hz, library, &mut stats) {
                            Some(audio) => {
                                media.insert(id.clone(), Arc::clone(&audio));
                                audio
                            }
                            None => continue,
                        }
                    }
                };
                count(&id, &audio, &mut budget);
                if session_counted.insert(id.clone()) {
                    session_bytes += audio.samples.len() * std::mem::size_of::<f32>();
                }
                if budget > budgets.board_bytes {
                    return Err(ControlError::NodeNotReady {
                        node_id: node.id.as_str().to_owned(),
                        message: format!(
                            "\"{}\" holds more sound than it can keep ready (limit {} MB decoded). Remove a long sound.",
                            node.name,
                            budgets.board_bytes / (1024 * 1024)
                        ),
                    });
                }
                if session_bytes > budgets.session_bytes {
                    return Err(ControlError::NodeNotReady {
                        node_id: node.id.as_str().to_owned(),
                        message: format!(
                            "The Soundboards of this route hold more sound than they can keep ready together (limit {} MB decoded). Remove a long sound or a board.",
                            budgets.session_bytes / (1024 * 1024)
                        ),
                    });
                }
                media.entry(key).or_insert(audio);
            }
        }
        Ok(stats)
    }

    /// Decodes one board sound: a `lib:` sound from the indexed library or
    /// the user's own from the media store. None when it is missing or cannot
    /// be decoded, so the slot stays empty and the route still starts.
    fn decode_board_sound(
        &mut self,
        id: &str,
        sample_rate_hz: u32,
        library: Option<(&std::path::Path, &HashMap<String, LibraryEntry>)>,
        stats: &mut SoundboardLoadStats,
    ) -> Option<Arc<DecodedAudio>> {
        let decoded = if id.starts_with(SOUND_LIBRARY_PREFIX) {
            let (root, index) = library?;
            self.while_servicing_audio(|| {
                let bytes = read_verified_library_file(root, id, index.get(id)).ok()?;
                audiorouter_engine::decode_audio_bytes(bytes, "flac", sample_rate_hz).ok()
            })
        } else {
            self.with_storage_worker(|storage| {
                let Some((_, format, bytes)) =
                    storage.load_audio_media(id).map_err(storage_error)?
                else {
                    return Ok(None);
                };
                Ok(audiorouter_engine::decode_audio_bytes(bytes, &format, sample_rate_hz).ok())
            })
            .ok()
            .flatten()
        };
        stats.decoded += usize::from(decoded.is_some());
        decoded.map(Arc::new)
    }
}

/// The ids of every sound of library category `category`, sorted by id so a
/// saved board keeps the same set across releases (SND-14), at most
/// `MAX_SLOT_SOUNDS`. Empty when the library or the category is missing; the
/// slot then falls back to its own sound.
fn category_sounds(manifest: Option<&Value>, category: &str) -> Vec<String> {
    let mut ids = manifest
        .and_then(|manifest| manifest.get("sounds"))
        .and_then(Value::as_array)
        .map(|sounds| {
            sounds
                .iter()
                .filter(|sound| sound.get("category").and_then(Value::as_str) == Some(category))
                .filter_map(|sound| sound.get("id").and_then(Value::as_str))
                .filter(|id| id.starts_with(SOUND_LIBRARY_PREFIX))
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    ids.sort();
    ids.dedup();
    ids.truncate(audiorouter_engine::MAX_SLOT_SOUNDS);
    ids
}

#[cfg(test)]
#[path = "sound_library_tests.rs"]
mod tests;
