//! Bounded parsing of VST2 `.fxp`/`.fxb` preset files.
//!
//! The format is a little-documented binary interchange format, so every
//! count, size and string is checked before it is used to allocate or index.
//! The parser returns owned data; applying it to a live plugin happens only in
//! the isolated worker through `effSetChunk` or parameter events.
//!
//! Layout follows Steinberg's `vst2persistence.cpp` (big-endian integers and
//! floats). Programs contain `CcnK`, body size, `FxCk`/`FPCh`, format version,
//! plugin ID/version, parameter count, a 28-byte name, then values or a chunk.
//! Banks contain `CcnK`, body size, `FxBk`/`FBCh`, bank version, plugin
//! ID/version, program count, and (for version >= 1) current program plus 124
//! reserved bytes. `FxBk` then contains complete nested program files;
//! `FBCh` contains one opaque bank chunk.

/// Largest preset body accepted, chosen to keep a hostile file from forcing a
/// large allocation while still covering the parameter banks plugins produce.
pub const MAX_VST2_PRESET_BYTES: usize = 16 * 1024 * 1024;
/// VST2 plugins expose at most this many parameters in practice; the host
/// already bounds descriptors at 4096.
pub const MAX_VST2_PRESET_PARAMETERS: usize = 4096;
/// Banks are bounded independently so a count cannot drive a loop for long.
pub const MAX_VST2_PRESET_PROGRAMS: usize = 256;
const NAME_BYTES: usize = 28;
const HEADER_BYTES: usize = 8;
const BODY_HEADER_BYTES: usize = 16;

#[derive(Debug, Clone, PartialEq)]
pub enum Vst2PresetError {
    TooShort,
    BadMagic,
    BodySizeMismatch,
    TooLarge,
    UnexpectedKind,
    InvalidCount,
    InvalidName,
    Truncated,
    TrailingBytes,
}

impl std::fmt::Display for Vst2PresetError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::TooShort => "the file is shorter than a VST2 preset header",
            Self::BadMagic => "the file does not start with the VST2 'CcnK' magic",
            Self::BodySizeMismatch => "the declared VST2 body size does not match the file",
            Self::TooLarge => "the VST2 preset is larger than AudioRouter accepts",
            Self::UnexpectedKind => "the VST2 preset kind is not FxCk, FPCh, FxBk or FBCh",
            Self::InvalidCount => "the VST2 preset declares an invalid count",
            Self::InvalidName => "the VST2 preset name is not valid text",
            Self::Truncated => "the VST2 preset ends before its declared data",
            Self::TrailingBytes => "the VST2 preset has data after its declared body",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for Vst2PresetError {}

