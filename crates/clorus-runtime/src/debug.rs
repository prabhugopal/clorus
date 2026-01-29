/// Debug and memory tracking for Clorus runtime
///
/// Provides instrumentation for tracking allocations, deallocations,
/// and reference count operations.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Global debug mode flag
static DEBUG_MODE: AtomicBool = AtomicBool::new(false);

/// Start time for timestamp calculations
static mut START_TIME: Option<Instant> = None;

/// Debug events log
static DEBUG_EVENTS: Mutex<Vec<DebugEvent>> = Mutex::new(Vec::new());

/// Types of debug events
#[derive(Debug, Clone)]
pub enum DebugEvent {
    Alloc {
        timestamp_ms: f64,
        address: usize,
        type_name: String,
        size: usize,
        refcount: u64,
        file: String,
        line: u32,
        expression: String,
    },
    Retain {
        timestamp_ms: f64,
        address: usize,
        refcount_before: u64,
        refcount_after: u64,
        file: String,
        line: u32,
    },
    Release {
        timestamp_ms: f64,
        address: usize,
        refcount_before: u64,
        refcount_after: u64,
        freed: bool,
        file: String,
        line: u32,
    },
}

/// Enable debug mode
pub fn enable_debug_mode() {
    unsafe {
        START_TIME = Some(Instant::now());
    }
    DEBUG_MODE.store(true, Ordering::SeqCst);
}

/// Check if debug mode is enabled
#[inline]
pub fn is_debug_mode() -> bool {
    DEBUG_MODE.load(Ordering::SeqCst)
}

/// Get elapsed time since debug mode started
fn elapsed_ms() -> f64 {
    unsafe {
        if let Some(start) = START_TIME {
            start.elapsed().as_secs_f64() * 1000.0
        } else {
            0.0
        }
    }
}

/// Record a debug event
fn record_event(event: DebugEvent) {
    if let Ok(mut events) = DEBUG_EVENTS.lock() {
        events.push(event);
    }
}

/// Get all debug events
pub fn get_debug_events() -> Vec<DebugEvent> {
    DEBUG_EVENTS.lock().unwrap().clone()
}

/// Clear debug events
pub fn clear_debug_events() {
    DEBUG_EVENTS.lock().unwrap().clear();
}

// FFI functions for LLVM-generated code

/// Record an allocation event
#[no_mangle]
pub extern "C" fn clorus_debug_alloc(
    address: usize,
    type_name: *const i8,
    size: usize,
    refcount: u64,
    file: *const i8,
    line: u32,
    expr: *const i8,
) {
    if !is_debug_mode() {
        return;
    }

    use std::ffi::CStr;

    let type_name = unsafe {
        if type_name.is_null() {
            "Unknown".to_string()
        } else {
            CStr::from_ptr(type_name)
                .to_string_lossy()
                .into_owned()
        }
    };

    let file = unsafe {
        if file.is_null() {
            "?".to_string()
        } else {
            CStr::from_ptr(file).to_string_lossy().into_owned()
        }
    };

    let expression = unsafe {
        if expr.is_null() {
            "".to_string()
        } else {
            CStr::from_ptr(expr).to_string_lossy().into_owned()
        }
    };

    let event = DebugEvent::Alloc {
        timestamp_ms: elapsed_ms(),
        address,
        type_name,
        size,
        refcount,
        file,
        line,
        expression,
    };

    record_event(event);
}

/// Record a retain event
#[no_mangle]
pub extern "C" fn clorus_debug_retain(
    address: usize,
    refcount_before: u64,
    refcount_after: u64,
    file: *const i8,
    line: u32,
) {
    if !is_debug_mode() {
        return;
    }

    use std::ffi::CStr;

    let file = unsafe {
        if file.is_null() {
            "?".to_string()
        } else {
            CStr::from_ptr(file).to_string_lossy().into_owned()
        }
    };

    let event = DebugEvent::Retain {
        timestamp_ms: elapsed_ms(),
        address,
        refcount_before,
        refcount_after,
        file,
        line,
    };

    record_event(event);
}

