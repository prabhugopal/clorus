//! Execution strategies and adaptive configuration

use serde::{Deserialize, Serialize};

/// How to execute a function based on detected characteristics
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionStrategy {
    /// Normal execution (blocks REPL until done)
    Normal,

    /// Spawn in background thread, return immediately
    Detached,

    /// Run on main thread in detached context (for GUI on macOS)
    DetachedMainThread,

    /// Suggest async execution to user
    SuggestAsync,

    /// Parallelize if possible
    Parallel,
}

/// Configuration for adaptive execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveConfig {
    /// Enable adaptive features
    pub enabled: bool,

    /// How aggressive to be with adaptations
    pub level: AdaptationLevel,

    /// Run on main thread (for GUI)
    pub main_thread: bool,

    /// Auto-detach GUI functions
    pub auto_detach_gui: bool,

    /// Auto-detach server functions
    pub auto_detach_servers: bool,

    /// Suggest optimizations to user
    pub suggest_optimizations: bool,

    /// Warn if function takes longer than this (ms)
    pub warn_long_running_ms: u64,

    /// Enable learning from execution patterns
    #[cfg(feature = "learning")]
    pub learn: bool,
}

impl Default for AdaptiveConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            level: AdaptationLevel::AutoAdapt,
            main_thread: false,
            auto_detach_gui: true,
            auto_detach_servers: false,
            suggest_optimizations: true,
            warn_long_running_ms: 5000, // 5 seconds

            #[cfg(feature = "learning")]
            learn: true,
        }
    }
}

/// How aggressively to adapt execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdaptationLevel {
    /// No adaptation (traditional REPL)
    None,

    /// Warn about potential issues
    Warn,

    /// Auto-adapt with notifications
    AutoAdapt,

    /// Auto-adapt silently
    Silent,
}

impl AdaptiveConfig {
    /// Create config for main-thread mode with GUI auto-detach
    pub fn main_thread_gui() -> Self {
        Self {
            main_thread: true,
            auto_detach_gui: true,
            ..Default::default()
        }
    }

    /// Create config with warnings only
    pub fn warn_only() -> Self {
        Self {
            level: AdaptationLevel::Warn,
            ..Default::default()
        }
    }

    /// Load from TOML file
    pub fn load_from_file(path: &str) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config: {}", e))?;

        toml::from_str(&content)
            .map_err(|e| format!("Failed to parse config: {}", e))
    }

    /// Save to TOML file
    pub fn save_to_file(&self, path: &str) -> Result<(), String> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        std::fs::write(path, content)
            .map_err(|e| format!("Failed to write config: {}", e))
    }
}
