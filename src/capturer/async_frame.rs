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
    Stopped,
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
                state,
            }
        )
    }

    /// Asynchronously receives the next frame
    pub async fn recv(&mut self) -> Result<Frame> {
        match self.rx.recv().await {
            Some(result) => result,
            None => Err(anyhow::anyhow!("Frame channel closed")),
        }
    }

    /// Try to receive a frame without blocking
    pub fn try_recv(&mut self) -> Result<Frame> {
        match self.rx.try_recv() {
            Ok(result) => result,
            Err(mpsc::error::TryRecvError::Empty) => Err(anyhow::anyhow!("No frame available")),
            Err(mpsc::error::TryRecvError::Disconnected) => Err(anyhow::anyhow!("Frame channel closed")),
        }
    }

    /// Gets the current capture state
    pub async fn state(&self) -> CaptureState {
        self.state.lock().await.clone()
    }
}

impl AsyncFrameSender {
    /// Sends a frame asynchronously
    pub fn send(&self, frame: Result<Frame>) -> Result<()> {
        self.tx.try_send(frame)
            .map_err(|e| anyhow::anyhow!("Failed to send frame: {}", e))
    }

    /// Sends a frame with success result
    pub fn send_frame(&self, frame: Result<Frame>) -> Result<()> {
        self.send(frame)
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
    let state = Arc::new(Mutex::new(CaptureState::Idle));
    (
        AsyncFrameSender { 
            tx, 
            state: Arc::clone(&state),
        },
        AsyncFrameReceiver { 
            rx, 
            state,
        },
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
                sender.send_frame(Ok(test_frame)).unwrap();
            }
        });

        let received_frame = receiver.recv().await.unwrap();
        match received_frame {
            Frame::RGB(frame) => {
                assert_eq!(frame.width, 100);
                assert_eq!(frame.height, 100);
            }
            _ => panic!("Unexpected frame type"),
        }
    }
}