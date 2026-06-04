use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};

use super::pipewire_linux;
use super::TARGET_SAMPLE_RATE;

/// System audio capture on Linux through PipeWire's monitor source.
///
/// Modern PipeWire desktops expose each output sink as a monitor input
/// named `<sink>.monitor`. `pw-record` can capture that monitor and emit raw PCM
/// directly. On some devices, targeting the monitor by name with mono channel
/// negotiation links to the headset microphone instead, so this backend targets
/// the monitor node serial and captures stereo before downmixing to mono.
pub struct SystemAudioCapture {
    is_capturing: Arc<AtomicBool>,
    child: Arc<Mutex<Option<Child>>>,
}

impl SystemAudioCapture {
    pub fn new() -> Self {
        Self {
            is_capturing: Arc::new(AtomicBool::new(false)),
            child: Arc::new(Mutex::new(None)),
        }
    }

    /// Start capturing the current default output monitor.
    /// Returns a receiver that yields PCM s16le 16kHz mono audio chunks.
    pub fn start(&self) -> Result<mpsc::Receiver<Vec<u8>>, String> {
        if self.is_capturing.load(Ordering::SeqCst) {
            return Err("Already capturing".to_string());
        }

        self.cleanup_previous_child();

        let target = pipewire_linux::default_monitor_target()?;
        println!("[LinuxSystemAudio] Capturing monitor target: {}", target);

        let mut child = Command::new("pw-record")
            .arg("--target")
            .arg(&target)
            .arg("--rate")
            .arg(TARGET_SAMPLE_RATE.to_string())
            .arg("--channels")
            .arg("2")
            .arg("--format")
            .arg("s16")
            .arg("--raw")
            .arg("-")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
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
                        let output = drain_stereo_s16le_to_mono(&mut pending);
                        if !output.is_empty() && sender.send(output).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("[LinuxSystemAudio] Failed reading pw-record stdout: {}", e);
                        break;
                    }
                }
            }

            is_capturing.store(false, Ordering::SeqCst);
        });

        Ok(receiver)
    }

    /// Stop capturing.
    pub fn stop(&self) {
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

impl Default for SystemAudioCapture {
    fn default() -> Self {
        Self::new()
    }
}

fn drain_stereo_s16le_to_mono(pending: &mut Vec<u8>) -> Vec<u8> {
    let frame_bytes = 4;
    let process_len = pending.len() / frame_bytes * frame_bytes;
    if process_len == 0 {
        return Vec::new();
    }

    let mut output = Vec::with_capacity(process_len / 2);
    for frame in pending[..process_len].chunks_exact(frame_bytes) {
        let left = i16::from_le_bytes([frame[0], frame[1]]) as i32;
        let right = i16::from_le_bytes([frame[2], frame[3]]) as i32;
        let mono = ((left + right) / 2).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        output.extend_from_slice(&mono.to_le_bytes());
    }

    pending.drain(..process_len);
    output
}
