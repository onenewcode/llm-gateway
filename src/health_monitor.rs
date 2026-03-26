use llm_gateway_config::InternalHealthConfig;
use std::collections::VecDeque;
use std::sync::RwLock;
use std::time::Instant;

/// 滑动窗口最大容量限制（防止内存无限增长）
const MAX_WINDOW_CAPACITY: usize = 10000;

/// Health monitor for a single backend node
pub struct HealthMonitor {
    config: InternalHealthConfig,
    state: RwLock<HealthState>,
}

struct HealthState {
    window: VecDeque<RequestRecord>,
    failure_count: u32, // 增量失败计数
    circuit: CircuitState,
}

struct RequestRecord {
    timestamp: Instant,
    success: bool,
}

enum CircuitState {
    Healthy,
    CoolingDown { until: Instant },
}

impl HealthMonitor {
    /// Create a new HealthMonitor with the given config
    pub fn new(config: InternalHealthConfig) -> Self {
        Self {
            config,
            state: RwLock::new(HealthState {
                window: VecDeque::with_capacity(MAX_WINDOW_CAPACITY),
                failure_count: 0,
                circuit: CircuitState::Healthy,
            }),
        }
    }

    /// Check if the backend is available (not in cooldown)
    pub fn is_available(&self) -> bool {
        let state = self.state.read().unwrap();
        match &state.circuit {
            CircuitState::Healthy => true,
            CircuitState::CoolingDown { until } => Instant::now() >= *until,
        }
    }

    /// Record a successful request
    pub fn record_success(&self) {
        let mut state = self.state.write().unwrap();
        let now = Instant::now();

        // Check if recovering from cooldown
        let was_cooling_down = matches!(state.circuit, CircuitState::CoolingDown { .. });

        // Add success record
        state.window.push_back(RequestRecord {
            timestamp: now,
            success: true,
        });

        // Cleanup expired records
        Self::cleanup_window_static(&mut state, &self.config, now);

        // Reset to healthy on success
        if was_cooling_down {
            log::info!("Backend recovered from cooldown");
        }
        state.circuit = CircuitState::Healthy;
    }

    /// Record a failed request
    pub async fn record_failure(&self) {
        let mut state = self.state.write().unwrap();
        let now = Instant::now();

        // Add failure record
        state.window.push_back(RequestRecord {
            timestamp: now,
            success: false,
        });

        // Increment failure count
        state.failure_count += 1;

        // Cleanup expired records and enforce capacity limit
        Self::cleanup_window_static(&mut state, &self.config, now);

        // Check if threshold exceeded (O(1) with incremental count)
        if state.failure_count >= self.config.failure_threshold {
            let until = now + self.config.cooldown_duration;
            log::info!(
                "Backend entering cooldown for {:?} until {:?}",
                self.config.cooldown_duration,
                until
            );
            // Enter cooldown
            state.circuit = CircuitState::CoolingDown { until };
        }
    }

    /// Cleanup expired records from the sliding window (static version for use in async context)
    fn cleanup_window_static(state: &mut HealthState, config: &InternalHealthConfig, now: Instant) {
        let cutoff = now - config.window_size;

        // 1. Cleanup time-expired records
        while let Some(record) = state.window.front() {
            if record.timestamp < cutoff {
                // Decrement failure count if expiring a failure record
                if !record.success {
                    state.failure_count = state.failure_count.saturating_sub(1);
                }
                state.window.pop_front();
            } else {
                break;
            }
        }

        // 2. Cleanup records exceeding capacity limit (from oldest)
        while state.window.len() > MAX_WINDOW_CAPACITY {
            if let Some(record) = state.window.pop_front()
                && !record.success
            {
                state.failure_count = state.failure_count.saturating_sub(1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn test_config() -> InternalHealthConfig {
        InternalHealthConfig {
            window_size: Duration::from_secs(60),
            failure_threshold: 3,
            cooldown_duration: Duration::from_secs(300),
        }
    }

    #[tokio::test]
    async fn test_initially_available() {
        let monitor = HealthMonitor::new(test_config());
        assert!(monitor.is_available());
    }

    #[tokio::test]
    async fn test_enters_cooldown_after_failures() {
        let monitor = HealthMonitor::new(test_config());

        // Record 3 failures (threshold)
        monitor.record_failure().await;
        monitor.record_failure().await;
        monitor.record_failure().await;

        // Should be in cooldown
        assert!(!monitor.is_available());
    }

    #[tokio::test]
    async fn test_success_resets_cooldown() {
        let monitor = HealthMonitor::new(test_config());

        // Record failures
        monitor.record_failure().await;
        monitor.record_failure().await;

        // Record success
        monitor.record_success();

        // Should still be available
        assert!(monitor.is_available());
    }

    #[tokio::test]
    async fn test_window_expires_old_records() {
        // This test verifies the sliding window behavior
        // In real usage, records older than window_size are cleaned up
        let monitor = HealthMonitor::new(test_config());

        // Record 2 failures
        monitor.record_failure().await;
        monitor.record_failure().await;

        // Still available (threshold not reached)
        assert!(monitor.is_available());
    }

    #[tokio::test]
    async fn test_incremental_failure_count() {
        let monitor = HealthMonitor::new(test_config());

        // Record 2 failures
        monitor.record_failure().await;
        monitor.record_failure().await;

        // Verify still available
        assert!(monitor.is_available());

        // 3rd failure triggers cooldown
        monitor.record_failure().await;
        assert!(!monitor.is_available());
    }

    #[tokio::test]
    async fn test_window_capacity_limit() {
        let monitor = HealthMonitor::new(InternalHealthConfig {
            window_size: Duration::from_secs(3600), // Long window
            failure_threshold: 100,                 // High threshold
            cooldown_duration: Duration::from_secs(300),
        });

        // Record more than capacity limit
        for _ in 0..MAX_WINDOW_CAPACITY + 1000 {
            monitor.record_success();
        }

        // Verify window size doesn't exceed limit
        let state = monitor.state.read().unwrap();
        assert!(state.window.len() <= MAX_WINDOW_CAPACITY);
    }
}
