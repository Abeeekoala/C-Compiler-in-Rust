use crate::ast::AstNode;
use crate::codegen::context::CodeGenContext;
use crate::codegen::context::StorageLocation;
use crate::error::CompileError;
/// Generate code for an expression
pub fn generate_expression(node: &AstNode, context: &mut CodeGenContext) -> Result<String, CompileError> {
    match node {
        AstNode::Identifier(name) => {
            // Get a register to store the result
            let result_reg = context.get_register();

            // Check if it's a variable reference
            if let Some((offset, _)) = context.get_variable(name) {
                // Load from stack
                context.emit(&format!("    lw {}, {}(s0)", result_reg, offset));
                return Ok(result_reg);
            }

            // Otherwise it might be a function name or something else
            Err(CompileError::CodegenError(format!("Unknown identifier: {}", name)))
        },
        AstNode::Assignment { lhs, rhs } => {
            let rhs_reg = generate_expression(rhs, context)?;

            match &**lhs {
                // Regular variable assignment
                AstNode::Identifier(name) => {
                    if let Some((offset, _)) = context.get_variable(name) {
                        context.emit(&format!("    sw {}, {}(s0)", rhs_reg, offset));
                        return Ok(rhs_reg);
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
        AstNode::BinaryOperation { op, left, right } => generate_binary_operation(op, left, right, context),
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
    let registers = context.save_temp_registers();

    // Process arguments
    for (i, arg) in args.iter().enumerate().take(8) {
        // Check if this is a simple constant that can be loaded directly
        if let AstNode::IntConstant(value) = &**arg {
            // Load immediate directly into argument register
            context.emit(&format!("    li a{}, {}", i, value));
        } else {
            // For complex expressions, evaluate and move to argument register
            let arg_reg = generate_expression(arg, context)?;
            if arg_reg != format!("a{}", i) {
                context.emit(&format!("    mv a{}, {}", i, arg_reg));
                context.free_register(&arg_reg);
            }
        }
    }

    // Call the function
    context.emit(&format!("    call {}", func_name));

    // Get a register for the result
    let result_reg = context.get_register();

    // Move return value (in a0) to our result register if needed
    if result_reg != "a0" {
        context.emit(&format!("    mv {}, a0", result_reg));
    }

    context.restore_temp_registers(registers);

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