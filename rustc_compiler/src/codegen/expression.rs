use crate::ast::AstNode;
use crate::codegen::context::CodeGenContext;
use crate::codegen::context::StorageLocation;
use crate::error::CompileError;
/// Generate code for an expression
pub fn generate_expression(node: &AstNode, context: &mut CodeGenContext) -> Result<String, CompileError> {
    match node {
        AstNode::Identifier(name) => {
            // Check if it's a variable reference
            if let Some(symbol) = context.lookup_symbol(name) {
                // Extract all information we need from the symbol before mutably borrowing context
                let is_float = symbol.type_info == "float" || symbol.type_info == "double";
                let location = symbol.location.clone(); // Clone to avoid borrow issues

                // Now we can get registers (which mutably borrow context)
                let result_reg = if is_float {
                    context.get_fp_register()
                } else {
                    context.get_register()
                };

                // Now use the extracted location information
                match location {
                    StorageLocation::Stack(offset) => {
                        // Load local variable from stack using the appropriate instruction
                        if is_float {
                            context.emit(&format!("    flw {}, {}(s0)", result_reg, offset));
                        } else {
                            context.emit(&format!("    lw {}, {}(s0)", result_reg, offset));
                        }
                    },
                    StorageLocation::Global(label) => {
                        // For global variables, we need a temporary integer register to hold the address
                        let addr_reg = context.get_register();

                        // Load global variable address
                        context.emit(&format!("    la {}, {}", addr_reg, label));

                        // Load the value using the appropriate instruction
                        if is_float {
                            context.emit(&format!("    flw {}, 0({})", result_reg, addr_reg));
                        } else {
                            context.emit(&format!("    lw {}, 0({})", result_reg, addr_reg));
                        }

                        // Free the temporary address register
                        context.free_register(&addr_reg);
                    },
                    StorageLocation::Register(reg) => {
                        // Variable is already in a register
                        if result_reg != reg {
                            if is_float {
                                context.emit(&format!("    fmv.s {}, {}", result_reg, reg));
                            } else {
                                context.emit(&format!("    mv {}, {}", result_reg, reg));
                            }
                        }
                    }
                }
                return Ok(result_reg);
            }

            // Otherwise it might be a function name or something else
            Err(CompileError::CodegenError(format!("Unknown identifier in expression: {}", name)))
        },
        AstNode::Assignment { lhs, rhs } => {
            // Generate RHS expression and get its type
            let rhs_reg = generate_expression(rhs, context)?;
            let rhs_type = get_expression_type(rhs, context)?;

            match &**lhs {
                AstNode::Identifier(name) => {
                    // Get LHS variable info (offset and type)
                    if let Some((offset, lhs_type)) = context.get_variable(name) {
                        match (lhs_type.as_str(), rhs_type.as_str()) {
                            ("int", "int") => {
                                // Integer to integer: use sw
                                context.emit(&format!("    sw {}, {}(s0)", rhs_reg, offset));
                                return Ok(rhs_reg)
                            },
                            ("float", "float") => {
                                // Float to float: ensure RHS is in an FP register, use fsw
                                if rhs_reg.starts_with('f') {
                                    context.emit(&format!("    fsw {}, {}(s0)", rhs_reg, offset));
                                    return Ok(rhs_reg)
                                } else {
                                    return Err(CompileError::CodegenError(
                                        "Expected floating-point register for float assignment".to_string()
                                    ));
                                };
                            },
                            ("float", "int") => {
                                // Int to float: convert RHS to float, then store with fsw
                                let fp_reg = context.get_fp_register();
                                context.emit(&format!("    fcvt.s.w {}, {}", fp_reg, rhs_reg));
                                context.emit(&format!("    fsw {}, {}(s0)", fp_reg, offset));
                                context.free_register(&rhs_reg);
                                context.free_fp_register(&fp_reg);
                                return Ok(fp_reg)
                            },
                            ("int", "float") => {
                                // Float to int: convert RHS to int, then store with sw
                                if rhs_reg.starts_with('f') {
                                    let int_reg = context.get_register();
                                    context.emit(&format!("    fcvt.w.s {}, {}", int_reg, rhs_reg));
                                    context.emit(&format!("    sw {}, {}(s0)", int_reg, offset));
                                    context.free_fp_register(&rhs_reg);
                                    context.free_register(&int_reg);
                                    return Ok(int_reg)
                                } else {
                                    return Err(CompileError::CodegenError(
                                        "Expected floating-point register for float expression".to_string()
                                    ));
                                };
                            },
                            _ => return Err(CompileError::CodegenError(
                                format!("Type mismatch in assignment: {} = {}", lhs_type, rhs_type)
                            )),
                        }
                    } else {
                        return Err(CompileError::CodegenError(format!("Variable '{}' not found", name)))
                    }
                },
                AstNode::ArraySubscript { .. } => {
                    let (base, indices) = collect_array_access(lhs)?;
                    let addr_reg = calculate_array_element_address(&base, &indices, context)?;
                    context.emit(&format!("    sw {}, 0({})", rhs_reg, addr_reg));
                    context.free_register(&addr_reg);
                    return Ok(rhs_reg);
                },
                _ => {},
            }
            // This error will be reached if none of the return statements above were executed
            Err(CompileError::CodegenError("Invalid assignment target".to_string()))
        },
        AstNode::IntegerLiteral(value) => generate_int_constant(*value, context),
        AstNode::BinaryOperation { op, left, right } => {
            // Check if both operands are floating-point
            let left_type = get_expression_type(left, context)?;
            let right_type = get_expression_type(right, context)?;

            if left_type == "float" || left_type == "double" ||
               right_type == "float" || right_type == "double" {
                generate_fp_binary_operation(op, left, right, context)
            } else {
                generate_binary_operation(op, left, right, context)
            }
        },
        AstNode::UnaryOperation { op, operand } => generate_unary_operation(op, operand, context),
        AstNode::IntConstant(value) => {
            let reg = context.get_register();
            context.emit(&format!("    li {}, {}", reg, value));
            Ok(reg)
        },
        AstNode::FunctionCall { function, args } => {
            // Generate code for function call
            generate_function_call(function, args, context)
        },
        // Array subscript: array[index]
        AstNode::ArraySubscript { array, index } => {
            let (base, indices) = collect_array_access(node)?;
            let addr_reg = calculate_array_element_address(&base, &indices, context)?;
            let result_reg = context.get_register();
            context.emit(&format!("    lw {}, 0({})", result_reg, addr_reg));
            context.free_register(&addr_reg);
            Ok(result_reg)
        },
        AstNode::FloatConstant(value) => {
            generate_float_constant(*value, context)
        },
        // Handle other expression types
        _ => Err(CompileError::CodegenError(format!("Unsupported expression type: {:?}", node))),
    }
}

