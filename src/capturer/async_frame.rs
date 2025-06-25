use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use anyhow::Result;
use crate::frame::Frame;

/// Represents the state of the capture stream
#[derive(Debug, Clone, PartialEq)]
pub enum CaptureState {
    Idle,
    Starting,
    Running,
    Pausing,
    Stopping,
    Error(String),
}

/// Async frame receiver that provides non-blocking access to captured frames
pub struct AsyncFrameReceiver {
    rx: mpsc::Receiver<Result<Frame>>,
    state: Arc<Mutex<CaptureState>>,
}

/// Async frame sender used by the capture engine to send frames
pub struct AsyncFrameSender {
    tx: mpsc::Sender<Result<Frame>>,
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
                state: Arc::clone(&state),
            }
        )
    }

    /// Asynchronously receives the next frame
    pub async fn next_frame(&mut self) -> Option<Result<Frame>> {
        self.rx.recv().await
    }

    /// Gets the current capture state
    pub async fn state(&self) -> CaptureState {
        self.state.lock().await.clone()
    }
}

impl AsyncFrameSender {
    /// Sends a frame asynchronously
    pub async fn send_frame(&self, frame: Result<Frame>) -> Result<()> {
        self.tx.send(frame).await.map_err(|e| anyhow::anyhow!("Failed to send frame: {}", e))
    }

    /// Updates the capture state
    pub async fn set_state(&self, state: CaptureState) {
        *self.state.lock().await = state;
    }
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