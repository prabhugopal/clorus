use super::*;

impl<'ctx> CodeGen<'ctx> {
    /// Resolves a Rust-interop method/associated-function call written as
    /// `lib.TypeName/method` (e.g. `(mylib.Counter/bump c)`), where
    /// `namespace_or_alias` is everything before the final `/` in the call
    /// target and `func_name` is everything after it.
    ///
    /// Clorus reuses the same `lib/fn` call grammar used for free functions:
    /// a dot inside the namespace segment is already legal there, so no new
    /// syntax is needed. `TypeName::method_name` is itself a valid Rust UFCS
    /// call path -- `Counter::bump(&mut c)` is exactly `c.bump()` -- so the
    /// analyzer records impl-block methods under that qualified name (see
    /// `clorus_ffi_gen::FfiGenerator::extract_impl_methods`), and this
    /// function just needs to reconstruct that same name from the call site
    /// and hand it to the existing free-function call codegen unchanged.
    ///
    /// Returns `None` when `namespace_or_alias` doesn't contain a `.` at
    /// all, or the part before it isn't a registered Rust library -- in
    /// either case this isn't a type-qualified Rust call, and the caller
    /// should fall through to whatever else `namespace_or_alias` might be
    /// (a Clorus namespace, a .clip package, a plain Rust library, etc.).
    pub(super) fn try_compile_rust_type_method_call(
        &mut self,
        namespace_or_alias: &str,
        func_name: &str,
        args: &[Expr],
    ) -> Option<Result<PointerValue<'ctx>, String>> {
        let dot_pos = namespace_or_alias.rfind('.')?;
        let lib_prefix = &namespace_or_alias[..dot_pos];
        let type_name = &namespace_or_alias[dot_pos + 1..];
        if lib_prefix.is_empty() || type_name.is_empty() {
            return None;
        }

        let rust_lib = self.resolve_rust_library(lib_prefix).or_else(|| {
            self.namespace
                .aliases
                .get(lib_prefix)
                .and_then(|resolved| self.resolve_rust_library(resolved))
        })?;

        let method_call_name = format!("{}::{}", type_name, func_name.replace('-', "_"));
        Some(self.compile_rust_library_call(&rust_lib, &method_call_name, args))
    }
}
