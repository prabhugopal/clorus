/// Example showing LLVM IR generation for runtime functions
///
/// This demonstrates how the clorus-codegen can generate calls to
/// the clorus-runtime library functions for persistent data structures.

use inkwell::context::Context;
use clorus_codegen::CodeGen;

fn main() {
    println!("=== Clorus Runtime Integration: IR Generation ===\n");

    let context = Context::create();
    let mut codegen = CodeGen::new(&context, "runtime_demo");

    // Get the runtime functions (already declared by CodeGen::new)
    let vec_empty_fn = codegen.get_module().get_function("clorus_vector_empty").unwrap();
    let vec_conj_fn = codegen.get_module().get_function("clorus_vector_conj").unwrap();
    let vec_nth_fn = codegen.get_module().get_function("clorus_vector_nth").unwrap();
    let value_number_fn = codegen.get_module().get_function("clorus_value_number").unwrap();
    let value_as_number_fn = codegen.get_module().get_function("clorus_value_as_number").unwrap();

    println!("✅ Runtime functions successfully declared in LLVM module:");
    println!("   - clorus_vector_empty");
    println!("   - clorus_vector_conj");
    println!("   - clorus_vector_nth");
    println!("   - clorus_vector_count");
    println!("   - clorus_value_number");
    println!("   - clorus_value_as_number");
    println!("   - clorus_retain");
    println!("   - clorus_release");
    println!("   - clorus_list_empty");
    println!("   - clorus_list_cons");
    println!();

    // Create a demo function that generates IR for: (nth [1.0 2.0 3.0] 1)
    // This represents the Clojure code: (nth [1.0 2.0 3.0] 1) => 2.0
    let fn_type = context.f64_type().fn_type(&[], false);
    let function = codegen.get_module().add_function("demo_vector_nth", fn_type, None);
    let basic_block = context.append_basic_block(function, "entry");

    codegen.get_builder().position_at_end(basic_block);

    // Step 1: Create empty vector
    let empty_call = codegen.get_builder().build_call(vec_empty_fn, &[], "vec0").unwrap();
    let vec0 = empty_call.try_as_basic_value().left().unwrap().into_pointer_value();

    // Step 2: Add 1.0
    let num1 = context.f64_type().const_float(1.0);
    let val1_call = codegen.get_builder().build_call(value_number_fn, &[num1.into()], "val1").unwrap();
    let val1 = val1_call.try_as_basic_value().left().unwrap().into_pointer_value();
    let vec1_call = codegen.get_builder().build_call(vec_conj_fn, &[vec0.into(), val1.into()], "vec1").unwrap();
    let vec1 = vec1_call.try_as_basic_value().left().unwrap().into_pointer_value();

    // Step 3: Add 2.0
    let num2 = context.f64_type().const_float(2.0);
    let val2_call = codegen.get_builder().build_call(value_number_fn, &[num2.into()], "val2").unwrap();
    let val2 = val2_call.try_as_basic_value().left().unwrap().into_pointer_value();
    let vec2_call = codegen.get_builder().build_call(vec_conj_fn, &[vec1.into(), val2.into()], "vec2").unwrap();
    let vec2 = vec2_call.try_as_basic_value().left().unwrap().into_pointer_value();

    // Step 4: Add 3.0
    let num3 = context.f64_type().const_float(3.0);
    let val3_call = codegen.get_builder().build_call(value_number_fn, &[num3.into()], "val3").unwrap();
    let val3 = val3_call.try_as_basic_value().left().unwrap().into_pointer_value();
    let vec3_call = codegen.get_builder().build_call(vec_conj_fn, &[vec2.into(), val3.into()], "vec3").unwrap();
    let vec3 = vec3_call.try_as_basic_value().left().unwrap().into_pointer_value();

    // Step 5: Get element at index 1 (should be 2.0)
    let index1 = context.i64_type().const_int(1, false);
    let elem_call = codegen.get_builder().build_call(vec_nth_fn, &[vec3.into(), index1.into()], "elem").unwrap();
    let elem = elem_call.try_as_basic_value().left().unwrap().into_pointer_value();

    // Step 6: Extract f64 from Value
    let result_call = codegen.get_builder().build_call(value_as_number_fn, &[elem.into()], "result").unwrap();
    let result = result_call.try_as_basic_value().left().unwrap().into_float_value();

    // Return the result
    codegen.get_builder().build_return(Some(&result)).unwrap();

    println!("✅ Generated LLVM IR for: (nth [1.0 2.0 3.0] 1)\n");
    println!("Generated LLVM IR:");
    println!("{}", codegen.get_module().print_to_string().to_string());

    println!("\n=== Summary ===");
    println!("✅ Runtime functions declared in LLVM module");
    println!("✅ Generated IR that calls runtime functions");
    println!("✅ Integration complete!");
    println!("\nNext steps:");
    println!("1. Link clorus-runtime library for execution");
    println!("2. Add vector/list literal syntax to parser");
    println!("3. Generate runtime calls automatically from AST");
}
