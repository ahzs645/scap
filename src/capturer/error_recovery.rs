use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;
use anyhow::Error;

use super::async_frame::CaptureState;

/// Configuration for error recovery behavior
#[derive(Debug, Clone)]
pub struct ErrorRecoveryConfig {
    /// Maximum number of retry attempts before giving up
    pub max_retries: u32,
    /// Initial delay before first retry attempt
    pub initial_retry_delay: Duration,
    /// Maximum delay between retry attempts
    pub max_retry_delay: Duration,
    /// Backoff multiplier - delay increases by this factor each attempt
    pub backoff_factor: f32,
}

impl Default for ErrorRecoveryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_retry_delay: Duration::from_millis(100),
            max_retry_delay: Duration::from_secs(5),
            backoff_factor: 2.0,
        }
    }
}

/// Handles automatic recovery from stream errors
pub struct ErrorRecovery {
    state: Arc<Mutex<CaptureState>>,
    config: ErrorRecoveryConfig,
    retry_count: u32,
    current_delay: Duration,
}

impl ErrorRecovery {
    /// Creates a new error recovery handler
    pub fn new(state: Arc<Mutex<CaptureState>>, config: Option<ErrorRecoveryConfig>) -> Self {
        let config = config.unwrap_or_default();
        Self {
            state,
            config,
            retry_count: 0,
            current_delay: config.initial_retry_delay,
        }
    }

    /// Handles a stream error and attempts recovery
    pub async fn handle_error(&mut self, error: &Error) -> Option<Duration> {
        // Check if we should retry
        if self.retry_count >= self.config.max_retries {
            // Reset retry count and delay for next time
            self.retry_count = 0;
            self.current_delay = self.config.initial_retry_delay;
            return None;
        }

        // Calculate next retry delay with exponential backoff
        let retry_delay = self.current_delay;
        self.current_delay = Duration::from_secs_f32(
            (self.current_delay.as_secs_f32() * self.config.backoff_factor)
                .min(self.config.max_retry_delay.as_secs_f32())
        );
        self.retry_count += 1;

        // Update state to indicate error
        let mut state = self.state.lock().await;
        *state = CaptureState::Stopped;

        Some(retry_delay)
    }

    /// Resets the retry count after successful recovery
    pub fn reset(&mut self) {
        self.retry_count = 0;
        self.current_delay = self.config.initial_retry_delay;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_error_recovery() {
        let state = Arc::new(Mutex::new(CaptureState::Running));
        let config = ErrorRecoveryConfig {
            initial_retry_delay: Duration::from_millis(10),
            max_retry_delay: Duration::from_millis(100),
            max_retries: 3,
            backoff_factor: 2.0,
        };
        
        let mut recovery = ErrorRecovery::new(state.clone(), Some(config));
        
        // First error
        assert!(recovery.handle_error(&Error::msg("Test error 1")).await.is_some());
        assert!(matches!(
            *state.lock().await,
            CaptureState::Stopped
        ));
        
        // Second error
        assert!(recovery.handle_error(&Error::msg("Test error 2")).await.is_some());
        
        // Third error
        assert!(recovery.handle_error(&Error::msg("Test error 3")).await.is_some());
        
        // Fourth error - should fail
        assert!(recovery.handle_error(&Error::msg("Test error 4")).await.is_none());
        
        // Reset and try again
        recovery.reset();
        assert!(recovery.handle_error(&Error::msg("Test error after reset")).await.is_some());
    }

    #[test]
    fn test_backoff_calculation() {
        let state = Arc::new(Mutex::new(CaptureState::Running));
        let config = ErrorRecoveryConfig {
            initial_retry_delay: Duration::from_millis(100),
            max_retry_delay: Duration::from_millis(1000),
            max_retries: 5,
            backoff_factor: 2.0,
        };
        
        let mut recovery = ErrorRecovery::new(state, Some(config));
        
        assert_eq!(recovery.current_delay.as_millis(), 100);
        assert_eq!(recovery.handle_error(&Error::msg("")).await.unwrap().as_millis(), 100);
        assert_eq!(recovery.current_delay.as_millis(), 200);
        assert_eq!(recovery.handle_error(&Error::msg("")).await.unwrap().as_millis(), 200);
        assert_eq!(recovery.current_delay.as_millis(), 400);
        assert_eq!(recovery.handle_error(&Error::msg("")).await.unwrap().as_millis(), 400);
        assert_eq!(recovery.calculate_delay(0).as_millis(), 100);
        assert_eq!(recovery.calculate_delay(1).as_millis(), 200);
        assert_eq!(recovery.calculate_delay(2).as_millis(), 400);
        assert_eq!(recovery.calculate_delay(3).as_millis(), 800);
        assert_eq!(recovery.calculate_delay(4).as_millis(), 1000); // Capped at max_delay
    }
} 