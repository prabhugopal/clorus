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

        // CRITICAL: On macOS, GUI functions MUST run on main thread
        // Wrapping in go blocks crashes with "EventLoop must be created on the main thread"
        // So we detect macOS + GUI and provide helpful guidance instead of auto-adapting
        #[cfg(target_os = "macos")]
        {
            if metadata.needs_main_thread {
                match self.config.level {
                    AdaptationLevel::None => {
                        return (expr.to_string(), false);
                    }
                    AdaptationLevel::Warn | AdaptationLevel::AutoAdapt => {
                        println!("⚠️  macOS GUI detected: {} requires main thread", metadata.name);
                        println!("💡 Note: GUI will block REPL until window closes");
                        println!("   This is a macOS limitation - GUI EventLoop must run on main thread");
                        return (expr.to_string(), false);
                    }
                    AdaptationLevel::Silent => {
                        return (expr.to_string(), false);
                    }
                }
            }
        }

        // For non-macOS or non-GUI functions, apply normal adaptation
        #[cfg(not(target_os = "macos"))]
        {
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
                    println!("⚡ Detected: Blocking function");
                    println!("✓ Auto-adapted: Running in go block");

                    // Transform: (server/listen) -> (go (server/listen))
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

        // Fallback for macOS non-GUI blocking functions
        #[cfg(target_os = "macos")]
        {
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
                    println!("⚡ Detected: Blocking function");
                    println!("✓ Auto-adapted: Running in go block");

                    let wrapped = format!("(go {})", expr);
                    (wrapped, true)
                }

                AdaptationLevel::Silent => {
                    let wrapped = format!("(go {})", expr);
                    (wrapped, true)
                }
            }
        }
    }
}
