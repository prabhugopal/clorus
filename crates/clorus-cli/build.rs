fn main() {
    // This binary embeds an LLVM MCJIT engine (via `clorus-codegen`/inkwell)
    // that JIT-compiles Clorus code directly into this same process. That
    // JIT-compiled code calls back into `clorus-runtime`'s `extern "C"
    // clorus_*` functions, which are statically linked into this very
    // executable (there is no separate `libclorus_runtime.so`).
    //
    // To resolve those calls, MCJIT's default symbol resolver (and the
    // explicit fallback in `clorus-repl`'s `bind_external_runtime_functions`,
    // and `LLVMLoadLibraryPermanently(NULL)` in `ensure_jit_configured_for_pie_host`)
    // ultimately go through the dynamic linker's symbol lookup
    // (`dlsym(RTLD_DEFAULT, "clorus_xyz")` or equivalent). On ELF/Linux, that
    // lookup only ever searches the *dynamic* symbol table (`.dynsym`) of the
    // executable and its loaded shared libraries. By default, `rustc`/`ld`
    // do NOT add a normal executable's own global symbols to `.dynsym` --
    // only a cdylib's or a shared library's symbols are exported that way.
    // So every `clorus_*` runtime function -- despite being `#[no_mangle]
    // pub extern "C" fn` and physically present in the binary -- resolves to
    // a NULL address at JIT symbol-resolution time. LLVM's JIT codegen does
    // not error out on this; it silently bakes the unresolved (null) address
    // into the compiled code as a literal immediate (`movabs $0x0, %rax` /
    // `call *%rax`), which segfaults the instant that call executes.
    //
    // `-rdynamic` (`--export-dynamic` at the linker level) tells the linker
    // to add this executable's own global symbols to `.dynsym` too, exactly
    // as a shared object would -- the standard, textbook requirement for
    // embedding a JIT that must call back into its own host process (the
    // same reason tools like LLVM's `lli` and other in-process JIT hosts
    // link this way). macOS's Mach-O/dyld does not have this restriction --
    // a PIE executable's global symbols are resolvable via `dlsym` there by
    // default -- which is why this class of bug never reproduces on macOS.
    //
    // Scoped to the actual link target (not the build-script's host OS) so
    // cross-compilation behaves correctly.
    let target = std::env::var("TARGET").unwrap_or_default();
    if target.contains("linux") {
        println!("cargo:rustc-link-arg=-rdynamic");
    }
}
