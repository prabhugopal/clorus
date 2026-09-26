fn main() {
    // See crates/clorus-cli/build.rs for the full explanation. This crate
    // also produces a binary (`repl-dev`) that embeds an MCJIT engine and
    // must be able to resolve its own statically-linked `clorus_*`
    // extern "C" runtime functions by symbol name at JIT time -- which
    // requires those symbols to be present in the executable's dynamic
    // symbol table on Linux.
    let target = std::env::var("TARGET").unwrap_or_default();
    if target.contains("linux") {
        println!("cargo:rustc-link-arg=-rdynamic");
    }
}
