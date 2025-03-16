use crate::ast::AstNode;
use crate::codegen::context::CodeGenContext;
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

                // Array element assignment: array[index] = value
                // Array element assignment: array[index] = value
                AstNode::ArraySubscript { array, index } => {
                    if let AstNode::Identifier(array_name) = &**array {
                        let addr_reg = calculate_array_element_address(array_name, index, context)?;
                        context.emit(&format!("    sw {}, 0({})", rhs_reg, addr_reg));
                        context.free_register(&addr_reg);
                        return Ok(rhs_reg);
                    }
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
            if let AstNode::Identifier(array_name) = &**array {
                let addr_reg = calculate_array_element_address(array_name, index, context)?;
                let result_reg = context.get_register();
                context.emit(&format!("    lw {}, 0({})", result_reg, addr_reg));
                context.free_register(&addr_reg);
                return Ok(result_reg);
            }
            Err(CompileError::CodegenError(format!("Unsupported expression type: {:?}", node)))
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
    let used_regs = context.get_used_registers();
    let needs_saving = !used_regs.is_empty();

    if needs_saving {
        // Allocate stack space for saving registers
        let stack_adjustment = used_regs.len() * 4; // 4 bytes per register
        context.emit(&format!("    addi sp, sp, -{}", stack_adjustment));

        // Save registers to stack
        for (i, reg) in used_regs.iter().enumerate() {
            let offset = i * 4;
            context.emit(&format!("    sw {}, {}(sp)", reg, offset));
        }
    }

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

    // Restore saved registers from stack
    if needs_saving {
        for (i, reg) in used_regs.iter().enumerate() {
            if reg != &result_reg {  // Don't restore if it's our result register
                let offset = i * 4;
                context.emit(&format!("    lw {}, {}(sp)", reg, offset));
            }
        }

        // Deallocate stack space
        let stack_adjustment = used_regs.len() * 4;
        context.emit(&format!("    addi sp, sp, {}", stack_adjustment));
    }

    Ok(result_reg)
}

fn calculate_array_element_address(
    array_name: &str,
    index: &AstNode,
    context: &mut CodeGenContext,
) -> Result<String, CompileError> {
    if let Some((base_offset, _)) = context.get_variable(array_name) {
        // Generate code for the index expression
        let index_reg = generate_expression(index, context)?;

        // Get a register for the address calculation
        let addr_reg = context.get_register();

        // Multiply index by 4 (size of int)
        context.emit(&format!("    slli {0}, {1}, 2", addr_reg, index_reg));

        // Add the base offset
        context.emit(&format!("    addi {0}, {0}, {1}", addr_reg, base_offset));

        // Add the frame pointer to get the final address
        context.emit(&format!("    add {0}, {0}, s0", addr_reg));

        // Free the index register
        context.free_register(&index_reg);

        Ok(addr_reg)
    } else {
        Err(CompileError::CodegenError(format!("Array '{}' not found", array_name)))
    }
}