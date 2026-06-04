pub mod resampler;

#[cfg(not(target_os = "linux"))]
pub mod microphone;

#[cfg(target_os = "linux")]
pub mod microphone_linux;

#[cfg(target_os = "linux")]
mod pipewire_linux;

#[cfg(target_os = "macos")]
pub mod system_audio;

#[cfg(target_os = "linux")]
pub mod system_audio_linux;

#[cfg(target_os = "windows")]
pub mod wasapi;

// Re-export SystemAudioCapture from the correct platform module
#[cfg(not(target_os = "linux"))]
pub use microphone::MicCapture;

#[cfg(target_os = "linux")]
pub use microphone_linux::MicCapture;

#[cfg(target_os = "macos")]
pub use system_audio::SystemAudioCapture;

#[cfg(target_os = "linux")]
pub use system_audio_linux::SystemAudioCapture;

#[cfg(target_os = "windows")]
pub use wasapi::SystemAudioCapture;

/// Target audio format for Soniox: PCM s16le, 16kHz, mono
pub const TARGET_SAMPLE_RATE: u32 = 16000;
pub const TARGET_CHANNELS: u16 = 1;
