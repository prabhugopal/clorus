use super::*;

impl<'ctx> CodeGen<'ctx> {
    /// Compile rust.egui-hello function calls
    pub(super) fn compile_egui_hello_call(
        &mut self,
        func: &str,
        args: &[Expr],
    ) -> Result<PointerValue<'ctx>, String> {
        match func {
            "egui-hello/show-gui" => {
                // show_gui takes 1 arg (message: String), returns f64
                if args.len() != 1 {
                    return Err("egui-hello/show-gui requires 1 argument: message".to_string());
                }

                let message_val = self.compile_expr(&args[0])?;
                let message_cstr = self.extract_cstring_from_value(message_val);

                let show_gui_fn = self.module.get_function("clorus_show_gui").ok_or_else(|| {
                    "clorus_show_gui not found - did you (use rust.egui-hello)?".to_string()
                })?;

                let result = self
                    .builder
                    .build_call(show_gui_fn, &[message_cstr.into()], "show_gui")
                    .unwrap();

                let f64_result = result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_float_value();
                Ok(self.box_number(f64_result))
            }

            "egui-hello/get-gui-version" => {
                // get_gui_version takes no args, returns String
                if !args.is_empty() {
                    return Err("egui-hello/get-gui-version takes no arguments".to_string());
                }

                let get_version_fn = self
                    .module
                    .get_function("clorus_get_gui_version")
                    .ok_or_else(|| {
                        "clorus_get_gui_version not found - did you (use rust.egui-hello)?"
                            .to_string()
                    })?;

                let result = self
                    .builder
                    .build_call(get_version_fn, &[], "get_gui_version")
                    .unwrap();

                let str_ptr = result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();
                Ok(self.box_string(str_ptr))
            }

            _ => Err(format!("Unknown rust.egui-hello function: {}", func)),
        }
    }

}