/// Generate code for an integer constant
fn generate_int_constant(value: i32, context: &mut CodeGenContext) -> Result<String, CompileError> {
    let reg = context.get_register();
    context.emit(&format!("    li {}, {}", reg, value));
    Ok(reg)
}

/// Generate code for an identifier (variable reference)
fn generate_identifier(name: &str, context: &mut CodeGenContext) -> Result<String, CompileError> {
    // First, extract the information we need from the symbol
    let location = match context.lookup_symbol(name) {
        Some(symbol) => symbol.location.clone(),
        None => return Err(CompileError::CodegenError(format!("Identifier '{}' not found", name))),
    };

    let result_reg = context.get_register();

    // Use the extracted location information
    match location {
        crate::codegen::context::StorageLocation::Register(reg) => {
            context.emit(&format!("    mv {}, {}", result_reg, reg));
        },
        crate::codegen::context::StorageLocation::Stack(offset) => {
            context.emit(&format!("    lw {}, {}(s0)", result_reg, offset));
        },
        StorageLocation::Global(label) => {
            // For global variables, load address then load value
            context.emit(&format!("    la {}, {}", result_reg, label));
            context.emit(&format!("    lw {}, 0({})", result_reg, result_reg));
        },
    }

    Ok(result_reg)
}

/// Generate code for a binary operation
fn generate_binary_operation(
    op: &str,
    left: &AstNode,
    right: &AstNode,
    context: &mut CodeGenContext
) -> Result<String, CompileError> {
    // Special handling for logical operators with short-circuit evaluation
    if op == "&&" || op == "||" {
        return generate_logical_operation(op, left, right, context);
    }

    // Generate code for operands
    let left_reg = generate_expression(left, context)?;
    let right_reg = generate_expression(right, context)?;

    // Handle comparison operators
    let result_reg = match op {
        "==" => {
            context.emit(&format!("    xor {0}, {1}, {2}", left_reg, left_reg, right_reg));
            context.emit(&format!("    seqz {0}, {0}", left_reg));
            left_reg
        },
        "!=" => {
            context.emit(&format!("    xor {0}, {1}, {2}", left_reg, left_reg, right_reg));
            context.emit(&format!("    snez {0}, {0}", left_reg));
            left_reg
        },
        "<" => {
            context.emit(&format!("    slt {0}, {1}, {2}", left_reg, left_reg, right_reg));
            left_reg
        },
        ">" => {
            context.emit(&format!("    slt {0}, {1}, {2}", left_reg, right_reg, left_reg));
            left_reg
        },
        "<=" => {
            context.emit(&format!("    slt {0}, {1}, {2}", left_reg, right_reg, left_reg));
            context.emit(&format!("    xori {0}, {0}, 1", left_reg));
            left_reg
        },
        ">=" => {
            context.emit(&format!("    slt {0}, {1}, {2}", left_reg, left_reg, right_reg));
            context.emit(&format!("    xori {0}, {0}, 1", left_reg));
            left_reg
        },
        // Arithmetic operators
        "+" => {
            context.emit(&format!("    add {0}, {1}, {2}", left_reg, left_reg, right_reg));
            left_reg
        },
        "-" => {
            context.emit(&format!("    sub {0}, {1}, {2}", left_reg, left_reg, right_reg));
            left_reg
        },
        "*" => {
            context.emit(&format!("    mul {0}, {1}, {2}", left_reg, left_reg, right_reg));
            left_reg
        },
        "/" => {
            context.emit(&format!("    div {0}, {1}, {2}", left_reg, left_reg, right_reg));
            left_reg
        },
        "%" => {
            context.emit(&format!("    rem {0}, {1}, {2}", left_reg, left_reg, right_reg));
            left_reg
        },
        // Bitwise operators
        "&" => {
            context.emit(&format!("    and {0}, {1}, {2}", left_reg, left_reg, right_reg));
            left_reg
        },
        "|" => {
            context.emit(&format!("    or {0}, {1}, {2}", left_reg, left_reg, right_reg));
            left_reg
        },
        "^" => {
            context.emit(&format!("    xor {0}, {1}, {2}", left_reg, left_reg, right_reg));
            left_reg
        },
        "<<" => {
            context.emit(&format!("    sll {0}, {1}, {2}", left_reg, left_reg, right_reg));
            left_reg
        },
        ">>" => {
            context.emit(&format!("    sra {0}, {1}, {2}", left_reg, left_reg, right_reg));
            left_reg
        },
        _ => return Err(CompileError::CodegenError(format!("Invalid binary operation: {}", op))),
    };

    // Free the right register since the result is in the left register
    context.free_register(&right_reg);

    Ok(result_reg)
}

