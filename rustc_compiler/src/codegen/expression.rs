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
            if let AstNode::Identifier(name) = &**lhs {
                if let Some((offset, _)) = context.get_variable(name) {
                    context.emit(&format!("sw {}, {}(s0)", rhs_reg, offset));
                    return Ok(rhs_reg);
                }
            }
            Err(CompileError::CodegenError("Invalid assignment".to_string()))
        }
        AstNode::IntegerLiteral(value) => generate_int_constant(*value, context),
        AstNode::BinaryOperation { op, left, right } => generate_binary_operation(op, left, right, context),
        AstNode::Assignment { lhs, rhs } => generate_assignment(lhs, rhs, context),
        AstNode::IntConstant(value) => {
            let reg = context.get_register();
            context.emit(&format!("    li {}, {}", reg, value));
            Ok(reg)
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

// For the relational operators (<, >, <=, >=, ==, !=)
fn generate_relational_op(left: &AstNode, right: &AstNode, op: &str, context: &mut CodeGenContext) -> Result<String, CompileError> {
    let left_reg = generate_expression(left, context)?;
    let right_reg = generate_expression(right, context)?;

    // Now left_reg and right_reg are String, not Option<String>
    match op {
        "<" => {
            context.emit(&format!("    slt {0}, {1}, {2}", left_reg, left_reg, right_reg));
            context.free_register(&right_reg);
            Ok(left_reg)
        },
        ">" => {
            context.emit(&format!("    slt {0}, {1}, {2}", left_reg, right_reg, left_reg));
            context.free_register(&right_reg);
            Ok(left_reg)
        },
        "<=" => {
            context.emit(&format!("    slt {0}, {1}, {2}", left_reg, right_reg, left_reg));
            context.emit(&format!("    xori {0}, {0}, 1", left_reg));
            context.free_register(&right_reg);
            Ok(left_reg)
        },
        ">=" => {
            context.emit(&format!("    slt {0}, {1}, {2}", left_reg, left_reg, right_reg));
            context.emit(&format!("    xori {0}, {0}, 1", left_reg));
            context.free_register(&right_reg);
            Ok(left_reg)
        },
        "==" => {
            context.emit(&format!("    xor {0}, {1}, {2}", left_reg, left_reg, right_reg));
            context.emit(&format!("    seqz {0}, {0}", left_reg));
            context.free_register(&right_reg);
            Ok(left_reg)
        },
        "!=" => {
            context.emit(&format!("    xor {0}, {1}, {2}", left_reg, left_reg, right_reg));
            context.emit(&format!("    snez {0}, {0}", left_reg));
            context.free_register(&right_reg);
            Ok(left_reg)
        },
        // Arithmetic and bitwise operators
        "+" | "-" | "*" | "/" | "%" | "&" | "|" | "^" | "<<" | ">>" => {
            let instruction = match op {
                "+" => "add",
                "-" => "sub",
                "*" => "mul",
                "/" => "div",
                "%" => "rem",
                "&" => "and",
                "|" => "or",
                "^" => "xor",
                "<<" => "sll",
                ">>" => "sra",  // Arithmetic shift
                _ => unreachable!(),
            };

            context.emit(&format!("    {} {}, {}, {}", instruction, left_reg, left_reg, right_reg));
            context.free_register(&right_reg);
            Ok(left_reg)
        },
        _ => Err(CompileError::CodegenError(format!("Invalid binary operation: {}", op))),
    }
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

// Add implementations for unary operations, function calls, etc.
