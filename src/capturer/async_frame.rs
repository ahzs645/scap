use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use anyhow::Result;
use crate::frame::Frame;

/// Represents the state of the capture stream
#[derive(Debug, Clone, PartialEq)]
pub enum CaptureState {
    Running,
    Stopped,
    Error(String),
}

/// Async frame receiver that provides non-blocking access to captured frames
pub struct AsyncFrameReceiver {
    rx: mpsc::Receiver<Frame>,
    state: Arc<Mutex<CaptureState>>,
}

/// Async frame sender used by the capture engine to send frames
pub struct AsyncFrameSender {
    tx: mpsc::Sender<Frame>,
    state: Arc<Mutex<CaptureState>>,
}

impl AsyncFrameReceiver {
    /// Creates a new async frame channel with specified buffer size
    pub fn new(buffer_size: usize) -> (Self, AsyncFrameSender) {
        let (tx, rx) = mpsc::channel(buffer_size);
        let state = Arc::new(Mutex::new(CaptureState::Idle));
        
        (
            Self {
                rx,
                state: Arc::clone(&state),
            },
            AsyncFrameSender {
                tx,
                state,
            }
        )
    }

    /// Asynchronously receives the next frame
    pub async fn recv(&mut self) -> Result<Frame> {
        self.rx.recv().await
            .ok_or_else(|| anyhow::anyhow!("Frame channel closed"))
    }

    /// Gets the current capture state
    pub async fn state(&self) -> CaptureState {
        *self.state.lock().await
    }
}

impl AsyncFrameSender {
    /// Sends a frame asynchronously
    pub fn send(&self, frame: Frame) -> Result<()> {
        self.tx.try_send(frame)
            .map_err(|e| anyhow::anyhow!("Failed to send frame: {}", e))
    }

    /// Sets the capture state
    pub async fn set_state(&self, state: CaptureState) {
        *self.state.lock().await = state;
    }
}

impl Clone for AsyncFrameSender {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
            state: Arc::clone(&self.state),
        }
    }
}

pub fn create_channel() -> (AsyncFrameSender, AsyncFrameReceiver) {
    let (tx, rx) = mpsc::channel(32); // Buffer size of 32 frames
    (
        AsyncFrameSender { tx, state: Arc::new(Mutex::new(CaptureState::Idle)) },
        AsyncFrameReceiver { rx },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_async_frame_channel() {
        let (mut receiver, sender) = AsyncFrameReceiver::new(10);

        // Test initial state
        assert_eq!(receiver.state().await, CaptureState::Idle);

        // Test state changes
        sender.set_state(CaptureState::Starting).await;
        assert_eq!(receiver.state().await, CaptureState::Starting);

        sender.set_state(CaptureState::Running).await;
        assert_eq!(receiver.state().await, CaptureState::Running);

        // Test frame sending
        let test_frame = Frame::RGB(crate::frame::RGBFrame {
            display_time: 0,
            width: 100,
            height: 100,
            data: vec![0; 30000],
        });

        tokio::spawn({
            let sender = sender.clone();
            async move {
                sleep(Duration::from_millis(100)).await;
                sender.send_frame(Ok(test_frame)).await.unwrap();
            }
        });

        let received_frame = receiver.next_frame().await.unwrap().unwrap();
        match received_frame {
            Frame::RGB(frame) => {
                assert_eq!(frame.width, 100);
                assert_eq!(frame.height, 100);
            }
            _ => panic!("Unexpected frame type"),
        }
    }
} 