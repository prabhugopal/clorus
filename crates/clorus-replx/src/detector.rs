//! Function detection and pattern analysis

/// Detects function characteristics and execution patterns
pub struct FunctionDetector {
    // Future: load learned patterns from file
}

impl FunctionDetector {
    pub fn new() -> Self {
        Self {}
    }

    /// Analyze an expression to determine execution characteristics
    pub fn analyze(&self, expr: &str) -> FunctionMetadata {
        let name = self.extract_function_name(expr);

        FunctionMetadata {
            name: name.clone(),
            blocks: self.is_blocking(&name),
            needs_main_thread: self.needs_main_thread(&name),
            pure: self.is_pure(&name),
            pattern: self.detect_pattern(&name),
            estimated_duration_ms: self.estimate_duration(&name),
        }
    }

    fn extract_function_name(&self, expr: &str) -> String {
        // Simple extraction: (function-name args...)
        let trimmed = expr.trim();
        if trimmed.starts_with('(') {
            let inner = trimmed.trim_start_matches('(');
            inner.split_whitespace()
                .next()
                .unwrap_or("")
                .to_string()
        } else {
            expr.to_string()
        }
    }

    fn is_blocking(&self, func_name: &str) -> bool {
        // Heuristic detection of blocking functions
        func_name.contains("show") ||
        func_name.contains("gui") ||
        func_name.contains("window") ||
        func_name.contains("display") ||
        func_name.contains("render") ||
        func_name.contains("server") ||
        func_name.contains("listen") ||
        func_name.contains("serve")
    }

    fn needs_main_thread(&self, func_name: &str) -> bool {
        // Functions that need main thread (macOS GUI requirement)
        func_name.contains("gui") ||
        func_name.contains("show") ||
        func_name.contains("window") ||
        func_name.contains("display")
    }

    fn is_pure(&self, func_name: &str) -> bool {
        // Heuristic: math/compute functions are usually pure
        func_name.contains("calc") ||
        func_name.contains("compute") ||
        func_name.contains("sum") ||
        func_name.contains("factorial") ||
        func_name.contains("fib") ||
        (!self.is_blocking(func_name) && !func_name.contains("set") && !func_name.contains("reset"))
    }

    fn detect_pattern(&self, func_name: &str) -> ExecutionPattern {
        if func_name.contains("gui") || func_name.contains("window") {
            ExecutionPattern::GUI
        } else if func_name.contains("server") || func_name.contains("listen") {
            ExecutionPattern::Server
        } else if func_name.contains("compute") || func_name.contains("calc") {
            ExecutionPattern::Compute
        } else if func_name.contains("fetch") || func_name.contains("download") || func_name.contains("read") {
            ExecutionPattern::IO
        } else {
            ExecutionPattern::Unknown
        }
    }

    fn estimate_duration(&self, func_name: &str) -> u64 {
        // Very rough heuristic
        match self.detect_pattern(func_name) {
            ExecutionPattern::GUI => 0,      // Indefinite (event loop)
            ExecutionPattern::Server => 0,   // Indefinite (server loop)
            ExecutionPattern::Compute => 1000, // 1 second guess
            ExecutionPattern::IO => 500,     // 500ms guess
            ExecutionPattern::Unknown => 100, // 100ms guess
        }
    }
}

/// Metadata about a function's execution characteristics
#[derive(Debug, Clone)]
pub struct FunctionMetadata {
    pub name: String,
    pub blocks: bool,              // Will block indefinitely?
    pub needs_main_thread: bool,   // Requires main thread? (macOS GUI)
    pub pure: bool,                // Side-effect free?
    pub pattern: ExecutionPattern,
    pub estimated_duration_ms: u64,
}

/// Detected execution pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionPattern {
    GUI,        // GUI operations (show, window, etc.)
    Server,     // Server/daemon operations (listen, serve)
    Compute,    // CPU-intensive computation
    IO,         // I/O operations (read, fetch, etc.)
    Unknown,    // Cannot determine
}
