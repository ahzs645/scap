use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::sleep;

use crate::capturer::async_frame::CaptureState;

/// Configuration for error recovery behavior
#[derive(Debug, Clone)]
pub struct ErrorRecoveryConfig {
    /// Initial delay before first retry attempt
    pub initial_delay: Duration,
    /// Maximum delay between retry attempts
    pub max_delay: Duration,
    /// Maximum number of retry attempts before giving up
    pub max_retries: u32,
    /// Backoff multiplier - delay increases by this factor each attempt
    pub backoff_factor: f32,
}

impl Default for ErrorRecoveryConfig {
    fn default() -> Self {
        Self {
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            max_retries: 5,
            backoff_factor: 2.0,
        }
    }
}

/// Handles automatic recovery from stream errors
pub struct ErrorRecovery {
    config: ErrorRecoveryConfig,
    state: Arc<Mutex<CaptureState>>,
    retry_count: Arc<Mutex<u32>>,
}

impl ErrorRecovery {
    /// Creates a new error recovery handler
    pub fn new(state: Arc<Mutex<CaptureState>>, config: Option<ErrorRecoveryConfig>) -> Self {
        Self {
            config: config.unwrap_or_default(),
            state,
            retry_count: Arc::new(Mutex::new(0)),
        }
    }

    /// Handles a stream error and attempts recovery
    pub async fn handle_error(&self, error: &str) -> bool {
        // Update state to error
        {
            let mut state = self.state.lock().await;
            *state = CaptureState::Error(error.to_string());
        }

        let mut retry_count = self.retry_count.lock().await;
        if *retry_count >= self.config.max_retries {
            log::error!("Max retry attempts ({}) reached, giving up", self.config.max_retries);
            return false;
        }

        // Calculate delay with exponential backoff
        let delay = self.calculate_delay(*retry_count);
        *retry_count += 1;

        log::warn!(
            "Stream error occurred: {}. Attempting recovery in {:?} (attempt {}/{})",
            error,
            delay,
            *retry_count,
            self.config.max_retries
        );

        // Wait before retry
        sleep(delay).await;
        true
    }

    /// Resets the retry count after successful recovery
    pub async fn reset(&self) {
        let mut retry_count = self.retry_count.lock().await;
        *retry_count = 0;
    }

    /// Calculates the delay for the current retry attempt
    fn calculate_delay(&self, attempt: u32) -> Duration {
        let delay = self.config.initial_delay.as_millis() as f32
            * self.config.backoff_factor.powi(attempt as i32);
        
        Duration::from_millis(
            (delay as u64).min(self.config.max_delay.as_millis() as u64)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_error_recovery() {
        let state = Arc::new(Mutex::new(CaptureState::Running));
        let config = ErrorRecoveryConfig {
            initial_delay: Duration::from_millis(10),
            max_delay: Duration::from_millis(100),
            max_retries: 3,
            backoff_factor: 2.0,
        };
        
        let recovery = ErrorRecovery::new(state.clone(), Some(config));
        
        // First error
        assert!(recovery.handle_error("Test error 1").await);
        assert!(matches!(
            *state.lock().await,
            CaptureState::Error(_)
        ));
        
        // Second error
        assert!(recovery.handle_error("Test error 2").await);
        
        // Third error
        assert!(recovery.handle_error("Test error 3").await);
        
        // Fourth error - should fail
        assert!(!recovery.handle_error("Test error 4").await);
        
        // Reset and try again
        recovery.reset().await;
        assert!(recovery.handle_error("Test error after reset").await);
    }

    #[test]
    fn test_backoff_calculation() {
        let state = Arc::new(Mutex::new(CaptureState::Running));
        let config = ErrorRecoveryConfig {
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_millis(1000),
            max_retries: 5,
            backoff_factor: 2.0,
        };
        
        let recovery = ErrorRecovery::new(state, Some(config));
        
        assert_eq!(recovery.calculate_delay(0).as_millis(), 100);
        assert_eq!(recovery.calculate_delay(1).as_millis(), 200);
        assert_eq!(recovery.calculate_delay(2).as_millis(), 400);
        assert_eq!(recovery.calculate_delay(3).as_millis(), 800);
        assert_eq!(recovery.calculate_delay(4).as_millis(), 1000); // Capped at max_delay
    }
} 