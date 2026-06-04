use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};

use super::pipewire_linux;
use super::{TARGET_CHANNELS, TARGET_SAMPLE_RATE};

/// Microphone capture on Linux through PipeWire/PulseAudio.
/// Captures the default input source as PCM s16le 16kHz mono.
pub struct MicCapture {
    is_capturing: Arc<AtomicBool>,
    child: Arc<Mutex<Option<Child>>>,
}

impl MicCapture {
    pub fn new() -> Self {
        Self {
            is_capturing: Arc::new(AtomicBool::new(false)),
            child: Arc::new(Mutex::new(None)),
        }
    }

    pub fn start(&mut self) -> Result<mpsc::Receiver<Vec<u8>>, String> {
        if self.is_capturing.load(Ordering::SeqCst) {
            return Err("Already capturing".to_string());
        }

        self.cleanup_previous_child();

        let target = pipewire_linux::default_input_target()?;
        println!("[LinuxMic] Capturing input target: {}", target);

        let mut child = Command::new("pw-record")
            .arg("--target")
            .arg(&target)
            .arg("--rate")
            .arg(TARGET_SAMPLE_RATE.to_string())
            .arg("--channels")
            .arg(TARGET_CHANNELS.to_string())
            .arg("--format")
            .arg("s16")
            .arg("--raw")
            .arg("-")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| {
                format!(
                    "Failed to start pw-record. Install pipewire-utils and make sure PipeWire is running: {}",
                    e
                )
            })?;

        let stdout = child
            .stdout
            .take()
            .ok_or("Failed to open pw-record stdout".to_string())?;

        {
            let mut slot = self.child.lock().map_err(|e| e.to_string())?;
            *slot = Some(child);
        }

        let (sender, receiver) = mpsc::channel::<Vec<u8>>();
        self.is_capturing.store(true, Ordering::SeqCst);

        let is_capturing = self.is_capturing.clone();
        std::thread::spawn(move || {
            let mut stdout = stdout;
            let mut buffer = [0u8; 4096];
            let mut pending = Vec::new();

            loop {
                match stdout.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(n) => {
                        pending.extend_from_slice(&buffer[..n]);
                        let output = drain_s16le_samples(&mut pending);
                        if !output.is_empty() && sender.send(output).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("[LinuxMic] Failed reading pw-record stdout: {}", e);
                        break;
                    }
                }
            }

            is_capturing.store(false, Ordering::SeqCst);
        });

        Ok(receiver)
    }

    pub fn stop(&mut self) {
        self.is_capturing.store(false, Ordering::SeqCst);

        if let Ok(mut slot) = self.child.lock() {
            if let Some(mut child) = slot.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }

    fn cleanup_previous_child(&self) {
        if let Ok(mut slot) = self.child.lock() {
            if let Some(mut child) = slot.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

impl Default for MicCapture {
    fn default() -> Self {
        Self::new()
    }
}

fn drain_s16le_samples(pending: &mut Vec<u8>) -> Vec<u8> {
    let process_len = pending.len() / 2 * 2;
    if process_len == 0 {
        return Vec::new();
    }

    let output = pending[..process_len].to_vec();
    pending.drain(..process_len);
    output
}