/// Generate code for unary operations
fn generate_unary_operation(
    op: &str,
    operand: &AstNode,
    context: &mut CodeGenContext
) -> Result<String, CompileError> {
    match op {
        // Unary plus: doesn't change the value
        "+" => {
            // Just evaluate the operand
            generate_expression(operand, context)
        },

        // Unary minus: negate the value
        "-" => {
            let operand_reg = generate_expression(operand, context)?;
            context.emit(&format!("    neg {0}, {0}", operand_reg));
            Ok(operand_reg)
        },

        // Bitwise NOT
        "~" => {
            let operand_reg = generate_expression(operand, context)?;
            context.emit(&format!("    not {0}, {0}", operand_reg));
            Ok(operand_reg)
        },

        // Logical NOT
        "!" => {
            let operand_reg = generate_expression(operand, context)?;
            // Set to 1 if operand is 0, otherwise set to 0
            context.emit(&format!("    seqz {0}, {0}", operand_reg));
            Ok(operand_reg)
        },

        // Pre-increment: increment operand, then return new value
        "++" => {
            if let AstNode::Identifier(var_name) = operand {
                if let Some((offset, _)) = context.get_variable(var_name) {
                    let result_reg = context.get_register();

                    // Load current value
                    context.emit(&format!("    lw {}, {}(s0)", result_reg, offset));

                    // Increment
                    context.emit(&format!("    addi {}, {}, 1", result_reg, result_reg));

                    // Store back
                    context.emit(&format!("    sw {}, {}(s0)", result_reg, offset));

                    // Result is the new value (already in result_reg)
                    return Ok(result_reg);
                }
            }
            Err(CompileError::CodegenError("Invalid operand for ++ operation".to_string()))
        },

        // Pre-decrement: decrement operand, then return new value
        "--" => {
            if let AstNode::Identifier(var_name) = operand {
                if let Some((offset, _)) = context.get_variable(var_name) {
                    let result_reg = context.get_register();

                    // Load current value
                    context.emit(&format!("    lw {}, {}(s0)", result_reg, offset));

                    // Decrement
                    context.emit(&format!("    addi {}, {}, -1", result_reg, result_reg));

                    // Store back
                    context.emit(&format!("    sw {}, {}(s0)", result_reg, offset));

                    // Result is the new value (already in result_reg)
                    return Ok(result_reg);
                }
            }
            Err(CompileError::CodegenError("Invalid operand for -- operation".to_string()))
        },

        // Post-increment: return original value, then increment
        "post++" => {
            if let AstNode::Identifier(var_name) = operand {
                if let Some((offset, _)) = context.get_variable(var_name) {
                    let result_reg = context.get_register();
                    let temp_reg = context.get_register();

                    // Load current value
                    context.emit(&format!("    lw {}, {}(s0)", result_reg, offset));

                    // Copy to temp register
                    context.emit(&format!("    mv {}, {}", temp_reg, result_reg));

                    // Increment temp
                    context.emit(&format!("    addi {}, {}, 1", temp_reg, temp_reg));

                    // Store back the incremented value
                    context.emit(&format!("    sw {}, {}(s0)", temp_reg, offset));

                    // Free temp register
                    context.free_register(&temp_reg);

                    // Result is the original value (in result_reg)
                    return Ok(result_reg);
                }
            }
            Err(CompileError::CodegenError("Invalid operand for post++ operation".to_string()))
        },

        // Post-decrement: return original value, then decrement
        "post--" => {
            if let AstNode::Identifier(var_name) = operand {
                if let Some((offset, _)) = context.get_variable(var_name) {
                    let result_reg = context.get_register();
                    let temp_reg = context.get_register();

                    // Load current value
                    context.emit(&format!("    lw {}, {}(s0)", result_reg, offset));

                    // Copy to temp register
                    context.emit(&format!("    mv {}, {}", temp_reg, result_reg));

                    // Decrement temp
                    context.emit(&format!("    addi {}, {}, -1", temp_reg, temp_reg));

                    // Store back the decremented value
                    context.emit(&format!("    sw {}, {}(s0)", temp_reg, offset));

                    // Free temp register
                    context.free_register(&temp_reg);

                    // Result is the original value (in result_reg)
                    return Ok(result_reg);
                }
            }
            Err(CompileError::CodegenError("Invalid operand for post-- operation".to_string()))
        },

        // Pointer dereference
        "*" => {
            let addr_reg = generate_expression(operand, context)?;
            let result_reg = context.get_register();

            // Load from address in addr_reg
            context.emit(&format!("    lw {}, 0({})", result_reg, addr_reg));

            // Free the address register if different from result
            if addr_reg != result_reg {
                context.free_register(&addr_reg);
            }

            Ok(result_reg)
        },

        // Address-of operator
        "&" => {
            if let AstNode::Identifier(var_name) = operand {
                if let Some((offset, _)) = context.get_variable(var_name) {
                    let result_reg = context.get_register();

                    // Calculate address: frame pointer + offset
                    context.emit(&format!("    addi {}, s0, {}", result_reg, offset));

                    return Ok(result_reg);
                }
            }
            Err(CompileError::CodegenError("Invalid operand for & operation".to_string()))
        },

        _ => Err(CompileError::CodegenError(format!("Unsupported unary operation: {}", op))),
    }
}

