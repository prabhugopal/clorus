//! Execution wrapper for adaptive REPL
//! Intercepts and transforms expressions based on detection

use crate::{FunctionDetector, AdaptiveConfig, AdaptationLevel};

/// Wraps base REPL engine with adaptive execution
pub struct AdaptiveWrapper {
    config: AdaptiveConfig,
    detector: FunctionDetector,
}

impl AdaptiveWrapper {
    pub fn new(config: AdaptiveConfig) -> Self {
        Self {
            config,
            detector: FunctionDetector::new(),
        }
    }

    /// Transform expression based on adaptive strategy
    pub fn transform_expr(&self, expr: &str) -> (String, bool) {
        if !self.config.enabled {
            return (expr.to_string(), false);
        }

        let metadata = self.detector.analyze(expr);

        // Determine if we should adapt
        let should_adapt = metadata.blocks &&
                          metadata.needs_main_thread &&
                          self.config.auto_detach_gui;

        if !should_adapt {
            return (expr.to_string(), false);
        }

        // Show message based on adaptation level
        match self.config.level {
            AdaptationLevel::None => {
                (expr.to_string(), false)
            }

            AdaptationLevel::Warn => {
                println!("⚠️  Warning: {} may block the REPL", metadata.name);
                println!("💡 Suggestion: Use (go {}) for background execution", expr);
                (expr.to_string(), false)
            }

            AdaptationLevel::AutoAdapt => {
                println!("⚡ Detected: GUI function (blocks on main thread)");
                println!("✓ Auto-adapted: Running in go block");

                // Transform: (gui/show-gui "x") -> (go (gui/show-gui "x"))
                let wrapped = format!("(go {})", expr);
                (wrapped, true)
            }

            AdaptationLevel::Silent => {
                // Auto-adapt silently
                let wrapped = format!("(go {})", expr);
                (wrapped, true)
            }
        }
    }
}
