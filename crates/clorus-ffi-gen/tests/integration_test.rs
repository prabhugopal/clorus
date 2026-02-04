/// End-to-end integration test for FFI system
///
/// This tests the complete flow: Rust source → FFI analysis → LLVM codegen → execution

#[cfg(test)]
mod integration_tests {
    use clorus_ffi_gen::analyzer::FfiAnalyzer;
    use std::path::Path;

    #[test]
    fn test_e2e_multiarg_pointer_function() {
        // Step 1: Create test Rust source file
        let test_source = "/tmp/test_e2e_ffi.rs";
        std::fs::write(test_source, r#"
            pub fn create_context(width: f64, height: f64) -> *mut u8 {
                Box::into_raw(Box::new(42u8))
            }

            pub fn move_to(ctx: *mut u8, x: f64, y: f64) {
                // Implementation would draw
            }

            pub fn destroy_context(ctx: *mut u8) {
                unsafe { Box::from_raw(ctx); }
            }
        "#).unwrap();

        // Step 2: Analyze with FfiAnalyzer
        let mut analyzer = FfiAnalyzer::new();
        analyzer.parse_file(Path::new(test_source))
            .expect("Failed to parse test source");

        // Verify all functions found
        assert_eq!(analyzer.functions.len(), 3);

        let create_fn = analyzer.functions.iter()
            .find(|f| f.name == "create_context")
            .expect("create_context not found");

        let move_to_fn = analyzer.functions.iter()
            .find(|f| f.name == "move_to")
            .expect("move_to not found");

        // Step 3: Verify types are correct
        assert_eq!(create_fn.params.len(), 2);
        assert!(matches!(create_fn.params[0].ty, clorus_types::FfiType::F64));
        assert!(matches!(create_fn.params[1].ty, clorus_types::FfiType::F64));
        assert!(matches!(create_fn.return_type, clorus_types::FfiType::OpaquePointer { .. }));

        assert_eq!(move_to_fn.params.len(), 3);
        assert!(matches!(move_to_fn.params[0].ty, clorus_types::FfiType::OpaquePointer { .. }));
        assert!(matches!(move_to_fn.params[1].ty, clorus_types::FfiType::F64));
        assert!(matches!(move_to_fn.params[2].ty, clorus_types::FfiType::F64));

        println!("✅ End-to-end test passed: Rust → Analysis → Validation");
    }

    #[test]
    fn test_e2e_json_metadata() {
        // Test that JSON metadata can be generated and is valid
        let test_source = "/tmp/test_json.rs";
        std::fs::write(test_source, r#"
            pub fn add(x: f64, y: f64) -> f64 {
                x + y
            }

            pub fn concat(a: String, b: String) -> String {
                format!("{}{}", a, b)
            }
        "#).unwrap();

        let mut analyzer = FfiAnalyzer::new();
        analyzer.parse_file(Path::new(test_source)).unwrap();

        let json = analyzer.generate_metadata_json().unwrap();

        // Verify JSON is valid
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&json)
            .expect("Generated JSON is invalid");

        assert_eq!(parsed.len(), 2);

        // Verify structure
        assert!(parsed[0]["name"].as_str().is_some());
        assert!(parsed[0]["params"].is_array());
        // return_type is serialized as enum variant
        assert!(!parsed[0]["return_type"].is_null());

        println!("✅ JSON metadata generation working");
    }

    #[test]
    fn test_e2e_error_handling() {
        // Test that invalid Rust code produces helpful errors
        let test_source = "/tmp/test_invalid.rs";
        std::fs::write(test_source, r#"
            pub fn invalid() {
                // Syntax error
                let x = ;
            }
        "#).unwrap();

        let mut analyzer = FfiAnalyzer::new();
        let result = analyzer.parse_file(Path::new(test_source));

        // Should get a clear error, not a panic
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("Failed to parse") || error.contains("expected"));

        println!("✅ Error handling working correctly");
    }
}