#[derive(Debug, Clone, PartialEq)]
pub enum Vst2PresetBody {
    /// `FxCk`: a single program as normalized parameter values.
    ProgramParameters { name: String, parameters: Vec<f32> },
    /// `FPCh`: a single program as an opaque plugin chunk.
    ProgramChunk {
        name: String,
        parameter_count: usize,
        chunk: Vec<u8>,
    },
    /// `FxBk`: a bank of independently selectable programs.
    Bank {
        current_program: u32,
        programs: Vec<Vst2BankProgram>,
    },
    /// `FBCh`: a plugin-owned opaque bank chunk.
    BankChunk {
        current_program: u32,
        chunk: Vec<u8>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Vst2BankProgram {
    Parameters {
        name: String,
        parameters: Vec<f32>,
    },
    Chunk {
        name: String,
        parameter_count: usize,
        chunk: Vec<u8>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Vst2Preset {
    pub fx_id: u32,
    pub fx_version: u32,
    pub body: Vst2PresetBody,
}

/// Parse one `.fxp`/`.fxb` file without loading plugin code.
pub fn parse(bytes: &[u8]) -> Result<Vst2Preset, Vst2PresetError> {
    if bytes.len() > MAX_VST2_PRESET_BYTES {
        return Err(Vst2PresetError::TooLarge);
    }
    if bytes.len() < HEADER_BYTES {
        return Err(Vst2PresetError::TooShort);
    }
    if &bytes[..4] != b"CcnK" {
        return Err(Vst2PresetError::BadMagic);
    }
    let declared = read_u32(bytes, 4).ok_or(Vst2PresetError::Truncated)? as usize;
    let body = bytes
        .get(HEADER_BYTES..)
        .ok_or(Vst2PresetError::Truncated)?;
    if body.len() > MAX_VST2_PRESET_BYTES {
        return Err(Vst2PresetError::TooLarge);
    }
    if declared > body.len() {
        return Err(Vst2PresetError::Truncated);
    }
    if declared < body.len() {
        return Err(Vst2PresetError::BodySizeMismatch);
    }
    if body.len() < BODY_HEADER_BYTES {
        return Err(Vst2PresetError::Truncated);
    }
    let mut cursor = body;
    let kind = take_u32(&mut cursor)?;
    let format_version = take_u32(&mut cursor)?;
    let fx_id = take_u32(&mut cursor)?;
    let fx_version = take_u32(&mut cursor)?;
    let body = match &kind.to_be_bytes() {
        b"FxCk" => {
            if format_version != 1 {
                return Err(Vst2PresetError::InvalidCount);
            }
            let (name, parameters) = parse_parameter_program(&mut cursor)?;
            ensure_empty(cursor)?;
            Vst2PresetBody::ProgramParameters { name, parameters }
        }
        b"FPCh" => {
            if format_version != 1 {
                return Err(Vst2PresetError::InvalidCount);
            }
            let (name, parameter_count, chunk) = parse_chunk_program(&mut cursor)?;
            ensure_empty(cursor)?;
            Vst2PresetBody::ProgramChunk {
                name,
                parameter_count,
                chunk,
            }
        }
        b"FxBk" | b"FBCh" => {
            if format_version > 2 {
                return Err(Vst2PresetError::InvalidCount);
            }
            let parameter_bank = &kind.to_be_bytes() == b"FxBk";
            let (program_count, current_program) =
                parse_bank_header(&mut cursor, format_version, !parameter_bank)?;
            if parameter_bank {
                let programs = parse_parameter_bank(&mut cursor, program_count, fx_id)?;
                ensure_empty(cursor)?;
                Vst2PresetBody::Bank {
                    current_program,
                    programs,
                }
            } else {
                let chunk = parse_chunk_bank(&mut cursor)?;
                ensure_empty(cursor)?;
                Vst2PresetBody::BankChunk {
                    current_program,
                    chunk,
                }
            }
        }
        _ => return Err(Vst2PresetError::UnexpectedKind),
    };
    Ok(Vst2Preset {
        fx_id,
        fx_version,
        body,
    })
}

/// `FxCk`: parameter count, 28-byte name, then floats.
fn parse_parameter_program(cursor: &mut &[u8]) -> Result<(String, Vec<f32>), Vst2PresetError> {
    let parameter_count = take_u32(cursor)? as usize;
    if parameter_count > MAX_VST2_PRESET_PARAMETERS {
        return Err(Vst2PresetError::InvalidCount);
    }
    let name = take_name(cursor)?;
    let mut parameters = Vec::with_capacity(parameter_count);
    for _ in 0..parameter_count {
        let raw = take(cursor, 4)?;
        *cursor = &cursor[4..];
        let value = f32::from_be_bytes([raw[0], raw[1], raw[2], raw[3]]);
        if !value.is_finite() {
            return Err(Vst2PresetError::InvalidCount);
        }
        parameters.push(value.clamp(0.0, 1.0));
    }
    Ok((name, parameters))
}

/// `FPCh`: parameter count, 28-byte name, chunk size and chunk bytes.
fn parse_chunk_program(cursor: &mut &[u8]) -> Result<(String, usize, Vec<u8>), Vst2PresetError> {
    let parameter_count = take_u32(cursor)? as usize;
    if parameter_count > MAX_VST2_PRESET_PARAMETERS {
        return Err(Vst2PresetError::InvalidCount);
    }
    let name = take_name(cursor)?;
    let size = take_u32(cursor)? as usize;
    if size == 0 {
        return Err(Vst2PresetError::InvalidCount);
    }
    let chunk = take(cursor, size)?.to_vec();
    *cursor = &cursor[size..];
    Ok((name, parameter_count, chunk))
}

/// Bank header: program count, current program and 124 reserved bytes.
fn parse_bank_header(
    cursor: &mut &[u8],
    version: u32,
    allow_empty: bool,
) -> Result<(usize, u32), Vst2PresetError> {
    let count = take_u32(cursor)? as usize;
    if (!allow_empty && count == 0) || count > MAX_VST2_PRESET_PROGRAMS {
        return Err(Vst2PresetError::InvalidCount);
    }
    let current = if version >= 1 {
        let current = take_u32(cursor)?;
        if count > 0 && current as usize >= count {
            return Err(Vst2PresetError::InvalidCount);
        }
        take(cursor, 124)?;
        *cursor = &cursor[124..];
        current
    } else {
        0
    };
    Ok((count, current))
}

/// `FxBk` embeds complete `CcnK` program files.
fn parse_parameter_bank(
    cursor: &mut &[u8],
    program_count: usize,
    expected_fx_id: u32,
) -> Result<Vec<Vst2BankProgram>, Vst2PresetError> {
    if program_count == 0 || program_count > MAX_VST2_PRESET_PROGRAMS {
        return Err(Vst2PresetError::InvalidCount);
    }
    let mut programs = Vec::with_capacity(program_count);
    for _ in 0..program_count {
        let declared = read_u32(cursor, 4).ok_or(Vst2PresetError::Truncated)? as usize;
        let total = HEADER_BYTES
            .checked_add(declared)
            .ok_or(Vst2PresetError::TooLarge)?;
        if total > cursor.len() {
            return Err(Vst2PresetError::Truncated);
        }
        let program = parse_embedded_program(&cursor[..total], expected_fx_id)?;
        programs.push(program);
        *cursor = &cursor[total..];
    }
    Ok(programs)
}

/// Parse the only two program record kinds valid inside an `FxBk`. This is
/// deliberately non-recursive: nested bank records are rejected before they
/// can consume stack proportional to attacker-controlled file contents.
fn parse_embedded_program(
    bytes: &[u8],
    expected_fx_id: u32,
) -> Result<Vst2BankProgram, Vst2PresetError> {
    if bytes.len() < HEADER_BYTES + BODY_HEADER_BYTES || &bytes[..4] != b"CcnK" {
        return Err(Vst2PresetError::BadMagic);
    }
    let declared = read_u32(bytes, 4).ok_or(Vst2PresetError::Truncated)? as usize;
    if declared.checked_add(HEADER_BYTES) != Some(bytes.len()) {
        return Err(Vst2PresetError::BodySizeMismatch);
    }
    let mut cursor = &bytes[HEADER_BYTES..];
    let kind = take_u32(&mut cursor)?;
    let _version = take_u32(&mut cursor)?;
    let fx_id = take_u32(&mut cursor)?;
    let _fx_version = take_u32(&mut cursor)?;
    if fx_id != expected_fx_id {
        return Err(Vst2PresetError::InvalidCount);
    }
    let program = match &kind.to_be_bytes() {
        b"FxCk" => {
            let (name, parameters) = parse_parameter_program(&mut cursor)?;
            ensure_empty(cursor)?;
            Vst2BankProgram::Parameters { name, parameters }
        }
        b"FPCh" => {
            let (name, parameter_count, chunk) = parse_chunk_program(&mut cursor)?;
            ensure_empty(cursor)?;
            Vst2BankProgram::Chunk {
                name,
                parameter_count,
                chunk,
            }
        }
        _ => return Err(Vst2PresetError::UnexpectedKind),
    };
    Ok(program)
}

/// `FBCh` stores one opaque bank-level chunk, not a list of chunks.
fn parse_chunk_bank(cursor: &mut &[u8]) -> Result<Vec<u8>, Vst2PresetError> {
    let size = take_u32(cursor)? as usize;
    if size == 0 {
        return Err(Vst2PresetError::InvalidCount);
    }
    let chunk = take(cursor, size)?.to_vec();
    *cursor = &cursor[size..];
    Ok(chunk)
}

fn ensure_empty(cursor: &[u8]) -> Result<(), Vst2PresetError> {
    if cursor.is_empty() {
        Ok(())
    } else {
        Err(Vst2PresetError::TrailingBytes)
    }
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let raw = bytes.get(offset..offset + 4)?;
    Some(u32::from_be_bytes([raw[0], raw[1], raw[2], raw[3]]))
}

fn take(bytes: &[u8], length: usize) -> Result<&[u8], Vst2PresetError> {
    bytes.get(..length).ok_or(Vst2PresetError::Truncated)
}

fn take_u32(bytes: &mut &[u8]) -> Result<u32, Vst2PresetError> {
    let raw = take(bytes, 4)?;
    *bytes = &bytes[4..];
    Ok(u32::from_be_bytes([raw[0], raw[1], raw[2], raw[3]]))
}

fn take_name(bytes: &mut &[u8]) -> Result<String, Vst2PresetError> {
    let raw = take(bytes, NAME_BYTES)?;
    *bytes = &bytes[NAME_BYTES..];
    let end = raw.iter().position(|byte| *byte == 0).unwrap_or(raw.len());
    let text = std::str::from_utf8(&raw[..end]).map_err(|_| Vst2PresetError::InvalidName)?;
    // A name is display text; control characters would corrupt the UI.
    if text.chars().any(char::is_control) {
        return Err(Vst2PresetError::InvalidName);
    }
    Ok(text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(kind);
        data.extend_from_slice(
            &(if kind == b"FxBk" || kind == b"FBCh" {
                2_u32
            } else {
                1_u32
            })
            .to_be_bytes(),
        );
        data.extend_from_slice(&0x1234_5678_u32.to_be_bytes());
        data.extend_from_slice(&7_u32.to_be_bytes());
        data.extend_from_slice(body);
        let mut out = b"CcnK".to_vec();
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        out.extend_from_slice(&data);
        out
    }

    fn parameter_program(name: &str, values: &[f32]) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&(values.len() as u32).to_be_bytes());
        body.extend_from_slice(&name_bytes(name));
        for value in values {
            body.extend_from_slice(&value.to_be_bytes());
        }
        body
    }

    fn name_bytes(text: &str) -> [u8; NAME_BYTES] {
        let mut raw = [0_u8; NAME_BYTES];
        raw[..text.len().min(NAME_BYTES - 1)]
            .copy_from_slice(&text.as_bytes()[..text.len().min(NAME_BYTES - 1)]);
        raw
    }

    #[test]
    fn parses_a_parameter_program() {
        let bytes = file(b"FxCk", &parameter_program("Bright lead", &[0.0, 0.5, 1.0]));
        let preset = parse(&bytes).unwrap();
        assert_eq!(preset.fx_id, 0x1234_5678);
        assert_eq!(preset.fx_version, 7);
        match preset.body {
            Vst2PresetBody::ProgramParameters { name, parameters } => {
                assert_eq!(name, "Bright lead");
                assert_eq!(parameters, vec![0.0, 0.5, 1.0]);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn parses_a_chunk_program() {
        let mut body = Vec::new();
        body.extend_from_slice(&0_u32.to_be_bytes());
        body.extend_from_slice(&name_bytes("chunk program"));
        body.extend_from_slice(&5_u32.to_be_bytes());
        body.extend_from_slice(b"hello");
        let preset = parse(&file(b"FPCh", &body)).unwrap();
        assert_eq!(
            preset.body,
            Vst2PresetBody::ProgramChunk {
                name: "chunk program".into(),
                parameter_count: 0,
                chunk: b"hello".to_vec()
            }
        );
    }

    #[test]
    fn parses_parameter_and_chunk_banks() {
        let mut parameter_bank = Vec::new();
        parameter_bank.extend_from_slice(&2_u32.to_be_bytes());
        parameter_bank.extend_from_slice(&1_u32.to_be_bytes());
        parameter_bank.extend_from_slice(&[0_u8; 124]);
        parameter_bank.extend_from_slice(&file(b"FxCk", &parameter_program("One", &[0.25])));
        parameter_bank.extend_from_slice(&file(b"FxCk", &parameter_program("Two", &[0.75])));
        let preset = parse(&file(b"FxBk", &parameter_bank)).unwrap();
        assert_eq!(
            preset.body,
            Vst2PresetBody::Bank {
                current_program: 1,
                programs: vec![
                    Vst2BankProgram::Parameters {
                        name: "One".into(),
                        parameters: vec![0.25]
                    },
                    Vst2BankProgram::Parameters {
                        name: "Two".into(),
                        parameters: vec![0.75]
                    },
                ]
            }
        );

        let mut chunk_bank = Vec::new();
        chunk_bank.extend_from_slice(&0_u32.to_be_bytes());
        chunk_bank.extend_from_slice(&0_u32.to_be_bytes());
        chunk_bank.extend_from_slice(&[0_u8; 124]);
        chunk_bank.extend_from_slice(&3_u32.to_be_bytes());
        chunk_bank.extend_from_slice(b"abc");
        let preset = parse(&file(b"FBCh", &chunk_bank)).unwrap();
        assert_eq!(
            preset.body,
            Vst2PresetBody::BankChunk {
                current_program: 0,
                chunk: b"abc".to_vec(),
            }
        );
    }

    #[test]
    fn rejects_malformed_files_without_allocating_from_their_counts() {
        assert_eq!(parse(b""), Err(Vst2PresetError::TooShort));
        assert_eq!(parse(b"nope00000000"), Err(Vst2PresetError::BadMagic));
        let mut short = file(b"FxCk", &[1, 0, 0, 0]);
        short.truncate(short.len() - 1);
        assert_eq!(parse(&short), Err(Vst2PresetError::Truncated));

        // A huge declared parameter count must be refused before allocation.
        let mut body = Vec::new();
        body.extend_from_slice(&u32::MAX.to_be_bytes());
        body.extend_from_slice(&name_bytes("huge"));
        assert_eq!(
            parse(&file(b"FxCk", &body)),
            Err(Vst2PresetError::InvalidCount)
        );

        let mut zero_programs = vec![0_u8; 4];
        zero_programs.extend_from_slice(&0_u32.to_be_bytes());
        zero_programs.extend_from_slice(&[0_u8; 124]);
        assert_eq!(
            parse(&file(b"FxBk", &zero_programs)),
            Err(Vst2PresetError::InvalidCount)
        );

        assert_eq!(
            parse(&file(b"WAT!", &[])),
            Err(Vst2PresetError::UnexpectedKind)
        );
    }

    #[test]
    fn rejects_trailing_or_non_finite_data() {
        let mut program = parameter_program("One", &[0.5]);
        program.push(0);
        assert_eq!(
            parse(&file(b"FxCk", &program)),
            Err(Vst2PresetError::TrailingBytes)
        );
        let mut non_finite = parameter_program("Bad", &[0.5]);
        let nan_at = non_finite.len() - 4;
        non_finite[nan_at..].copy_from_slice(&f32::NAN.to_be_bytes());
        assert_eq!(
            parse(&file(b"FxCk", &non_finite)),
            Err(Vst2PresetError::InvalidCount)
        );
    }

    #[test]
    fn a_name_is_bounded_and_must_be_text() {
        let program = parameter_program("A very long preset name that overflows", &[]);
        let preset = parse(&file(b"FxCk", &program)).unwrap();
        match preset.body {
            Vst2PresetBody::ProgramParameters { name, .. } => {
                assert!(name.len() < NAME_BYTES);
            }
            other => panic!("{other:?}"),
        }
        let mut invalid = file(b"FxCk", &parameter_program("", &[]));
        invalid[28] = 0xFF;
        assert_eq!(parse(&invalid), Err(Vst2PresetError::InvalidName));
    }

    #[test]
    fn an_oversized_file_is_refused_before_parsing() {
        let bytes = vec![0_u8; MAX_VST2_PRESET_BYTES + 1];
        assert_eq!(parse(&bytes), Err(Vst2PresetError::TooLarge));
    }
}