/// Generate code for logical operators with short-circuit evaluation
fn generate_logical_operation(
    op: &str,
    left: &AstNode,
    right: &AstNode,
    context: &mut CodeGenContext
) -> Result<String, CompileError> {
    // Generate code for left operand
    let result_reg = generate_expression(left, context)?;

    // Generate unique labels for short-circuit evaluation
    let end_label = context.generate_label("logical_end");
    let short_circuit_label = context.generate_label("short_circuit");

    // Normalize left operand to 0 or 1
    context.emit(&format!("    snez {0}, {0}", result_reg));

    if op == "&&" {
        // For AND: if left is 0, short-circuit to end (result is already 0)
        context.emit(&format!("    beqz {}, {}", result_reg, short_circuit_label));

        // Left is 1, evaluate right operand
        let right_reg = generate_expression(right, context)?;

        // Normalize right operand to 0 or 1
        context.emit(&format!("    snez {0}, {0}", right_reg));

        // Move right result to result register
        if result_reg != right_reg {
            context.emit(&format!("    mv {}, {}", result_reg, right_reg));
            context.free_register(&right_reg);
        }

        context.emit(&format!("    j {}", end_label));
        context.emit(&format!("{}:", short_circuit_label));
        // For short-circuit, result is already 0 in result_reg

    } else if op == "||" {
        // For OR: if left is 1, short-circuit to end (result is already 1)
        context.emit(&format!("    bnez {}, {}", result_reg, short_circuit_label));

        // Left is 0, evaluate right operand
        let right_reg = generate_expression(right, context)?;

        // Normalize right operand to 0 or 1
        context.emit(&format!("    snez {0}, {0}", right_reg));

        // Move right result to result register
        if result_reg != right_reg {
            context.emit(&format!("    mv {}, {}", result_reg, right_reg));
            context.free_register(&right_reg);
        }

        context.emit(&format!("    j {}", end_label));
        context.emit(&format!("{}:", short_circuit_label));
        // For short-circuit, result is already 1 in result_reg

    } else {
        return Err(CompileError::CodegenError(format!("Invalid logical operation: {}", op)));
    }

    context.emit(&format!("{}:", end_label));

    Ok(result_reg)
}

