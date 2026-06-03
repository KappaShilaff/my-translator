use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};

/// Linux builds in this fork are system-audio only.
///
/// Keeping this stub lets the shared UI and command state compile without
/// pulling `cpal`/ALSA development headers into Linux builds.
pub struct MicCapture {
    is_capturing: Arc<AtomicBool>,
}

impl MicCapture {
    pub fn new() -> Self {
        Self {
            is_capturing: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start(&mut self) -> Result<mpsc::Receiver<Vec<u8>>, String> {
        Err(
            "Microphone capture is disabled on Linux in this build. Use System Audio instead."
                .to_string(),
        )
    }

    pub fn stop(&mut self) {
        self.is_capturing.store(false, Ordering::SeqCst);
    }
}

impl Default for MicCapture {
    fn default() -> Self {
        Self::new()
    }
}
