use crate::audio::{MicCapture, SystemAudioCapture};
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{ipc::Channel, State};

const MIX_SAMPLE_RATE: usize = 16000;
const MIX_BYTES_PER_SAMPLE: usize = 2;
const MIX_INTERVAL_MS: usize = 20;
const MIX_CHUNK_BYTES: usize = MIX_SAMPLE_RATE * MIX_BYTES_PER_SAMPLE * MIX_INTERVAL_MS / 1000;

/// State for tracking active audio captures
pub struct AudioState {
    pub system_audio: Mutex<SystemAudioCapture>,
    pub microphone: Mutex<MicCapture>,
    pub active_receiver: Mutex<Option<AudioForwarder>>,
}

/// Forwards audio from a receiver to a Tauri IPC channel
pub struct AudioForwarder {
    /// Handle to signal stop
    stop_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl AudioForwarder {
    fn stop(&self) {
        self.stop_flag
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

#[derive(Serialize, Clone)]
pub struct PermissionStatus {
    pub screen_recording: String,
    pub microphone: String,
}

/// Start audio capture and forward data to the frontend via IPC channel
#[tauri::command]
pub fn start_capture(
    source: String,
    channel: Channel<Vec<u8>>,
    state: State<'_, AudioState>,
) -> Result<(), String> {
    // Stop any existing capture first
    stop_capture_inner(&state);

    let receiver: mpsc::Receiver<Vec<u8>> = match source.as_str() {
        "system" => {
            let sys = state.system_audio.lock().map_err(|e| e.to_string())?;
            sys.start()?
        }
        "microphone" => {
            let mut mic = state.microphone.lock().map_err(|e| e.to_string())?;
            mic.start()?
        }
        "both" => {
            let sys = state.system_audio.lock().map_err(|e| e.to_string())?;
            let sys_rx = sys.start()?;
            let mut mic = state.microphone.lock().map_err(|e| e.to_string())?;
            let mic_rx = match mic.start() {
                Ok(receiver) => receiver,
                Err(err) => {
                    sys.stop();
                    return Err(err);
                }
            };

            mix_audio_receivers(sys_rx, mic_rx)
        }
        _ => return Err(format!("Unknown source: {}", source)),
    };

    // Spawn a thread to forward audio data from receiver to IPC channel
    let stop_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let stop_flag_clone = stop_flag.clone();

    std::thread::spawn(move || {
        let mut buffer: Vec<u8> = Vec::with_capacity(32000); // ~1 sec at 16kHz s16le
        let batch_interval = std::time::Duration::from_millis(200);
        let mut last_flush = std::time::Instant::now();

        loop {
            if stop_flag_clone.load(std::sync::atomic::Ordering::SeqCst) {
                // Flush remaining buffer before exit
                if !buffer.is_empty() {
                    let _ = channel.send(buffer.clone());
                }
                break;
            }

            match receiver.recv_timeout(std::time::Duration::from_millis(10)) {
                Ok(data) => {
                    buffer.extend_from_slice(&data);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    if !buffer.is_empty() {
                        let _ = channel.send(buffer.clone());
                    }
                    break;
                }
            }

            // Flush buffer every 200ms
            if last_flush.elapsed() >= batch_interval && !buffer.is_empty() {
                if let Err(_e) = channel.send(buffer.clone()) {
                    break; // Channel closed
                }
                buffer.clear();
                last_flush = std::time::Instant::now();
            }
        }
    });

    // Store the forwarder so we can stop it later
    let forwarder = AudioForwarder { stop_flag };
    let mut active = state.active_receiver.lock().map_err(|e| e.to_string())?;
    *active = Some(forwarder);

    Ok(())
}

struct MixInput {
    buffer: Mutex<VecDeque<u8>>,
    alive: AtomicBool,
}

impl MixInput {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            buffer: Mutex::new(VecDeque::new()),
            alive: AtomicBool::new(true),
        })
    }
}

fn mix_audio_receivers(
    system_rx: mpsc::Receiver<Vec<u8>>,
    mic_rx: mpsc::Receiver<Vec<u8>>,
) -> mpsc::Receiver<Vec<u8>> {
    let system = MixInput::new();
    let mic = MixInput::new();

    spawn_mix_input_reader(system_rx, system.clone());
    spawn_mix_input_reader(mic_rx, mic.clone());

    let (mixed_tx, mixed_rx) = mpsc::channel::<Vec<u8>>();

    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(MIX_INTERVAL_MS as u64));

        let system_chunk = drain_input_chunk(&system, MIX_CHUNK_BYTES);
        let mic_chunk = drain_input_chunk(&mic, MIX_CHUNK_BYTES);

        if system_chunk.is_empty() && mic_chunk.is_empty() {
            let system_alive = system.alive.load(Ordering::SeqCst);
            let mic_alive = mic.alive.load(Ordering::SeqCst);
            if !system_alive && !mic_alive {
                break;
            }
            continue;
        }

        let mixed = mix_s16le_mono(&system_chunk, &mic_chunk);
        if !mixed.is_empty() && mixed_tx.send(mixed).is_err() {
            break;
        }
    });

    mixed_rx
}

