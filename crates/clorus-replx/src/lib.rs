//! Clorus REPL Extended (replx)
//!
//! Enhanced REPL with smart, adaptive execution that prevents blocking
//! and auto-detects execution contexts (GUI, compute, I/O, etc.)

mod detector;
mod strategy;

pub use detector::{FunctionDetector, FunctionMetadata, ExecutionPattern};
pub use strategy::{ExecutionStrategy, AdaptiveConfig, AdaptationLevel};

use clorus_repl::ReplConfig;

#[cfg(feature = "learning")]
use std::collections::HashMap;

/// Extended REPL engine with adaptive execution
pub struct ReplX {
    config: AdaptiveConfig,
    detector: FunctionDetector,

    #[cfg(feature = "learning")]
    profiles: HashMap<String, FunctionProfile>,
}

#[cfg(feature = "learning")]
#[derive(Debug, Clone)]
struct FunctionProfile {
    call_count: u64,
    blocked_count: u64,
    learned_pattern: Option<ExecutionPattern>,
}

impl ReplX {
    /// Create a new extended REPL with smart execution
    pub fn new(config: AdaptiveConfig) -> Self {
        Self {
            config,
            detector: FunctionDetector::new(),

            #[cfg(feature = "learning")]
            profiles: HashMap::new(),
        }
    }

    /// Create with default configuration
    pub fn default() -> Self {
        Self::new(AdaptiveConfig::default())
    }

    /// Convert to base REPL config
    pub fn to_repl_config(&self) -> ReplConfig {
        ReplConfig {
            main_thread: self.config.main_thread,
            ..Default::default()
        }
    }

    /// Analyze expression and determine if adaptation needed
    pub fn should_adapt(&self, expr: &str) -> Option<ExecutionStrategy> {
        if !self.config.enabled {
            return None;
        }

        let metadata = self.detector.analyze(expr);

        match self.config.level {
            AdaptationLevel::None => None,

            AdaptationLevel::Warn => {
                if metadata.blocks && metadata.needs_main_thread {
                    println!("⚠️  Warning: This function may block the REPL");
                    println!("💡 Consider using: clorus replx --main-thread");
                }
                None
            }

            AdaptationLevel::AutoAdapt | AdaptationLevel::Silent => {
                self.determine_strategy(&metadata)
            }
        }
    }

    fn determine_strategy(&self, metadata: &FunctionMetadata) -> Option<ExecutionStrategy> {
        // Auto-detach GUI functions that block
        if metadata.blocks && metadata.needs_main_thread && self.config.auto_detach_gui {
            if self.config.level == AdaptationLevel::AutoAdapt {
                println!("⚡ Detected: GUI function (blocks on main thread)");
                println!("✓ Auto-adapted: Spawned in detached context");
            }
            return Some(ExecutionStrategy::DetachedMainThread);
        }

        // Suggest async for long-running operations
        if metadata.estimated_duration_ms > self.config.warn_long_running_ms {
            if self.config.suggest_optimizations {
                println!("💡 Hint: Long computation detected.");
                println!("   Consider: (async {}) for background execution", metadata.name);
            }
        }

        None
    }

    #[cfg(feature = "learning")]
    pub fn learn_from_execution(&mut self, func_name: &str, blocked: bool) {
        let profile = self.profiles.entry(func_name.to_string())
            .or_insert(FunctionProfile {
                call_count: 0,
                blocked_count: 0,
                learned_pattern: None,
            });

        profile.call_count += 1;
        if blocked {
            profile.blocked_count += 1;
        }

        // Learn: if blocked >3 times, remember to detach
        if profile.blocked_count > 3 && profile.learned_pattern.is_none() {
            profile.learned_pattern = Some(ExecutionPattern::GUI);
            println!("📝 Learned: {} → auto-detach", func_name);
        }
    }
}

/// Run the extended REPL with adaptive features
pub fn run() -> Result<(), String> {
    run_with_config(AdaptiveConfig::default())
}

/// Run with custom configuration
pub fn run_with_config(config: AdaptiveConfig) -> Result<(), String> {
    let replx = ReplX::new(config);
    let repl_config = replx.to_repl_config();

    // For now, delegate to base REPL
    // Future: wrap evaluation to apply adaptive strategies
    clorus_repl::run_with_config(repl_config)
}