// For assignment operations
fn generate_assignment(left: &AstNode, right: &AstNode, context: &mut CodeGenContext) -> Result<String, CompileError> {
    let right_reg = generate_expression(right, context)?;

    if let AstNode::Identifier(var_name) = left {
        if let Some((offset, _)) = context.get_variable(var_name) {
            // Store in memory
            context.emit(&format!("    sw {}, {}(s0)", right_reg, offset));
            return Ok(right_reg);
        }
    }

    // Handle lvalue
    let left_reg = generate_expression(left, context)?;

    // Store right value to address in left_reg
    context.emit(&format!("    sw {}, 0({})", right_reg, left_reg));

    // Free the left register and return the right one
    context.free_register(&left_reg);
    Ok(right_reg)
}

fn generate_function_call(
    function: &AstNode,
    args: &[Box<AstNode>],
    context: &mut CodeGenContext
) -> Result<String, CompileError> {
    // Get function name
    let func_name = match function {
        AstNode::Identifier(name) => name,
        _ => return Err(CompileError::CodegenError("Expected function name".to_string())),
    };

    // Save all used registers to stack before the call
    let int_registers = context.save_temp_registers();
    let fp_registers = context.save_fp_registers();

    // Process arguments
    for (i, arg) in args.iter().enumerate().take(8) {
        let arg_type = get_expression_type(arg, context)?;
        let is_float = arg_type == "float" || arg_type == "double";

        if is_float {
            // Floating-point arguments go in fa0-fa7
            if let AstNode::FloatConstant(value) = &**arg {
                // For float constants, we need to load from memory
                let const_label = context.generate_label("float_const");
                context.emit_data(&format!("{}:", const_label));
                context.emit_data(&format!("    .word 0x{:08x}  # float {}",
                                          f32::to_bits(*value as f32), value));

                let temp_reg = context.get_register();
                context.emit(&format!("    la {}, {}", temp_reg, const_label));
                context.emit(&format!("    flw fa{}, 0({})", i, temp_reg));
                context.free_register(&temp_reg);
            } else {
                // For complex expressions, evaluate and move to argument register
                let arg_reg = generate_expression(arg, context)?;

                if arg_reg.starts_with('f') {
                    // Already a floating-point register
                    if arg_reg != format!("fa{}", i) {
                        context.emit(&format!("    fmv.s fa{}, {}", i, arg_reg));
                        context.free_fp_register(&arg_reg);
                    }
                } else {
                    // Integer register, need to convert
                    context.emit(&format!("    fcvt.s.w fa{}, {}", i, arg_reg));
                    context.free_register(&arg_reg);
                }
            }
        } else {
            // Regular integer arguments go in a0-a7
            if let AstNode::IntConstant(value) = &**arg {
                // Load immediate directly into argument register
                context.emit(&format!("    li a{}, {}", i, value));
            } else {
                // For complex expressions, evaluate and move to argument register
                let arg_reg = generate_expression(arg, context)?;

                if arg_reg.starts_with('f') {
                    // Floating-point register, need to convert to integer
                    context.emit(&format!("    fcvt.w.s a{}, {}", i, arg_reg));
                    context.free_fp_register(&arg_reg);
                } else if arg_reg != format!("a{}", i) {
                    context.emit(&format!("    mv a{}, {}", i, arg_reg));
                    context.free_register(&arg_reg);
                }
            }
        }
    }

    // Call the function
    context.emit(&format!("    call {}", func_name));

    // Look up the function's return type
    let return_type = context.get_function_return_type(func_name)
        .unwrap_or_else(|| {
            // If we can't find the function, default to "int" and emit a warning comment
            context.emit(&format!("    # Warning: Unknown return type for function {}, assuming int", func_name));
            "int".to_string()
        });

    let result_reg = if return_type == "float" || return_type == "double" {
        let fp_reg = context.get_fp_register();

        // Move return value (in fa0) to our result register if needed
        if fp_reg != "fa0" {
            context.emit(&format!("    fmv.s {}, fa0", fp_reg));
        }
        fp_reg
    } else {
        let reg = context.get_register();

        // Move return value (in a0) to our result register if needed
        if reg != "a0" {
            context.emit(&format!("    mv {}, a0", reg));
        }
        reg
    };

    // Restore registers
    context.restore_fp_registers(fp_registers);
    context.restore_temp_registers(int_registers);

    Ok(result_reg)
}

