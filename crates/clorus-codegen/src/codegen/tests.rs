    use super::*;
    use clorus_syntax::parse;

    #[test]
    fn test_compile_number() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test");

        let expr = Expr::Double(42.0);

        // Wrap in function to provide basic block context
        let result = codegen.wrap_in_function(&expr, "test_number");
        assert!(result.is_ok());
    }

    #[test]
    fn test_compile_add() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test");

        let exprs = parse("(+ 1 2)").unwrap();

        // Wrap in function to provide basic block context
        let result = codegen.wrap_in_function(&exprs[0], "test_add");
        assert!(result.is_ok());
    }

    #[test]
    fn test_compile_nested() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test");

        let exprs = parse("(+ (* 2 3) 4)").unwrap();

        // Wrap in function to provide basic block context
        let result = codegen.wrap_in_function(&exprs[0], "test_nested");
        assert!(result.is_ok());
    }

    #[test]
    fn test_compile_fn_basic() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test");

        // Parse: (fn [x] x)
        let exprs = parse("(fn [x] x)").unwrap();

        // Wrap in function to provide basic block context
        let result = codegen.wrap_in_function(&exprs[0], "test_fn_basic");
        assert!(result.is_ok());

        // Verify a lambda function was created
        assert!(codegen.functions.keys().any(|k| k.ends_with("_lambda_0")));
    }

    #[test]
    fn test_compile_fn_with_body() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test");

        // Parse: (fn [x y] (+ x y))
        let exprs = parse("(fn [x y] (+ x y))").unwrap();

        // Wrap in function to provide basic block context
        let result = codegen.wrap_in_function(&exprs[0], "test_fn_body");
        assert!(result.is_ok());

        // Verify a lambda function was created
        assert!(codegen.functions.keys().any(|k| k.ends_with("_lambda_0")));
    }

    #[test]
    fn test_compile_fn_no_params() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test");

        // Parse: (fn [] 42)
        let exprs = parse("(fn [] 42)").unwrap();

        // Wrap in function to provide basic block context
        let result = codegen.wrap_in_function(&exprs[0], "test_fn_no_params");
        assert!(result.is_ok());

        // Verify a lambda function was created
        assert!(codegen.functions.keys().any(|k| k.ends_with("_lambda_0")));
    }

    #[test]
    fn test_rust_library_resolution_hyphen_underscore_prefix_variants() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test");

        codegen.register_rust_library(RustLibrary {
            name: "egui-hello".to_string(),
            functions: vec![],
        });

        assert!(codegen.resolve_rust_library("egui-hello").is_some());
        assert!(codegen.resolve_rust_library("egui_hello").is_some());
        assert!(codegen.resolve_rust_library("rust.egui-hello").is_some());
        assert!(codegen.resolve_rust_library("rust.egui_hello").is_some());
    }

    #[test]
    fn test_rust_library_resolution_when_registered_with_prefixed_name() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test");

        codegen.register_rust_library(RustLibrary {
            name: "rust.coral-gfx".to_string(),
            functions: vec![],
        });

        assert!(codegen.resolve_rust_library("coral-gfx").is_some());
        assert!(codegen.resolve_rust_library("coral_gfx").is_some());
        assert!(codegen.resolve_rust_library("rust.coral-gfx").is_some());
        assert!(codegen.resolve_rust_library("rust.coral_gfx").is_some());
    }

    #[test]
    fn test_use_rust_module_accepts_hyphen_and_underscore_variants() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test");

        codegen.register_rust_library(RustLibrary {
            name: "coral-gfx".to_string(),
            functions: vec![],
        });

        let exprs = parse("(use rust.coral_gfx)").unwrap();
        let result = codegen.wrap_in_function(&exprs[0], "test_use_rust_coral_gfx_underscore");
        assert!(result.is_ok(), "unexpected error: {:?}", result.err());

        let exprs = parse("(use rust.coral-gfx)").unwrap();
        let result = codegen.wrap_in_function(&exprs[0], "test_use_rust_coral_gfx_hyphen");
        assert!(result.is_ok(), "unexpected error: {:?}", result.err());
    }

    #[test]
    fn test_use_rust_module_accepts_prefixed_library_registration() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test");

        codegen.register_rust_library(RustLibrary {
            name: "rust.coral-gfx".to_string(),
            functions: vec![],
        });

        let exprs = parse("(use rust.coral_gfx)").unwrap();
        let result =
            codegen.wrap_in_function(&exprs[0], "test_use_rust_coral_gfx_prefixed_library");
        assert!(result.is_ok(), "unexpected error: {:?}", result.err());
    }

    #[test]
    fn test_rust_ffi_symbol_name_is_dependency_scoped() {
        let symbol = CodeGen::rust_ffi_symbol_name("example-rust-lib", "add");
        assert_eq!(symbol, "clorus_example_rust_lib__add");
    }

    #[test]
    fn test_rust_ffi_symbol_name_normalizes_punctuation() {
        let symbol = CodeGen::rust_ffi_symbol_name("demo.lib-name", "foo-bar?");
        assert_eq!(symbol, "clorus_demo_lib_name__foo_bar_");
    }

    #[test]
    fn test_declare_rust_library_functions_does_not_collide_with_core_add() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test");

        // Runtime intrinsic symbol should already exist.
        assert!(codegen.module.get_function("clorus_add").is_some());

        let lib = RustLibrary {
            name: "example-rust-lib".to_string(),
            functions: vec![RustFunction {
                name: "add".to_string(),
                params: vec![
                    RustParam {
                        name: "a".to_string(),
                        type_name: "f64".to_string(),
                    },
                    RustParam {
                        name: "b".to_string(),
                        type_name: "f64".to_string(),
                    },
                ],
                return_type: "f64".to_string(),
            }],
        };

        let result = codegen.declare_rust_library_functions(&lib);
        assert!(result.is_ok(), "unexpected error: {:?}", result.err());
        assert!(
            codegen
                .module
                .get_function("clorus_example_rust_lib__add")
                .is_some()
        );
        // Core intrinsic remains intact.
        assert!(codegen.module.get_function("clorus_add").is_some());
    }

    #[test]
    fn test_compile_namespaced_rust_add_uses_dependency_scoped_symbol() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test");

        let lib = RustLibrary {
            name: "example-rust-lib".to_string(),
            functions: vec![RustFunction {
                name: "add".to_string(),
                params: vec![
                    RustParam {
                        name: "a".to_string(),
                        type_name: "f64".to_string(),
                    },
                    RustParam {
                        name: "b".to_string(),
                        type_name: "f64".to_string(),
                    },
                ],
                return_type: "f64".to_string(),
            }],
        };

        codegen.register_rust_library(lib.clone());
        codegen
            .declare_rust_library_functions(&lib)
            .expect("declare rust ffi symbols");

        let exprs = parse("(example-rust-lib/add 1 2)").expect("parse expression");
        codegen
            .wrap_in_function(&exprs[0], "test_rust_namespaced_add")
            .expect("compile rust namespaced call");

        let ir = codegen.module.print_to_string().to_string();
        assert!(
            ir.contains("clorus_example_rust_lib__add"),
            "IR should call dependency-scoped symbol, got:\n{}",
            ir
        );
    }

    #[test]
    fn test_compile_namespaced_rust_add_underscore_variant_uses_same_scoped_symbol() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test");

        let lib = RustLibrary {
            name: "example-rust-lib".to_string(),
            functions: vec![RustFunction {
                name: "add".to_string(),
                params: vec![
                    RustParam {
                        name: "a".to_string(),
                        type_name: "f64".to_string(),
                    },
                    RustParam {
                        name: "b".to_string(),
                        type_name: "f64".to_string(),
                    },
                ],
                return_type: "f64".to_string(),
            }],
        };

        codegen.register_rust_library(lib.clone());
        codegen
            .declare_rust_library_functions(&lib)
            .expect("declare rust ffi symbols");

        // Underscore namespace variant should resolve to the same registered rust library.
        let exprs = parse("(example_rust_lib/add 1 2)").expect("parse expression");
        codegen
            .wrap_in_function(&exprs[0], "test_rust_namespaced_add_underscore")
            .expect("compile rust namespaced call");

        let ir = codegen.module.print_to_string().to_string();
        assert!(
            ir.contains("clorus_example_rust_lib__add"),
            "IR should call dependency-scoped symbol, got:\n{}",
            ir
        );
    }

    #[test]
    fn test_compile_prefixed_rust_namespace_call_uses_scoped_symbol() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test");

        let lib = RustLibrary {
            name: "example-rust-lib".to_string(),
            functions: vec![RustFunction {
                name: "add".to_string(),
                params: vec![
                    RustParam {
                        name: "a".to_string(),
                        type_name: "f64".to_string(),
                    },
                    RustParam {
                        name: "b".to_string(),
                        type_name: "f64".to_string(),
                    },
                ],
                return_type: "f64".to_string(),
            }],
        };

        codegen.register_rust_library(lib.clone());
        codegen
            .declare_rust_library_functions(&lib)
            .expect("declare rust ffi symbols");

        // Fully-prefixed rust namespace variant should resolve identically.
        let exprs = parse("(rust.example-rust-lib/add 1 2)").expect("parse expression");
        codegen
            .wrap_in_function(&exprs[0], "test_rust_prefixed_namespace_add")
            .expect("compile prefixed rust namespaced call");

        let ir = codegen.module.print_to_string().to_string();
        assert!(
            ir.contains("clorus_example_rust_lib__add"),
            "IR should call dependency-scoped symbol, got:\n{}",
            ir
        );
    }

    #[test]
    fn test_compile_prefixed_underscore_rust_namespace_call_uses_scoped_symbol() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test");

        let lib = RustLibrary {
            name: "example-rust-lib".to_string(),
            functions: vec![RustFunction {
                name: "add".to_string(),
                params: vec![
                    RustParam {
                        name: "a".to_string(),
                        type_name: "f64".to_string(),
                    },
                    RustParam {
                        name: "b".to_string(),
                        type_name: "f64".to_string(),
                    },
                ],
                return_type: "f64".to_string(),
            }],
        };

        codegen.register_rust_library(lib.clone());
        codegen
            .declare_rust_library_functions(&lib)
            .expect("declare rust ffi symbols");

        // rust.<lib> namespace with underscore variant should resolve identically.
        let exprs = parse("(rust.example_rust_lib/add 1 2)").expect("parse expression");
        codegen
            .wrap_in_function(&exprs[0], "test_rust_prefixed_namespace_add_underscore")
            .expect("compile prefixed underscore rust namespaced call");

        let ir = codegen.module.print_to_string().to_string();
        assert!(
            ir.contains("clorus_example_rust_lib__add"),
            "IR should call dependency-scoped symbol, got:\n{}",
            ir
        );
    }

    #[test]
    fn test_declare_rust_library_functions_supports_extended_numeric_types() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test");

        let lib = RustLibrary {
            name: "mathx".to_string(),
            functions: vec![
                RustFunction {
                    name: "f32_id".to_string(),
                    params: vec![RustParam {
                        name: "x".to_string(),
                        type_name: "f32".to_string(),
                    }],
                    return_type: "f32".to_string(),
                },
                RustFunction {
                    name: "u32_id".to_string(),
                    params: vec![RustParam {
                        name: "x".to_string(),
                        type_name: "u32".to_string(),
                    }],
                    return_type: "u32".to_string(),
                },
                RustFunction {
                    name: "u64_id".to_string(),
                    params: vec![RustParam {
                        name: "x".to_string(),
                        type_name: "u64".to_string(),
                    }],
                    return_type: "u64".to_string(),
                },
                RustFunction {
                    name: "isize_id".to_string(),
                    params: vec![RustParam {
                        name: "x".to_string(),
                        type_name: "isize".to_string(),
                    }],
                    return_type: "isize".to_string(),
                },
                RustFunction {
                    name: "usize_id".to_string(),
                    params: vec![RustParam {
                        name: "x".to_string(),
                        type_name: "usize".to_string(),
                    }],
                    return_type: "usize".to_string(),
                },
                RustFunction {
                    name: "const_ptr_id".to_string(),
                    params: vec![RustParam {
                        name: "x".to_string(),
                        type_name: "*const u8".to_string(),
                    }],
                    return_type: "*const u8".to_string(),
                },
            ],
        };

        let result = codegen.declare_rust_library_functions(&lib);
        assert!(result.is_ok(), "unexpected error: {:?}", result.err());
        for fn_name in [
            "f32_id",
            "u32_id",
            "u64_id",
            "isize_id",
            "usize_id",
            "const_ptr_id",
        ] {
            assert!(
                codegen
                    .module
                    .get_function(&format!("clorus_mathx__{}", fn_name))
                    .is_some()
            );
        }
    }

    #[test]
    fn test_compile_multiple_fn() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test");

        // Parse multiple lambda functions
        let exprs1 = parse("(fn [x] (* x 2))").unwrap();
        let exprs2 = parse("(fn [y] (+ y 1))").unwrap();

        // Compile both
        let result1 = codegen.wrap_in_function(&exprs1[0], "test_fn1");
        assert!(result1.is_ok());

        let result2 = codegen.wrap_in_function(&exprs2[0], "test_fn2");
        assert!(result2.is_ok());

        // Verify two different lambda functions were created
        assert!(codegen.functions.keys().any(|k| k.ends_with("_lambda_0")));
        assert!(codegen.functions.keys().any(|k| k.ends_with("_lambda_1")));
    }

    #[test]
    fn test_namespace_alias_conflict_errors() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test");

        let exprs = parse(
            "(ns tests.alias-conflict
               (:require [clorus.set :as s]
                         [clorus.string :as s]))",
        )
        .unwrap();

        let result = codegen.wrap_in_function(&exprs[0], "test_ns_alias_conflict");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("Namespace alias conflict"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn test_namespace_import_conflict_errors() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test");

        let exprs = parse(
            "(ns tests.import-conflict
               (:require [clorus.set :rename {union shared}]
                         [clorus.string :rename {join shared}]))",
        )
        .unwrap();

        let result = codegen.wrap_in_function(&exprs[0], "test_ns_import_conflict");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("Namespace import conflict"),
            "unexpected error: {}",
            err
        );
    }