/// Record a release event
#[no_mangle]
pub extern "C" fn clorus_debug_release(
    address: usize,
    refcount_before: u64,
    refcount_after: u64,
    freed: bool,
    file: *const i8,
    line: u32,
) {
    if !is_debug_mode() {
        return;
    }

    use std::ffi::CStr;

    let file = unsafe {
        if file.is_null() {
            "?".to_string()
        } else {
            CStr::from_ptr(file).to_string_lossy().into_owned()
        }
    };

    let event = DebugEvent::Release {
        timestamp_ms: elapsed_ms(),
        address,
        refcount_before,
        refcount_after,
        freed,
        file,
        line,
    };

    record_event(event);
}

/// Print debug summary
pub fn print_debug_summary() {
    let events = get_debug_events();

    let mut allocs = 0;
    let mut frees = 0;

    for event in &events {
        match event {
            DebugEvent::Alloc { .. } => allocs += 1,
            DebugEvent::Release { freed: true, .. } => frees += 1,
            _ => {}
        }
    }

    println!("\n🔧 Clorus Debug Summary");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Total Allocations:   {}", allocs);
    println!("Total Deallocations: {}", frees);
    println!("Leaked Objects:      {}", allocs - frees);

    if allocs == frees {
        println!("✓ No memory leaks detected!");
    } else {
        println!("⚠️  Potential memory leak!");
    }
}

/// Print detailed debug events
pub fn print_debug_events() {
    let events = get_debug_events();

    println!("\n🔧 Clorus Debug Events");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    for event in events {
        match event {
            DebugEvent::Alloc {
                timestamp_ms,
                address,
                type_name,
                size,
                refcount,
                line,
                expression,
                ..
            } => {
                println!(
                    "[{:07.3}ms] ALLOC   {:?}@{:x}  size={}  rc={}  line:{}  {}",
                    timestamp_ms,
                    type_name,
                    address,
                    size,
                    refcount,
                    line,
                    if expression.is_empty() {
                        "".to_string()
                    } else {
                        format!("({})", expression)
                    }
                );
            }
            DebugEvent::Retain {
                timestamp_ms,
                address,
                refcount_before,
                refcount_after,
                line,
                ..
            } => {
                println!(
                    "[{:07.3}ms] RETAIN  @{:x}  rc={}→{}  line:{}",
                    timestamp_ms, address, refcount_before, refcount_after, line
                );
            }
            DebugEvent::Release {
                timestamp_ms,
                address,
                refcount_before,
                refcount_after,
                freed,
                line,
                ..
            } => {
                println!(
                    "[{:07.3}ms] RELEASE @{:x}  rc={}→{}  {}  line:{}",
                    timestamp_ms,
                    address,
                    refcount_before,
                    refcount_after,
                    if freed { "FREE" } else { "" },
                    line
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_mode() {
        assert!(!is_debug_mode());
        enable_debug_mode();
        assert!(is_debug_mode());
    }

    #[test]
    fn test_event_recording() {
        enable_debug_mode();
        clear_debug_events();

        let type_name = std::ffi::CString::new("TestType").unwrap();
        let file = std::ffi::CString::new("test.clrs").unwrap();
        let expr = std::ffi::CString::new("(def x 10)").unwrap();

        clorus_debug_alloc(
            0x1234,
            type_name.as_ptr(),
            48,
            1,
            file.as_ptr(),
            10,
            expr.as_ptr(),
        );

        let events = get_debug_events();
        assert_eq!(events.len(), 1);

        match &events[0] {
            DebugEvent::Alloc { address, .. } => {
                assert_eq!(*address, 0x1234);
            }
            _ => panic!("Expected Alloc event"),
        }
    }
}
