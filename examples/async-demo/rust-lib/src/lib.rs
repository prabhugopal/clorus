/// Async demo with smol runtime
use smol::Timer;
use std::time::Duration;

/// Simple async hello world
pub async fn async_hello() -> String {
    // Sleep for 100ms asynchronously
    Timer::after(Duration::from_millis(100)).await;
    "Hello from async world!".to_string()
}

/// Async countdown
pub async fn async_countdown(n: f64) -> f64 {
    let count = n as u64;
    for i in (1..=count).rev() {
        println!("Countdown: {}", i);
        Timer::after(Duration::from_millis(500)).await;
    }
    println!("Liftoff!");
    0.0
}

/// Blocking wrapper for async_hello (for FFI)
pub fn hello_blocking() -> String {
    smol::block_on(async_hello())
}

/// Blocking wrapper for countdown (for FFI)
pub fn countdown_blocking(n: f64) -> f64 {
    smol::block_on(async_countdown(n))
}

pub mod ffi;
