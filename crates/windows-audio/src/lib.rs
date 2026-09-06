//! Windows audio device metadata boundary for M02.
//!
//! This first adapter slice is intentionally read-only: it enumerates active
//! endpoints and reports the endpoint-owned shared-mode format and periods.
//! It does not initialize, start, or read an audio stream.

use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EndpointDirection {
    Capture,
    Render,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EndpointInfo {
    pub id: String,
    pub direction: EndpointDirection,
    pub default_period_100ns: i64,
    pub minimum_period_100ns: i64,
    pub sample_rate_hz: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
    pub format_tag: u16,
}

#[derive(Debug)]
pub enum AudioError {
    Windows(windows::core::Error),
    InvalidUtf16,
}

impl fmt::Display for AudioError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Windows(error) => write!(formatter, "Windows audio error: {error}"),
            Self::InvalidUtf16 => formatter.write_str("endpoint ID was not valid UTF-16"),
        }
    }
}

impl std::error::Error for AudioError {}

impl From<windows::core::Error> for AudioError {
    fn from(error: windows::core::Error) -> Self {
        Self::Windows(error)
    }
}

/// Enumerate active capture and render endpoints without opening streams.
pub fn enumerate_active_endpoints() -> Result<Vec<EndpointInfo>, AudioError> {
    unsafe {
        let initialized = windows::Win32::System::Com::CoInitializeEx(
            None,
            windows::Win32::System::Com::COINIT_MULTITHREADED,
        );
        initialized.ok()?;
        let result = enumerate_after_com_init();
        windows::Win32::System::Com::CoUninitialize();
        result
    }
}

unsafe fn enumerate_after_com_init() -> Result<Vec<EndpointInfo>, AudioError> {
    use windows::Win32::Media::Audio::{
        eCapture, eRender, IAudioClient, IMMDeviceEnumerator, MMDeviceEnumerator,
        DEVICE_STATE_ACTIVE,
    };
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};

    let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
    let mut endpoints = Vec::new();
    for (direction, flow) in [
        (EndpointDirection::Capture, eCapture),
        (EndpointDirection::Render, eRender),
    ] {
        let devices = enumerator.EnumAudioEndpoints(flow, DEVICE_STATE_ACTIVE)?;
        let count = devices.GetCount()?;
        for index in 0..count {
            let device = devices.Item(index)?;
            let id = device
                .GetId()
                .map_err(AudioError::Windows)?
                .to_string()
                .map_err(|_| AudioError::InvalidUtf16)?;
            let client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;
            let mut default_period = 0;
            let mut minimum_period = 0;
            client.GetDevicePeriod(Some(&mut default_period), Some(&mut minimum_period))?;
            let format = client.GetMixFormat()?;
            let format_value = *format;
            endpoints.push(EndpointInfo {
                id,
                direction,
                default_period_100ns: default_period,
                minimum_period_100ns: minimum_period,
                sample_rate_hz: format_value.nSamplesPerSec,
                channels: format_value.nChannels,
                bits_per_sample: format_value.wBitsPerSample,
                format_tag: format_value.wFormatTag,
            });
            windows::Win32::System::Com::CoTaskMemFree(Some(format.cast()));
        }
    }
    Ok(endpoints)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_metadata_shape_is_stable() {
        let info = EndpointInfo {
            id: "endpoint".into(),
            direction: EndpointDirection::Capture,
            default_period_100ns: 100_000,
            minimum_period_100ns: 20_000,
            sample_rate_hz: 48_000,
            channels: 2,
            bits_per_sample: 32,
            format_tag: 0xfffe,
        };
        assert_eq!(info.direction, EndpointDirection::Capture);
        assert!(info.minimum_period_100ns <= info.default_period_100ns);
    }

    #[cfg(windows)]
    #[test]
    fn active_endpoint_enumeration_is_read_only() {
        let endpoints = enumerate_active_endpoints().unwrap();
        assert!(!endpoints.is_empty());
        assert!(endpoints.iter().all(|endpoint| !endpoint.id.is_empty()));
        assert!(endpoints.iter().all(|endpoint| endpoint.sample_rate_hz > 0));
    }
}
