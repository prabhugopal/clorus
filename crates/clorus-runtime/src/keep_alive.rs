//! Makes every `#[no_mangle] pub extern "C" fn` in this crate genuinely
//! Rust-reachable, so a normal executable link retains all of them --
//! not just the ones some *other* Rust crate happens to call directly.
//!
//! See `build.rs` for the full rationale and how the list is generated.
//! Every host binary that embeds a JIT engine (`clorus-cli`, `clorus-repl`,
//! `clorus-replx`) must call `retain_all_runtime_symbols()` once, early,
//! before compiling any Clorus code that might call one of these functions.

include!(concat!(env!("OUT_DIR"), "/keep_alive_generated.rs"));
