//! Capture only the guest speaker mix; never open a render or cable stream.
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use audiorouter_windows_audio::{
    enumerate_active_endpoint_display_info, enumerate_active_endpoints, AudioServiceThreadGuard,
    EndpointDirection, EndpointDisplayInfo, EndpointInfo, SharedCapture,
};
use serde_json::{json, Value};

use super::{write_json, Recording};

const NAME: &str = "Speakers (High Definition Audio Device)";
const RATE: usize = 44_100;
const SECONDS: u64 = 30;

fn select_speaker(
    displays: &[EndpointDisplayInfo],
    formats: &[EndpointInfo],
) -> Result<(EndpointInfo, u16), String> {
    let mut found = displays
        .iter()
        .filter(|e| e.name == NAME && e.direction == EndpointDirection::Render);
    let display = found.next().ok_or("exact active guest Speakers missing")?;
    if found.next().is_some() {
        return Err("duplicate active guest Speakers; no fallback allowed".into());
    }
    let format = formats
        .iter()
        .find(|e| e.id == display.id && e.direction == EndpointDirection::Render)
        .ok_or("guest speaker mix format missing")?;
    let extensible = format.subformat_guid.trim_matches(['{', '}']);
    let tag = match (format.bits_per_sample, format.format_tag) {
        (16, 1) => 1,
        (32, 3) => 3,
        (16, 0xfffe) if extensible.eq_ignore_ascii_case("00000001-0000-0010-8000-00aa00389b71") => {
            1
        }
        (32, 0xfffe) if extensible.eq_ignore_ascii_case("00000003-0000-0010-8000-00aa00389b71") => {
            3
        }
        _ => return Err(format!("unsupported speaker sample encoding: {format:?}")),
    };
    if format.channels != 2 || format.sample_rate_hz != RATE as u32 {
        return Err(format!("require stereo native 44100 Hz: {format:?}"));
    }
    Ok((format.clone(), tag))
}

pub fn record_speaker(directory: &Path) -> Result<(), String> {
    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = Arc::clone(&stop);
    let (ready_tx, ready_rx) = mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || pump(worker_stop, ready_tx));
    let control: Result<(), String> = (|| {
        let metadata = ready_rx
            .recv_timeout(Duration::from_secs(10))
            .map_err(|e| format!("speaker readiness: {e}"))?;
        write_json(&directory.join("ready.pending"), &metadata)?;
        std::fs::rename(
            directory.join("ready.pending"),
            directory.join("ready.json"),
        )
        .map_err(|e| e.to_string())?;
        let deadline = Instant::now() + Duration::from_secs(40);
        while !worker.is_finished() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(50));
        }
        if !worker.is_finished() {
            return Err("speaker controller deadline reached".into());
        }
        Ok(())
    })();
    stop.store(true, Ordering::Release);
    let (recording, format, tag) = worker
        .join()
        .map_err(|_| "speaker worker panicked".to_string())??;
    // The worker has stopped/reset the endpoint before any file serialization.
    write_wav(
        &directory.join("speakers.wav"),
        &recording.bytes,
        &format,
        tag,
    )?;
    write_json(&directory.join("recording.json"), &recording.report)?;
    control?;
    recording.error.map_or(Ok(()), Err)
}