fn spawn_mix_input_reader(receiver: mpsc::Receiver<Vec<u8>>, input: Arc<MixInput>) {
    std::thread::spawn(move || {
        while let Ok(mut data) = receiver.recv() {
            if data.len() % MIX_BYTES_PER_SAMPLE != 0 {
                data.truncate(data.len() / MIX_BYTES_PER_SAMPLE * MIX_BYTES_PER_SAMPLE);
            }
            if data.is_empty() {
                continue;
            }

            if let Ok(mut buffer) = input.buffer.lock() {
                buffer.extend(data);
            } else {
                break;
            }
        }

        input.alive.store(false, Ordering::SeqCst);
    });
}

fn drain_input_chunk(input: &MixInput, max_bytes: usize) -> Vec<u8> {
    let Ok(mut buffer) = input.buffer.lock() else {
        return Vec::new();
    };

    let drain_len = buffer.len().min(max_bytes) / MIX_BYTES_PER_SAMPLE * MIX_BYTES_PER_SAMPLE;
    buffer.drain(..drain_len).collect()
}

fn mix_s16le_mono(a: &[u8], b: &[u8]) -> Vec<u8> {
    let output_len = a.len().max(b.len()) / MIX_BYTES_PER_SAMPLE * MIX_BYTES_PER_SAMPLE;
    let mut mixed = Vec::with_capacity(output_len);

    for offset in (0..output_len).step_by(MIX_BYTES_PER_SAMPLE) {
        let sample_a = read_s16le_sample(a, offset).unwrap_or(0);
        let sample_b = read_s16le_sample(b, offset).unwrap_or(0);
        let sample = (sample_a as i32 + sample_b as i32).clamp(i16::MIN as i32, i16::MAX as i32);
        mixed.extend_from_slice(&(sample as i16).to_le_bytes());
    }

    mixed
}

fn read_s16le_sample(data: &[u8], offset: usize) -> Option<i16> {
    if offset + 1 >= data.len() {
        return None;
    }

    Some(i16::from_le_bytes([data[offset], data[offset + 1]]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn samples_to_bytes(samples: &[i16]) -> Vec<u8> {
        samples
            .iter()
            .flat_map(|sample| sample.to_le_bytes())
            .collect()
    }

    fn bytes_to_samples(bytes: &[u8]) -> Vec<i16> {
        bytes
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
            .collect()
    }

    #[test]
    fn mix_s16le_mono_passes_through_single_source() {
        let a = samples_to_bytes(&[100, -200, 300]);

        assert_eq!(bytes_to_samples(&mix_s16le_mono(&a, &[])), [100, -200, 300]);
    }

    #[test]
    fn mix_s16le_mono_sums_sources_with_clipping() {
        let a = samples_to_bytes(&[1000, 30_000, -30_000]);
        let b = samples_to_bytes(&[2000, 30_000, -30_000]);

        assert_eq!(
            bytes_to_samples(&mix_s16le_mono(&a, &b)),
            [3000, i16::MAX, i16::MIN]
        );
    }
}

/// Stop audio capture
#[tauri::command]
pub fn stop_capture(state: State<'_, AudioState>) -> Result<(), String> {
    stop_capture_inner(&state);
    Ok(())
}

fn stop_capture_inner(state: &AudioState) {
    // Stop the forwarder
    if let Ok(mut active) = state.active_receiver.lock() {
        if let Some(forwarder) = active.take() {
            forwarder.stop();
        }
    }

    // Stop system audio
    if let Ok(sys) = state.system_audio.lock() {
        sys.stop();
    }

    // Stop microphone
    if let Ok(mut mic) = state.microphone.lock() {
        mic.stop();
    }
}

/// Check audio capture permissions
#[tauri::command]
pub fn check_permissions() -> PermissionStatus {
    // Note: Actual permission checking on macOS requires Objective-C interop
    // For now, we return "unknown" and permissions will be prompted on first use
    PermissionStatus {
        screen_recording: "unknown".to_string(),
        microphone: "unknown".to_string(),
    }
}
