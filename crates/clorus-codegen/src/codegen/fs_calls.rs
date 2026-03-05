use super::*;

impl<'ctx> CodeGen<'ctx> {
    /// Handles both string literals and string variables (Value*)
    pub(super) fn compile_string_to_ptr(&mut self, expr: &Expr) -> Result<PointerValue<'ctx>, String> {
        match expr {
            Expr::String(s) => {
                // Create global string constant (null-terminated C string)
                let c_str = self.builder.build_global_string_ptr(s, "str").unwrap();
                Ok(c_str.as_pointer_value())
            }
            Expr::Symbol(_) => {
                // Variable containing a Value* (which may be a string)
                // Compile to get the Value*, then extract C string from it
                let value_ptr = self.compile_expr(expr)?;
                Ok(self.extract_cstring_from_value(value_ptr))
            }
            _ => Err("Expected string literal or string variable".to_string()),
        }
    }

    /// Compile fs/* function calls (rust.fs module functions)

    /// Compile fs/* function calls (rust.fs module functions)
    pub(super) fn compile_fs_call(&mut self, func: &str, args: &[Expr]) -> Result<PointerValue<'ctx>, String> {
        match func {
            "fs/read" => {
                // fs/read takes 1 arg: path (string)
                if args.len() != 1 {
                    return Err("fs/read requires 1 argument: path".to_string());
                }

                let path_ptr = self.compile_string_to_ptr(&args[0])?;

                let fs_read_fn = self
                    .module
                    .get_function("clorus_fs_read")
                    .ok_or("fs/read not declared. Did you forget (use rust.fs)?")?;

                let result = self
                    .builder
                    .build_call(fs_read_fn, &[path_ptr.into()], "fs_read_call")
                    .unwrap();

                // fs/read returns *mut c_char (string pointer)
                // Box it into a Value* string
                let str_ptr = result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();
                Ok(self.box_string(str_ptr))
            }

            "fs/write" => {
                // fs/write takes 2 args: path (string), content (string)
                if args.len() != 2 {
                    return Err("fs/write requires 2 arguments: path, content".to_string());
                }

                let path_ptr = self.compile_string_to_ptr(&args[0])?;
                let content_ptr = self.compile_string_to_ptr(&args[1])?;

                let fs_write_fn = self
                    .module
                    .get_function("clorus_fs_write")
                    .ok_or("fs/write not declared. Did you forget (use rust.fs)?")?;

                let result = self
                    .builder
                    .build_call(
                        fs_write_fn,
                        &[path_ptr.into(), content_ptr.into()],
                        "fs_write_call",
                    )
                    .unwrap();

                // fs/write returns i32 (1 = success, 0 = failure)
                // Convert to f64 and box as number
                let i32_result = result.try_as_basic_value().left().unwrap().into_int_value();
                let float_result = self
                    .builder
                    .build_unsigned_int_to_float(
                        i32_result,
                        self.context.f64_type(),
                        "i32_to_float",
                    )
                    .unwrap();

                Ok(self.box_number(float_result))
            }

            "fs/append" => {
                if args.len() != 2 {
                    return Err("fs/append requires 2 arguments: path, content".to_string());
                }

                let path_ptr = self.compile_string_to_ptr(&args[0])?;
                let content_ptr = self.compile_string_to_ptr(&args[1])?;

                let fs_append_fn = self
                    .module
                    .get_function("clorus_fs_append")
                    .ok_or("fs/append not declared. Did you forget (use rust.fs)?")?;

                let result = self
                    .builder
                    .build_call(
                        fs_append_fn,
                        &[path_ptr.into(), content_ptr.into()],
                        "fs_append_call",
                    )
                    .unwrap();

                let i32_result = result.try_as_basic_value().left().unwrap().into_int_value();
                let float_result = self
                    .builder
                    .build_unsigned_int_to_float(
                        i32_result,
                        self.context.f64_type(),
                        "i32_to_float",
                    )
                    .unwrap();

                Ok(self.box_number(float_result))
            }

            "fs/exists?" => {
                if args.len() != 1 {
                    return Err("fs/exists? requires 1 argument: path".to_string());
                }

                let path_ptr = self.compile_string_to_ptr(&args[0])?;

                let fs_exists_fn = self
                    .module
                    .get_function("clorus_fs_exists")
                    .ok_or("fs/exists? not declared. Did you forget (use rust.fs)?")?;

                let result = self
                    .builder
                    .build_call(fs_exists_fn, &[path_ptr.into()], "fs_exists_call")
                    .unwrap();

                let i32_result = result.try_as_basic_value().left().unwrap().into_int_value();
                let float_result = self
                    .builder
                    .build_unsigned_int_to_float(
                        i32_result,
                        self.context.f64_type(),
                        "i32_to_float",
                    )
                    .unwrap();

                Ok(self.box_number(float_result))
            }

            "fs/is-file?" => {
                if args.len() != 1 {
                    return Err("fs/is-file? requires 1 argument: path".to_string());
                }

                let path_ptr = self.compile_string_to_ptr(&args[0])?;

                let fs_is_file_fn = self
                    .module
                    .get_function("clorus_fs_is_file")
                    .ok_or("fs/is-file? not declared. Did you forget (use rust.fs)?")?;

                let result = self
                    .builder
                    .build_call(fs_is_file_fn, &[path_ptr.into()], "fs_is_file_call")
                    .unwrap();

                let i32_result = result.try_as_basic_value().left().unwrap().into_int_value();
                let float_result = self
                    .builder
                    .build_unsigned_int_to_float(
                        i32_result,
                        self.context.f64_type(),
                        "i32_to_float",
                    )
                    .unwrap();

                Ok(self.box_number(float_result))
            }

            "fs/is-dir?" => {
                if args.len() != 1 {
                    return Err("fs/is-dir? requires 1 argument: path".to_string());
                }

                let path_ptr = self.compile_string_to_ptr(&args[0])?;

                let fs_is_dir_fn = self
                    .module
                    .get_function("clorus_fs_is_dir")
                    .ok_or("fs/is-dir? not declared. Did you forget (use rust.fs)?")?;

                let result = self
                    .builder
                    .build_call(fs_is_dir_fn, &[path_ptr.into()], "fs_is_dir_call")
                    .unwrap();

                let i32_result = result.try_as_basic_value().left().unwrap().into_int_value();
                let float_result = self
                    .builder
                    .build_unsigned_int_to_float(
                        i32_result,
                        self.context.f64_type(),
                        "i32_to_float",
                    )
                    .unwrap();

                Ok(self.box_number(float_result))
            }

            "fs/remove" => {
                if args.len() != 1 {
                    return Err("fs/remove requires 1 argument: path".to_string());
                }

                let path_ptr = self.compile_string_to_ptr(&args[0])?;

                let fs_remove_fn = self
                    .module
                    .get_function("clorus_fs_remove")
                    .ok_or("fs/remove not declared. Did you forget (use rust.fs)?")?;

                let result = self
                    .builder
                    .build_call(fs_remove_fn, &[path_ptr.into()], "fs_remove_call")
                    .unwrap();

                let i32_result = result.try_as_basic_value().left().unwrap().into_int_value();
                let float_result = self
                    .builder
                    .build_unsigned_int_to_float(
                        i32_result,
                        self.context.f64_type(),
                        "i32_to_float",
                    )
                    .unwrap();

                Ok(self.box_number(float_result))
            }

            "fs/copy" => {
                if args.len() != 2 {
                    return Err("fs/copy requires 2 arguments: src, dst".to_string());
                }

                let src_ptr = self.compile_string_to_ptr(&args[0])?;
                let dst_ptr = self.compile_string_to_ptr(&args[1])?;

                let fs_copy_fn = self
                    .module
                    .get_function("clorus_fs_copy")
                    .ok_or("fs/copy not declared. Did you forget (use rust.fs)?")?;

                let result = self
                    .builder
                    .build_call(
                        fs_copy_fn,
                        &[src_ptr.into(), dst_ptr.into()],
                        "fs_copy_call",
                    )
                    .unwrap();

                let i32_result = result.try_as_basic_value().left().unwrap().into_int_value();
                let float_result = self
                    .builder
                    .build_unsigned_int_to_float(
                        i32_result,
                        self.context.f64_type(),
                        "i32_to_float",
                    )
                    .unwrap();

                Ok(self.box_number(float_result))
            }

            "fs/rename" => {
                if args.len() != 2 {
                    return Err("fs/rename requires 2 arguments: old, new".to_string());
                }

                let old_ptr = self.compile_string_to_ptr(&args[0])?;
                let new_ptr = self.compile_string_to_ptr(&args[1])?;

                let fs_rename_fn = self
                    .module
                    .get_function("clorus_fs_rename")
                    .ok_or("fs/rename not declared. Did you forget (use rust.fs)?")?;

                let result = self
                    .builder
                    .build_call(
                        fs_rename_fn,
                        &[old_ptr.into(), new_ptr.into()],
                        "fs_rename_call",
                    )
                    .unwrap();

                let i32_result = result.try_as_basic_value().left().unwrap().into_int_value();
                let float_result = self
                    .builder
                    .build_unsigned_int_to_float(
                        i32_result,
                        self.context.f64_type(),
                        "i32_to_float",
                    )
                    .unwrap();

                Ok(self.box_number(float_result))
            }

            "fs/create-dir" => {
                if args.len() != 1 {
                    return Err("fs/create-dir requires 1 argument: path".to_string());
                }

                let path_ptr = self.compile_string_to_ptr(&args[0])?;

                let fs_create_dir_fn = self
                    .module
                    .get_function("clorus_fs_create_dir")
                    .ok_or("fs/create-dir not declared. Did you forget (use rust.fs)?")?;

                let result = self
                    .builder
                    .build_call(fs_create_dir_fn, &[path_ptr.into()], "fs_create_dir_call")
                    .unwrap();

                let i32_result = result.try_as_basic_value().left().unwrap().into_int_value();
                let float_result = self
                    .builder
                    .build_unsigned_int_to_float(
                        i32_result,
                        self.context.f64_type(),
                        "i32_to_float",
                    )
                    .unwrap();

                Ok(self.box_number(float_result))
            }

            "fs/create-dir-all" => {
                if args.len() != 1 {
                    return Err("fs/create-dir-all requires 1 argument: path".to_string());
                }

                let path_ptr = self.compile_string_to_ptr(&args[0])?;

                let fs_create_dir_all_fn = self
                    .module
                    .get_function("clorus_fs_create_dir_all")
                    .ok_or("fs/create-dir-all not declared. Did you forget (use rust.fs)?")?;

                let result = self
                    .builder
                    .build_call(
                        fs_create_dir_all_fn,
                        &[path_ptr.into()],
                        "fs_create_dir_all_call",
                    )
                    .unwrap();

                let i32_result = result.try_as_basic_value().left().unwrap().into_int_value();
                let float_result = self
                    .builder
                    .build_unsigned_int_to_float(
                        i32_result,
                        self.context.f64_type(),
                        "i32_to_float",
                    )
                    .unwrap();

                Ok(self.box_number(float_result))
            }

            _ => Err(format!("Unknown fs function: {}", func)),
        }
    }

}
