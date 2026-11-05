use crate::AudioError;

/// Decode caller-owned interleaved IEEE float32 bytes into the engine's
/// caller-owned planar block. The native capture adapter must copy a WASAPI
/// packet into `source` and release the COM buffer before calling this helper.
/// No allocation, endpoint access, or format conversion occurs here.
pub fn decode_interleaved_float32(
    source: &[u8],
    channels: usize,
    destination: &mut audiorouter_engine::AudioBlock,
) -> Result<(), AudioError> {
    if channels == 0 || destination.channels() != channels {
        return Err(AudioError::InvalidFrameSize);
    }
    let expected = channels
        .checked_mul(destination.frames())
        .and_then(|samples| samples.checked_mul(std::mem::size_of::<f32>()))
        .ok_or(AudioError::InvalidFrameSize)?;
    if source.len() != expected {
        return Err(AudioError::BufferTooSmall {
            required: expected,
            available: source.len(),
        });
    }
    for frame in 0..destination.frames() {
        for channel in 0..channels {
            let offset = (frame * channels + channel) * std::mem::size_of::<f32>();
            let sample = f32::from_ne_bytes([
                source[offset],
                source[offset + 1],
                source[offset + 2],
                source[offset + 3],
            ]);
            destination.channel_mut(channel).expect("validated channel")[frame] =
                if sample.is_finite() { sample } else { 0.0 };
        }
    }
    Ok(())
}

/// Encode the engine's planar float32 block into caller-owned interleaved
/// IEEE float32 bytes for a WASAPI render packet. The destination must already
/// be sized for exactly one block; this helper never allocates or touches an
/// endpoint.
pub fn encode_interleaved_float32(
    source: &audiorouter_engine::AudioBlock,
    destination: &mut [u8],
) -> Result<(), AudioError> {
    let expected = source
        .channels()
        .checked_mul(source.frames())
        .and_then(|samples| samples.checked_mul(std::mem::size_of::<f32>()))
        .ok_or(AudioError::InvalidFrameSize)?;
    if destination.len() != expected {
        return Err(AudioError::BufferTooSmall {
            required: expected,
            available: destination.len(),
        });
    }
    for frame in 0..source.frames() {
        for channel in 0..source.channels() {
            let sample = source.channel(channel).expect("validated channel")[frame];
            let offset = (frame * source.channels() + channel) * std::mem::size_of::<f32>();
            destination[offset..offset + 4].copy_from_slice(&sample.to_ne_bytes());
        }
    }
    Ok(())
}