fn calculate_array_element_address(
    array_name: &str,
    indices: &[AstNode],
    context: &mut CodeGenContext,
) -> Result<String, CompileError> {
    // data from the symbol (immutable borrow)
    let (base_offset, dimensions, is_global, global_label) = {
        if let Some(symbol) = context.lookup_symbol(array_name) {
            match &symbol.location {
                StorageLocation::Stack(offset) => {
                    // Clone the dimensions
                    (*offset, symbol.dimensions.clone(), false, String::new())
                },
                StorageLocation::Global(label) => {
                    (0, symbol.dimensions.clone(), true, label.clone())
                },
                _ => {
                    return Err(CompileError::CodegenError("Array has invalid storage type".to_string()));
                }
            }
        } else {
            return Err(CompileError::CodegenError(format!("Array '{}' not found", array_name)));
        }
    };

    // Check if dimensions match
    if indices.len() != dimensions.len() {
        return Err(CompileError::CodegenError(format!(
            "Number of indices ({}) does not match array dimensions ({})",
            indices.len(),
            dimensions.len()
        )));
    }

    // Generate code for each index expression
    let mut index_regs = Vec::new();
    for index in indices {
        let reg = generate_expression(index, context)?;
        index_regs.push(reg);
    }

    // Compute linear index for row-major order
    let mut offset_reg = context.get_register();
    context.emit(&format!("    mv {}, {}", offset_reg, index_regs[0])); // Start with i1
    let mut tmp_reg = context.get_register();
    for m in 1..dimensions.len() {
        let dim = dimensions[m];
        context.emit(&format!("    li {}, {}", tmp_reg, dim)); // Load dimension into temp reg
        context.emit(&format!("    mul {}, {}, {}", offset_reg, offset_reg, tmp_reg)); // offset *= dm
        context.emit(&format!("    add {}, {}, {}", offset_reg, offset_reg, index_regs[m])); // offset += im
    }
    context.free_register(&tmp_reg);
    // Compute final address: s0 + base_offset + linear_index * element_size
    let addr_reg = context.get_register();
    context.emit(&format!("    slli {0}, {1}, 2", addr_reg, offset_reg)); // *4 for int size

    if is_global {
        // For global arrays, load the base address then add the offset
        let temp_reg = context.get_register();
        context.emit(&format!("    la {}, {}", temp_reg, global_label));
        context.emit(&format!("    add {}, {}, {}", addr_reg, addr_reg, temp_reg));
        context.free_register(&temp_reg);
    } else {
        // For local arrays, add the base offset and frame pointer
        if base_offset >= -2048 && base_offset <= 2047 {
            context.emit(&format!("    addi {0}, {0}, {1}", addr_reg, base_offset));
        } else {
            let temp_reg = context.get_register();
            context.emit(&format!("    li {}, {}", temp_reg, base_offset));
            context.emit(&format!("    add {0}, {0}, {1}", addr_reg, temp_reg));
            context.free_register(&temp_reg);
        }
        context.emit(&format!("    add {0}, {0}, s0", addr_reg));
    }

    // Free temporary registers
    for reg in index_regs {
        context.free_register(&reg);
    }
    context.free_register(&offset_reg);

    Ok(addr_reg)
}

