/// Integration test for Value* system
use clorus::*;
use clorus_runtime::value::{clorus_value_as_long, clorus_value_long, clorus_release, Value};
use inkwell::context::Context;
use inkwell::OptimizationLevel;

/// Load runtime library so JIT can find symbols
fn ensure_runtime_symbols() {
    // Touch runtime functions to ensure they're linked
    unsafe {
        let _test_val = clorus_value_long(42);
        clorus_release(_test_val);
    }
}

#[test]
fn test_value_star_arithmetic() {
    ensure_runtime_symbols();

    let context = Context::create();
    let mut codegen = CodeGen::new(&context, "test");

    // Test basic arithmetic: (+ 1 2)
    println!("Parsing expression...");
    let exprs = parse("(+ 1 2)").unwrap();
    println!("Wrapping in function...");
    let _func = codegen.wrap_in_function(&exprs[0], "test_add").unwrap();

    // Print LLVM IR for debugging
    println!("\nGenerated LLVM IR:");
    codegen.print_ir();

    println!("\nCreating JIT engine...");
    let engine = codegen.get_module()
        .create_jit_execution_engine(OptimizationLevel::None)
        .unwrap();

    unsafe {
        type TestFunc = unsafe extern "C" fn() -> *mut u8;
        println!("Getting function...");
        let jit_fn = engine.get_function::<TestFunc>("test_add").unwrap();
        println!("Calling function...");
        let result_ptr = jit_fn.call();
        println!("Got result pointer: {:p}", result_ptr);
        let result_val = result_ptr as *mut Value;
        println!("Converting to long...");
        let result = clorus_value_as_long(result_val);

        println!("Result: {}", result);
        assert_eq!(result, 3, "Expected 3, got {}", result);
    }
}

#[test]
fn test_value_star_string_literal() {
    ensure_runtime_symbols();

    let context = Context::create();
    let mut codegen = CodeGen::new(&context, "test");

    // Test string literal: "Hello"
    let exprs = parse(r#""Hello Clorus""#).unwrap();
    let _func = codegen.wrap_in_function(&exprs[0], "test_string").unwrap();

    let engine = codegen.get_module()
        .create_jit_execution_engine(OptimizationLevel::None)
        .unwrap();

    unsafe {
        type TestFunc = unsafe extern "C" fn() -> *mut u8;
        let jit_fn = engine.get_function::<TestFunc>("test_string").unwrap();
        let result_ptr = jit_fn.call();

        // Should return non-null pointer for string
        assert!(!result_ptr.is_null(), "String returned null pointer");
    }
}

#[test]
fn test_value_star_let_binding() {
    ensure_runtime_symbols();

    let context = Context::create();
    let mut codegen = CodeGen::new(&context, "test");

    // Test let with Value*: (let [x 10] x)
    let exprs = parse("(let [x 10] x)").unwrap();
    let _func = codegen.wrap_in_function(&exprs[0], "test_let").unwrap();

    let engine = codegen.get_module()
        .create_jit_execution_engine(OptimizationLevel::None)
        .unwrap();

    unsafe {
        type TestFunc = unsafe extern "C" fn() -> *mut u8;
        let jit_fn = engine.get_function::<TestFunc>("test_let").unwrap();
        let result_ptr = jit_fn.call();
        let result_val = result_ptr as *mut Value;
        let result = clorus_value_as_long(result_val);

        assert_eq!(result, 10, "Expected 10, got {}", result);
    }
}

#[test]
fn test_value_star_if_expression() {
    ensure_runtime_symbols();

    let context = Context::create();
    let mut codegen = CodeGen::new(&context, "test");

    // Test if with Value*: (if (< 5 10) 100 200)
    let exprs = parse("(if (< 5 10) 100 200)").unwrap();
    let _func = codegen.wrap_in_function(&exprs[0], "test_if").unwrap();

    let engine = codegen.get_module()
        .create_jit_execution_engine(OptimizationLevel::None)
        .unwrap();

    unsafe {
        type TestFunc = unsafe extern "C" fn() -> *mut u8;
        let jit_fn = engine.get_function::<TestFunc>("test_if").unwrap();
        let result_ptr = jit_fn.call();
        let result_val = result_ptr as *mut Value;
        let result = clorus_value_as_long(result_val);

        assert_eq!(result, 100, "Expected 100, got {}", result);
    }
}
