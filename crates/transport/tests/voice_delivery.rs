//! Explicit local-only reuse of an approved voice sample.
use audiorouter_engine::{AudioBlock, AudioTap};
use audiorouter_recording::{Mp3Writer, WavFormat, WavWriter};
use audiorouter_windows_audio::{decode_network_packet, NetworkSender};

#[test]
#[ignore = "private voice sample and localhost UDP; explicit opt-in"]
fn private_voice_recording_and_network_preserve_audio() {
    let path = std::env::var("AUDIOROUTER_VOICE_SAMPLE").expect("approved voice sample");
    let bytes = std::fs::read(path).unwrap();
    assert_eq!(&bytes[36..40], b"data");
    let samples = bytes[44..]
        .chunks_exact(2)
        .map(|s| f32::from(i16::from_le_bytes(s.try_into().unwrap())) / 32768.0)
        .collect::<Vec<_>>();
    let folder =
        std::env::temp_dir().join(format!("audiorouter-voice-delivery-{}", std::process::id()));
    std::fs::create_dir(&folder).unwrap();
    for format in [WavFormat::Pcm16, WavFormat::Pcm24, WavFormat::Float32] {
        let mut writer =
            WavWriter::new(std::io::Cursor::new(Vec::new()), format, 1, 48000, false).unwrap();
        for chunk in samples.chunks(128) {
            writer.write_interleaved(chunk).unwrap();
        }
        assert_eq!(writer.frames(), samples.len() as u64);
        let path = folder.join(format!("{format:?}.wav"));
        std::fs::write(&path, writer.finish().unwrap().into_inner()).unwrap();
        let decoded = audiorouter_engine::decode_audio_file(&path, 48000).unwrap();
        assert_eq!(decoded.samples.len(), samples.len());
        let max_error = decoded
            .samples
            .iter()
            .zip(&samples)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0_f32, f32::max);
        assert!(max_error < 0.00004, "{format:?}: {max_error}");
    }
    let mut mp3 = Mp3Writer::new(Vec::new(), 1, 48000).unwrap();
    mp3.write_interleaved(&samples).unwrap();
    let path = folder.join("voice.mp3");
    std::fs::write(&path, mp3.finish().unwrap().0).unwrap();
    let decoded = audiorouter_engine::decode_audio_file(&path, 48000).unwrap();
    assert!(decoded.samples.iter().all(|s| s.is_finite()));
    assert!(decoded.samples.len() >= samples.len());
    let energy = |s: &[f32]| s.iter().map(|s| f64::from(*s).powi(2)).sum::<f64>();
    assert!(
        (energy(&decoded.samples) / energy(&samples) - 1.0).abs() < 0.15,
        "MP3 preserves speech energy"
    );
    // Actual production sender/tap, with a localhost socket as reference receiver.
    let socket = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    socket
        .set_read_timeout(Some(std::time::Duration::from_secs(2)))
        .unwrap();
    let sender = NetworkSender::start(socket.local_addr().unwrap()).unwrap();
    let tap = sender.tap();
    let mut packet = [0_u8; 4096];
    let mut received = Vec::new();
    for (index, chunk) in samples.chunks(128).enumerate() {
        let mut block = AudioBlock::new(1, chunk.len()).unwrap();
        block.channel_mut(0).unwrap().copy_from_slice(chunk);
        tap.on_processed_block(index as u64 * 128, &block);
        let length = socket.recv(&mut packet).unwrap();
        let (header, payload) = decode_network_packet(&packet[..length]).unwrap();
        assert_eq!(header.sequence, index as u32);
        assert_eq!(header.channels, 1);
        received.extend(
            payload
                .chunks_exact(4)
                .map(|s| f32::from_le_bytes(s.try_into().unwrap())),
        );
    }
    assert_eq!(received, samples);
    assert_eq!(sender.stats().dropped_packets, 0);
    // Production receiver and its paced capture API, local-only.
    use audiorouter_windows_audio::{AudioCaptureSource, NetworkReceiver};
    let probe = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    let port = probe.local_addr().unwrap().port();
    drop(probe);
    let receiver = NetworkReceiver::start("127.0.0.1".parse().unwrap(), port, 20.0).unwrap();
    let sender = NetworkSender::start((std::net::Ipv4Addr::LOCALHOST, port).into()).unwrap();
    let tap = sender.tap();
    let start = std::time::Instant::now();
    let mut offset = 0;
    let mut played = Vec::new();
    let mut buffer = [0_u8; 512];
    while start.elapsed() < std::time::Duration::from_millis(5100) {
        let due = (start.elapsed().as_secs_f64() * 48000.0) as usize;
        while offset + 128 <= samples.len() && offset <= due {
            let mut block = AudioBlock::new(1, 128).unwrap();
            block
                .channel_mut(0)
                .unwrap()
                .copy_from_slice(&samples[offset..offset + 128]);
            tap.on_processed_block(offset as u64, &block);
            offset += 128;
        }
        while let Some((_, length)) = receiver.next_packet_into(&mut buffer, 4).unwrap() {
            played.extend(
                buffer[..length]
                    .chunks_exact(4)
                    .map(|s| f32::from_le_bytes(s.try_into().unwrap())),
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let stats = receiver.stats();
    assert_eq!(stats.lost_packets, 0);
    assert_eq!(stats.rejected_datagrams, 0);
    assert_eq!(stats.overflow_packets, 0);
    let ratio = energy(&played) / energy(&samples);
    assert!(
        (ratio - 1.0).abs() < 0.15,
        "receiver preserves voice energy: {ratio}"
    );
    // Compensate its start buffer before comparing the actual speech waveform.
    let reference = &samples[48000..96000];
    let best = (0..4800)
        .step_by(16)
        .filter(|delay| played.len() > 96000 + delay)
        .map(|delay| {
            let output = &played[48000 + delay..96000 + delay];
            output
                .iter()
                .zip(reference)
                .map(|(a, b)| f64::from(a - b).powi(2))
                .sum::<f64>()
                / energy(reference)
        })
        .fold(f64::INFINITY, f64::min);
    assert!(
        best < 0.15,
        "receiver speech waveform correspondence: {best}"
    );
    eprintln!(
        "Voice recording and localhost sender: passed; private artifacts in {}",
        folder.display()
    );
}