fn collect_array_access(node: &AstNode) -> Result<(String, Vec<AstNode>), CompileError> {
    let mut indices = Vec::new();
    let mut current = node;
    loop {
        match current {
            AstNode::ArraySubscript { array, index } => {
                indices.push((**index).clone());
                current = array;
            },
            AstNode::Identifier(name) => {
                indices.reverse(); // Correct order: [i, j] for x[i][j]
                return Ok((name.clone(), indices));
            },
            _ => return Err(CompileError::CodegenError("Invalid array access".to_string())),
        }
    }
}

// Add new function for generating float constant code
fn generate_float_constant(value: f64, context: &mut CodeGenContext) -> Result<String, CompileError> {
    let reg = context.get_fp_register();

    // For floating-point constants, we need to load from a data section label
    let label = context.generate_label("float_const");
    context.emit_data(&format!("{}:", label));
    context.emit_data(&format!("    .word 0x{:08x}  # float {}", f32::to_bits(value as f32), value));

    // Load the float into a register
    let temp_reg = context.get_register();
    context.emit(&format!("    la {}, {}", temp_reg, label));
    context.emit(&format!("    flw {}, 0({})", reg, temp_reg));
    context.free_register(&temp_reg);

    Ok(reg)
}

// Add a new function for floating-point binary operations
fn generate_fp_binary_operation(
    op: &str,
    left: &AstNode,
    right: &AstNode,
    context: &mut CodeGenContext
) -> Result<String, CompileError> {
    // Generate code for operands
    let left_reg = generate_expression(left, context)?;
    let right_reg = generate_expression(right, context)?;

    // Determine if we need to convert integer operands to floating-point
    let left_type = get_expression_type(left, context)?;
    let right_type = get_expression_type(right, context)?;

    let left_fp_reg = if !left_reg.starts_with('f') && (left_type == "float" || left_type == "double") {
        // Convert integer to float if needed
        let fp_reg = context.get_fp_register();
        context.emit(&format!("    fcvt.s.w {}, {}", fp_reg, left_reg));
        context.free_register(&left_reg);
        fp_reg
    } else {
        left_reg
    };

    let right_fp_reg = if !right_reg.starts_with('f') && (right_type == "float" || right_type == "double") {
        // Convert integer to float if needed
        let fp_reg = context.get_fp_register();
        context.emit(&format!("    fcvt.s.w {}, {}", fp_reg, right_reg));
        context.free_register(&right_reg);
        fp_reg
    } else {
        right_reg
    };

    // Handle floating-point operations
    let result_reg = match op {
        "+" => {
            context.emit(&format!("    fadd.s {}, {}, {}", left_fp_reg, left_fp_reg, right_fp_reg));
            left_fp_reg
        },
        "-" => {
            context.emit(&format!("    fsub.s {}, {}, {}", left_fp_reg, left_fp_reg, right_fp_reg));
            left_fp_reg
        },
        "*" => {
            context.emit(&format!("    fmul.s {}, {}, {}", left_fp_reg, left_fp_reg, right_fp_reg));
            left_fp_reg
        },
        "/" => {
            context.emit(&format!("    fdiv.s {}, {}, {}", left_fp_reg, left_fp_reg, right_fp_reg));
            left_fp_reg
        },
        // For comparisons, we need to use floating-point comparison instructions
        "==" => {
            let int_reg = context.get_register();
            context.emit(&format!("    feq.s {}, {}, {}", int_reg, left_fp_reg, right_fp_reg));
            context.free_fp_register(&left_fp_reg);
            context.free_fp_register(&right_fp_reg);
            int_reg
        },
        "!=" => {
            let int_reg = context.get_register();
            context.emit(&format!("    feq.s {}, {}, {}", int_reg, left_fp_reg, right_fp_reg));
            context.emit(&format!("    xori {}, {}, 1", int_reg, int_reg));
            context.free_fp_register(&left_fp_reg);
            context.free_fp_register(&right_fp_reg);
            int_reg
        },
        "<" => {
            let int_reg = context.get_register();
            context.emit(&format!("    flt.s {}, {}, {}", int_reg, left_fp_reg, right_fp_reg));
            context.free_fp_register(&left_fp_reg);
            context.free_fp_register(&right_fp_reg);
            int_reg
        },
        ">" => {
            let int_reg = context.get_register();
            context.emit(&format!("    flt.s {}, {}, {}", int_reg, right_fp_reg, left_fp_reg));
            context.free_fp_register(&left_fp_reg);
            context.free_fp_register(&right_fp_reg);
            int_reg
        },
        "<=" => {
            let int_reg = context.get_register();
            context.emit(&format!("    fle.s {}, {}, {}", int_reg, left_fp_reg, right_fp_reg));
            context.free_fp_register(&left_fp_reg);
            context.free_fp_register(&right_fp_reg);
            int_reg
        },
        ">=" => {
            let int_reg = context.get_register();
            context.emit(&format!("    fle.s {}, {}, {}", int_reg, right_fp_reg, left_fp_reg));
            context.free_fp_register(&left_fp_reg);
            context.free_fp_register(&right_fp_reg);
            int_reg
        },
        _ => return Err(CompileError::CodegenError(format!("Invalid floating-point operation: {}", op))),
    };

    // Free the right register if it's not the same as the result
    if right_fp_reg != result_reg {
        if right_fp_reg.starts_with('f') {
            context.free_fp_register(&right_fp_reg);
        } else {
            context.free_register(&right_fp_reg);
        }
    }

    Ok(result_reg)
}