fn pump(
    stop: Arc<AtomicBool>,
    ready: mpsc::SyncSender<Value>,
) -> Result<(Recording, EndpointInfo, u16), String> {
    let (_guard, capabilities) = AudioServiceThreadGuard::enter();
    if !capabilities.com_multithreaded
        || !capabilities.mmcss_pro_audio
        || !capabilities.one_millisecond_timer
    {
        return Err(format!("required scheduling unavailable: {capabilities:?}"));
    }
    let displays = enumerate_active_endpoint_display_info().map_err(|e| e.to_string())?;
    let formats = enumerate_active_endpoints().map_err(|e| e.to_string())?;
    let (format, tag) = select_speaker(&displays, &formats)?;
    let stride = format.bytes_per_frame().map_err(|e| e.to_string())?;
    let mut capture = SharedCapture::open_loopback(&format.id).map_err(|e| e.to_string())?;
    if !enumerate_active_endpoints()
        .map_err(|e| e.to_string())?
        .contains(&format)
    {
        return Err("speaker mix changed during open; no stream started".into());
    }
    // All data/metadata storage and ready serialization are prepared before Start.
    let mut bytes = Vec::with_capacity(RATE * 35 * stride);
    let mut packet = vec![0u8; RATE * stride];
    let mut packets = Vec::with_capacity(60_000);
    let metadata = json!({"endpointId":format.id,"endpointName":NAME,"rate":RATE,
        "channels":2,"bits":format.bits_per_sample,"wavTag":tag,
        "seconds":SECONDS,"qualification":false,"mode":"guest-speaker-loopback"});
    capture.start().map_err(|e| e.to_string())?;
    ready.try_send(metadata).map_err(|e| e.to_string())?;
    let start = Instant::now();
    let mut previous = start;
    let mut max_gap_us = 0;
    let outcome: Result<(), String> = (|| {
        while !stop.load(Ordering::Acquire) && start.elapsed() < Duration::from_secs(SECONDS) {
            let now = Instant::now();
            let gap = now.duration_since(previous).as_micros();
            max_gap_us = max_gap_us.max(gap);
            previous = now;
            for _ in 0..32 {
                let Some((info, count)) = capture
                    .next_packet_into(&mut packet, stride)
                    .map_err(|e| e.to_string())?
                else {
                    break;
                };
                if count != info.frames as usize * stride
                    || bytes.len() + count > bytes.capacity()
                    || packets.len() == packets.capacity()
                {
                    return Err("speaker capture storage exhausted or packet shape changed".into());
                }
                packets.push((bytes.len() / stride, info, start.elapsed().as_micros(), gap));
                bytes.extend_from_slice(&packet[..count]);
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        if stop.load(Ordering::Acquire) {
            return Err("speaker capture cancelled before its deadline".into());
        }
        if bytes.is_empty() {
            return Err("no speaker samples captured; preserve readiness/playback evidence".into());
        }
        Ok(())
    })();
    let mut error = outcome.err();
    if let Err(e) = capture.stop() {
        error.get_or_insert_with(|| e.to_string());
    }
    let packet_report: Vec<_> = packets
        .iter()
        .map(|(offset, p, elapsed, gap)| {
            json!({"fileFrame":offset,"frames":p.frames,"flags":p.flags,
            "devicePosition":p.device_position,"qpc100ns":p.qpc_position,
            "elapsedUs":elapsed,"pumpGapUs":gap})
        })
        .collect();
    Ok((
        Recording {
            report: json!({"mode":"guest-speaker-loopback","qualification":false,
            "endpointId":format.id,"rate":RATE,"channels":2,"bits":format.bits_per_sample,
            "wavTag":tag,"elapsedSeconds":start.elapsed().as_secs_f64(),
            "frames":bytes.len()/stride,"maxPumpGapUs":max_gap_us,"packets":packet_report,
            "error":error}),
            bytes,
            error,
        },
        format,
        tag,
    ))
}

fn wav_header(data: usize, bits: u16, tag: u16) -> Result<[u8; 44], String> {
    if !matches!((bits, tag), (16, 1) | (32, 3)) {
        return Err("unsupported speaker WAV encoding".into());
    }
    let stride = usize::from(bits / 8) * 2;
    if data > RATE * 35 * stride || data % stride != 0 {
        return Err("speaker WAV storage/stride bound".into());
    }
    let mut header = [0u8; 44];
    header[..4].copy_from_slice(b"RIFF");
    header[4..8].copy_from_slice(&(data as u32 + 36).to_le_bytes());
    header[8..16].copy_from_slice(b"WAVEfmt ");
    header[16..20].copy_from_slice(&16u32.to_le_bytes());
    header[20..22].copy_from_slice(&tag.to_le_bytes());
    header[22..24].copy_from_slice(&2u16.to_le_bytes());
    header[24..28].copy_from_slice(&(RATE as u32).to_le_bytes());
    header[28..32].copy_from_slice(&((RATE * stride) as u32).to_le_bytes());
    header[32..34].copy_from_slice(&(stride as u16).to_le_bytes());
    header[34..36].copy_from_slice(&bits.to_le_bytes());
    header[36..40].copy_from_slice(b"data");
    header[40..44].copy_from_slice(&(data as u32).to_le_bytes());
    Ok(header)
}

fn write_wav(path: &Path, bytes: &[u8], format: &EndpointInfo, tag: u16) -> Result<(), String> {
    let header = wav_header(bytes.len(), format.bits_per_sample, tag)?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.write_all(&header)
        .and_then(|_| file.write_all(bytes))
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (EndpointDisplayInfo, EndpointInfo) {
        (
            EndpointDisplayInfo {
                id: "speaker-id".into(),
                name: NAME.into(),
                direction: EndpointDirection::Render,
                device_description: String::new(),
                driver_inf_section: String::new(),
            },
            EndpointInfo {
                id: "speaker-id".into(),
                direction: EndpointDirection::Render,
                default_period_100ns: 100000,
                minimum_period_100ns: 100000,
                sample_rate_hz: RATE as u32,
                channels: 2,
                bits_per_sample: 32,
                format_tag: 3,
                channel_mask: 3,
                subformat_guid: String::new(),
            },
        )
    }

    #[test]
    fn speaker_selection_accepts_only_exact_native_formats() {
        let (display, format) = fixture();
        for (bits, tag, guid, expected) in [
            (32, 3, "", 3),
            (16, 1, "", 1),
            (32, 0xfffe, "{00000003-0000-0010-8000-00AA00389B71}", 3),
            (16, 0xfffe, "{00000001-0000-0010-8000-00aa00389b71}", 1),
        ] {
            let mut candidate = format.clone();
            candidate.bits_per_sample = bits;
            candidate.format_tag = tag;
            candidate.subformat_guid = guid.into();
            assert_eq!(
                select_speaker(std::slice::from_ref(&display), &[candidate])
                    .unwrap()
                    .1,
                expected
            );
        }
        for (rate, channels, bits, tag, guid) in [
            (48000, 2, 32, 3, ""),
            (44100, 1, 32, 3, ""),
            (44100, 2, 32, 1, ""),
            (44100, 2, 24, 1, ""),
            (44100, 2, 32, 0xfffe, "00000003-wrong"),
        ] {
            let mut candidate = format.clone();
            candidate.sample_rate_hz = rate;
            candidate.channels = channels;
            candidate.bits_per_sample = bits;
            candidate.format_tag = tag;
            candidate.subformat_guid = guid.into();
            assert!(select_speaker(std::slice::from_ref(&display), &[candidate]).is_err());
        }
        assert!(select_speaker(&[], std::slice::from_ref(&format)).is_err());
        assert!(select_speaker(
            &[display.clone(), display.clone()],
            std::slice::from_ref(&format)
        )
        .is_err());
        let mut wrong = display;
        wrong.name = "AudioRouter Cable B Input (AudioRouter Virtual Cable)".into();
        assert!(select_speaker(&[wrong], &[format]).is_err());
    }

    #[test]
    fn speaker_wav_preserves_native_rate_and_encoding() {
        for (bits, tag, stride) in [(16, 1, 4usize), (32, 3, 8)] {
            let data = RATE * stride;
            let header = wav_header(data, bits, tag).unwrap();
            let u16_at = |o| u16::from_le_bytes(header[o..o + 2].try_into().unwrap());
            let u32_at = |o| u32::from_le_bytes(header[o..o + 4].try_into().unwrap());
            assert_eq!(&header[8..16], b"WAVEfmt ");
            assert_eq!((u16_at(20), u16_at(22), u16_at(34)), (tag, 2, bits));
            assert_eq!(
                (u32_at(24), u32_at(28), u32_at(40)),
                (44100, (RATE * stride) as u32, data as u32)
            );
            assert_eq!(u32_at(4), data as u32 + 36);
            assert!(wav_header(data + 1, bits, tag).is_err());
            assert!(wav_header(RATE * 36 * stride, bits, tag).is_err());
        }
        assert!(wav_header(8, 32, 1).is_err());
    }
}
