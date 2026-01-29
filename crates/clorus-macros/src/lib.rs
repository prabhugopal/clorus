/// Clorus Macros - Auto-wrapping Rust APIs for Clorus
///
/// This crate provides procedural macros to automatically generate
/// C-compatible FFI wrappers for Rust functions.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Ident, ItemFn, ReturnType, Type};

/// Wrap a Rust function for use in Clorus
///
/// Example:
/// ```rust
/// #[export_to_clorus]
/// pub fn add(a: i32, b: i32) -> i32 {
///     a + b
/// }
/// ```
///
/// Generates a C-compatible wrapper:
/// ```rust
/// #[no_mangle]
/// pub extern "C" fn clorus_add(a: i32, b: i32) -> i32 {
///     add(a, b)
/// }
/// ```
#[proc_macro_attribute]
pub fn export_to_clorus(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);

    let fn_name = &input_fn.sig.ident;
    let wrapper_name = Ident::new(&format!("clorus_{}", fn_name), fn_name.span());

    let inputs = &input_fn.sig.inputs;
    let output = &input_fn.sig.output;
    let block = &input_fn.block;

    // Extract parameter names for forwarding
    let param_names: Vec<_> = input_fn.sig.inputs.iter().filter_map(|arg| {
        if let syn::FnArg::Typed(pat_type) = arg {
            if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                return Some(&pat_ident.ident);
            }
        }
        None
    }).collect();

    // Generate the wrapper
    let expanded = quote! {
        // Keep original function
        #input_fn

        // Generate C-compatible wrapper
        #[no_mangle]
        pub extern "C" fn #wrapper_name(#inputs) #output {
            #fn_name(#(#param_names),*)
        }
    };

    TokenStream::from(expanded)
}

/// Simple macro for now - will expand later
///
/// Usage:
/// ```rust
/// wrap_rust_module! {
///     pub fn read_to_string(path: String) -> String;
///     pub fn write(path: String, content: String) -> ();
/// }
/// ```
#[proc_macro]
pub fn wrap_rust_module(input: TokenStream) -> TokenStream {
    // For now, just return empty
    // We'll implement the full parser in next iteration
    TokenStream::new()
}