// Helper function to determine expression type
pub fn get_expression_type(node: &AstNode, context: &mut CodeGenContext) -> Result<String, CompileError> {
    match node {
        AstNode::IntConstant(_) => Ok("int".to_string()),
        AstNode::FloatConstant(_) => Ok("float".to_string()),
        AstNode::Identifier(name) => {
            if let Some(symbol) = context.lookup_symbol(name) {
                Ok(symbol.type_info.clone())
            } else {
                // Could be a function name without a call
                if let Some(return_type) = context.get_function_return_type(name) {
                    Ok(return_type)
                } else {
                    Err(CompileError::CodegenError(format!("Unknown identifier: {}", name)))
                }
            }
        },
        AstNode::FunctionCall { function, .. } => {
            if let AstNode::Identifier(func_name) = &**function {
                if let Some(return_type) = context.get_function_return_type(func_name) {
                    Ok(return_type)
                } else {
                    // Default to int if we don't know the return type
                    // Alternatively, you could return an error here
                    Ok("int".to_string())
                }
            } else {
                // Handle function pointers or complex expressions later
                Ok("int".to_string())
            }
        },
        // For binary operations, determine return type based on operands
        AstNode::BinaryOperation { left, right, .. } => {
            let left_type = get_expression_type(left, context)?;
            let right_type = get_expression_type(right, context)?;

            // Simple type promotion: float/double wins
            if left_type == "double" || right_type == "double" {
                Ok("double".to_string())
            } else if left_type == "float" || right_type == "float" {
                Ok("float".to_string())
            } else {
                Ok("int".to_string())
            }
        },
        // Handle other expression types
        _ => Ok("int".to_string()), // Default to int for unknown expressions
    }
}