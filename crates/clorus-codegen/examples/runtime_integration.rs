/// Example demonstrating runtime integration with LLVM codegen
///
/// This shows how to generate LLVM IR that calls the clorus-runtime functions
/// for creating and manipulating persistent vectors.

use inkwell::context::Context;
use inkwell::execution_engine::JitFunction;
use inkwell::OptimizationLevel;
use clorus_codegen::CodeGen;

type TestFunc = unsafe extern "C" fn() -> f64;

fn main() {
    println!("=== Clorus Runtime Integration Example ===\n");

    let context = Context::create();
    let mut codegen = CodeGen::new(&context, "runtime_test");

    // Get the runtime functions
    let vec_empty_fn = codegen.get_module().get_function("clorus_vector_empty").unwrap();
    let vec_conj_fn = codegen.get_module().get_function("clorus_vector_conj").unwrap();
    let vec_nth_fn = codegen.get_module().get_function("clorus_vector_nth").unwrap();
    let vec_count_fn = codegen.get_module().get_function("clorus_vector_count").unwrap();
    let value_number_fn = codegen.get_module().get_function("clorus_value_number").unwrap();
    let value_as_number_fn = codegen.get_module().get_function("clorus_value_as_number").unwrap();
    let release_fn = codegen.get_module().get_function("clorus_release").unwrap();

    // Create a test function that:
    // 1. Creates an empty vector
    // 2. Adds numbers 1.0, 2.0, 3.0 to it
    // 3. Retrieves element at index 1 (should be 2.0)
    // 4. Returns the value
    let fn_type = context.f64_type().fn_type(&[], false);
    let function = codegen.get_module().add_function("test_vector", fn_type, None);
    let basic_block = context.append_basic_block(function, "entry");

    codegen.get_builder().position_at_end(basic_block);

    // Create empty vector: vec = clorus_vector_empty()
    let empty_call = codegen.get_builder().build_call(vec_empty_fn, &[], "vec0").unwrap();
    let vec0 = empty_call.try_as_basic_value().left().unwrap().into_pointer_value();

    // Create Value for 1.0
    let num1 = context.f64_type().const_float(1.0);
    let val1_call = codegen.get_builder().build_call(value_number_fn, &[num1.into()], "val1").unwrap();
    let val1 = val1_call.try_as_basic_value().left().unwrap().into_pointer_value();

    // vec1 = clorus_vector_conj(vec0, val1)
    let vec1_call = codegen.get_builder().build_call(
        vec_conj_fn,
        &[vec0.into(), val1.into()],
        "vec1"
    ).unwrap();
    let vec1 = vec1_call.try_as_basic_value().left().unwrap().into_pointer_value();

    // Create Value for 2.0
    let num2 = context.f64_type().const_float(2.0);
    let val2_call = codegen.get_builder().build_call(value_number_fn, &[num2.into()], "val2").unwrap();
    let val2 = val2_call.try_as_basic_value().left().unwrap().into_pointer_value();

    // vec2 = clorus_vector_conj(vec1, val2)
    let vec2_call = codegen.get_builder().build_call(
        vec_conj_fn,
        &[vec1.into(), val2.into()],
        "vec2"
    ).unwrap();
    let vec2 = vec2_call.try_as_basic_value().left().unwrap().into_pointer_value();

    // Create Value for 3.0
    let num3 = context.f64_type().const_float(3.0);
    let val3_call = codegen.get_builder().build_call(value_number_fn, &[num3.into()], "val3").unwrap();
    let val3 = val3_call.try_as_basic_value().left().unwrap().into_pointer_value();

    // vec3 = clorus_vector_conj(vec2, val3)
    let vec3_call = codegen.get_builder().build_call(
        vec_conj_fn,
        &[vec2.into(), val3.into()],
        "vec3"
    ).unwrap();
    let vec3 = vec3_call.try_as_basic_value().left().unwrap().into_pointer_value();

    // Get element at index 1: elem = clorus_vector_nth(vec3, 1)
    let index1 = context.i64_type().const_int(1, false);
    let elem_call = codegen.get_builder().build_call(
        vec_nth_fn,
        &[vec3.into(), index1.into()],
        "elem"
    ).unwrap();
    let elem = elem_call.try_as_basic_value().left().unwrap().into_pointer_value();

    // Extract f64 from elem: result = clorus_value_as_number(elem)
    let result_call = codegen.get_builder().build_call(
        value_as_number_fn,
        &[elem.into()],
        "result"
    ).unwrap();
    let result = result_call.try_as_basic_value().left().unwrap().into_float_value();

    // Clean up: release the vectors and values
    // (In real code, this would be more sophisticated with proper lifetimes)
    codegen.get_builder().build_call(release_fn, &[vec0.into()], "").unwrap();
    codegen.get_builder().build_call(release_fn, &[vec1.into()], "").unwrap();
    codegen.get_builder().build_call(release_fn, &[vec2.into()], "").unwrap();
    codegen.get_builder().build_call(release_fn, &[vec3.into()], "").unwrap();
    codegen.get_builder().build_call(release_fn, &[val1.into()], "").unwrap();
    codegen.get_builder().build_call(release_fn, &[val2.into()], "").unwrap();
    codegen.get_builder().build_call(release_fn, &[val3.into()], "").unwrap();
    codegen.get_builder().build_call(release_fn, &[elem.into()], "").unwrap();

    // Return the result
    codegen.get_builder().build_return(Some(&result)).unwrap();

    // Print the generated LLVM IR
    println!("Generated LLVM IR:");
    println!("{}\n", codegen.get_module().print_to_string().to_string());

    // Execute the function with JIT
    println!("Creating JIT execution engine...");
    let execution_engine = codegen.get_module()
        .create_jit_execution_engine(OptimizationLevel::None)
        .expect("Failed to create JIT engine");

    println!("Executing test_vector function...");
    unsafe {
        let jit_function: JitFunction<TestFunc> = execution_engine
            .get_function("test_vector")
            .expect("Function not found");

        let result = jit_function.call();
        println!("Result: {}", result);
        println!("\n✅ Expected: 2.0 (element at index 1 of [1.0, 2.0, 3.0])");

        if (result - 2.0).abs() < 0.001 {
            println!("✅ Test PASSED!");
        } else {
            println!("❌ Test FAILED! Expected 2.0, got {}", result);
        }
    }
}
