//! Worst-Case Execution Time (WCET) enforcement.
//!
//! Provides a guard that measures elapsed time and either logs or
//! hard-fails if the operation exceeds its WCET budget.

use crate::config::WcetMode;
use std::time::Instant;

/// WCET budget configuration
#[derive(Debug, Clone, Copy)]
pub struct WcetBudget {
    pub envelope_parse_us: u64,
    pub blake3_hash_us: u64,
    pub ed25519_verify_us: u64,
    pub ring_buffer_us: u64,
    pub total_pipeline_us: u64,
}

impl Default for WcetBudget {
    fn default() -> Self {
        Self {
            envelope_parse_us: 50,
            blake3_hash_us: 100,
            ed25519_verify_us: 500,
            ring_buffer_us: 5,
            total_pipeline_us: 1000,
        }
    }
}

/// A RAII guard that measures execution time and enforces WCET.
pub struct WcetGuard {
    operation: &'static str,
    budget_us: u64,
    mode: WcetMode,
    start: Instant,
}

/// Result of a WCET check
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WcetResult {
    /// Operation completed within budget
    Ok { elapsed_us: u64 },
    /// Operation exceeded budget but mode is Log
    Violation { elapsed_us: u64, budget_us: u64 },
}

/// Error when WCET is exceeded in HardFail mode
#[derive(Debug, Clone)]
pub struct WcetViolation {
    pub operation: &'static str,
    pub elapsed_us: u64,
    pub budget_us: u64,
}

impl core::fmt::Display for WcetViolation {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "WCET VIOLATION: {} took {}μs (budget: {}μs)",
            self.operation, self.elapsed_us, self.budget_us
        )
    }
}

impl WcetGuard {
    /// Start timing an operation.
    pub fn start(operation: &'static str, budget_us: u64, mode: WcetMode) -> Self {
        Self {
            operation,
            budget_us,
            mode,
            start: Instant::now(),
        }
    }

    /// Finish timing and check against budget.
    /// Returns `Err` only in `HardFail` mode when budget is exceeded.
    pub fn finish(self) -> Result<WcetResult, WcetViolation> {
        let elapsed = self.start.elapsed();
        let elapsed_us = elapsed.as_micros() as u64;

        if elapsed_us <= self.budget_us {
            return Ok(WcetResult::Ok { elapsed_us });
        }

        // Budget exceeded
        match self.mode {
            WcetMode::Log => {
                println!(
                    "WCET WARNING: {} took {}μs (budget: {}μs)",
                    self.operation, elapsed_us, self.budget_us
                );
                Ok(WcetResult::Violation {
                    elapsed_us,
                    budget_us: self.budget_us,
                })
            }
            WcetMode::HardFail => Err(WcetViolation {
                operation: self.operation,
                elapsed_us,
                budget_us: self.budget_us,
            }),
        }
    }

    /// Returns elapsed microseconds so far (without finishing).
    pub fn elapsed_us(&self) -> u64 {
        self.start.elapsed().as_micros() as u64
    }
}

/// Convenience: time an entire closure with WCET enforcement.
pub fn timed<F, T>(
    operation: &'static str,
    budget_us: u64,
    mode: WcetMode,
    f: F,
) -> Result<(T, WcetResult), WcetViolation>
where
    F: FnOnce() -> T,
{
    let guard = WcetGuard::start(operation, budget_us, mode);
    let result = f();
    let wcet = guard.finish()?;
    Ok((result, wcet))
}
