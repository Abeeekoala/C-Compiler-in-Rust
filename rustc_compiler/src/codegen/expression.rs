use crate::ast::AstNode;
use crate::codegen::context::CodeGenContext;

/// Generate code for an expression
pub fn generate_expression(node: &AstNode, context: &mut CodeGenContext) -> Option<String> {
    match node {
        AstNode::IntConstant(value) => generate_int_constant(*value, context),
        AstNode::Identifier(name) => generate_identifier(name, context),
        AstNode::BinaryOperation { op, left, right } =>
            generate_binary_operation(op, left, right, context),
        // AstNode::UnaryOperation { op, operand } =>
        //     generate_unary_operation(op, operand, context),
        // AstNode::FunctionCall { function, args } =>
        //     generate_function_call(function, args, context),
        // Add other expression types as needed
        _ => None,
    }
}

/// Generate code for an integer constant
fn generate_int_constant(value: i32, context: &mut CodeGenContext) -> Option<String> {
    let reg = context.allocate_register()?;
    context.emit(&format!("    li {}, {}", reg, value));
    Some(reg)
}

/// Generate code for an identifier (variable reference)
fn generate_identifier(name: &str, context: &mut CodeGenContext) -> Option<String> {
    // First, extract the information we need from the symbol
    let location = match context.lookup_symbol(name) {
        Some(symbol) => symbol.location.clone(),
        None => return None,
    };

    // Now we can allocate a register (mutable borrow)
    let result_reg = context.allocate_register()?;

    // Use the extracted location information
    match location {
        crate::codegen::context::StorageLocation::Register(reg) => {
            context.emit(&format!("    mv {}, {}", result_reg, reg));
        },
        crate::codegen::context::StorageLocation::Stack(offset) => {
            context.emit(&format!("    lw {}, {}(s0)", result_reg, offset));
        },
    }

    Some(result_reg)
}

/// Generate code for a binary operation
fn generate_binary_operation(
    op: &str,
    left: &AstNode,
    right: &AstNode,
    context: &mut CodeGenContext
) -> Option<String> {
    // Generate code for left and right operands
    let left_reg = generate_expression(left, context)?;
    let right_reg = generate_expression(right, context)?;

    // Perform the operation
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
        "==" => {
            context.emit(&format!("    xor {0}, {0}, {1}", left_reg, right_reg));
            context.emit(&format!("    seqz {0}, {0}", left_reg));
            context.free_register(&right_reg);
            return Some(left_reg);
        },
        "!=" => {
            context.emit(&format!("    xor {0}, {0}, {1}", left_reg, right_reg));
            context.emit(&format!("    snez {0}, {0}", left_reg));
            context.free_register(&right_reg);
            return Some(left_reg);
        },
        "<" => {
            context.emit(&format!("    slt {0}, {0}, {1}", left_reg, right_reg));
            context.free_register(&right_reg);
            return Some(left_reg);
        },
        ">" => {
            context.emit(&format!("    slt {0}, {1}, {0}", left_reg, right_reg));
            context.free_register(&right_reg);
            return Some(left_reg);
        },
        "<=" => {
            context.emit(&format!("    slt {0}, {1}, {0}", left_reg, right_reg));
            context.emit(&format!("    xori {0}, {0}, 1", left_reg));
            context.free_register(&right_reg);
            return Some(left_reg);
        },
        ">=" => {
            context.emit(&format!("    slt {0}, {0}, {1}", left_reg, right_reg));
            context.emit(&format!("    xori {0}, {0}, 1", left_reg));
            context.free_register(&right_reg);
            return Some(left_reg);
        },
        "=" => {
            // Assignment
            // For now, just handle simple variable assignment
            if let AstNode::Identifier(name) = &*left {
                // Extract symbol information first
                let location_opt = context.lookup_symbol(name).map(|sym| sym.location.clone());

                if let Some(location) = location_opt {
                    match location {
                        crate::codegen::context::StorageLocation::Register(reg) => {
                            context.emit(&format!("    mv {}, {}", reg, right_reg));
                        },
                        crate::codegen::context::StorageLocation::Stack(offset) => {
                            context.emit(&format!("    sw {}, {}(s0)", right_reg, offset));
                        },
                    }
                }
            }
            context.free_register(&left_reg);
            return Some(right_reg);
        },
        _ => return None,
    };

    context.emit(&format!("    {} {}, {}, {}", instruction, left_reg, left_reg, right_reg));
    context.free_register(&right_reg);
    Some(left_reg)
}

// Add implementations for unary operations, function calls, etc.
