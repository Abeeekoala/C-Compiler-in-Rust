use crate::ast::AstNode;
use crate::codegen::context::CodeGenContext;
use crate::codegen::context::StorageLocation;
use crate::error::CompileError;
/// Generate code for an expression
pub fn generate_expression(node: &AstNode, context: &mut CodeGenContext) -> Result<String, CompileError> {
    match node {
        AstNode::Identifier(name) => {
            if let Some(symbol) = context.lookup_symbol(name) {
                let type_info = symbol.type_info.clone();
                let location = symbol.location.clone();
                let is_float_or_double = type_info == "float" || type_info == "double";

                let result_reg = if is_float_or_double {
                    context.get_fp_register()
                } else {
                    context.get_register()
                };

                match location {
                    StorageLocation::Stack(offset) => {
                        if type_info == "double" {
                            context.emit(&format!("    fld {}, {}(s0)", result_reg, offset));
                        } else if type_info == "float" {
                            context.emit(&format!("    flw {}, {}(s0)", result_reg, offset));
                        } else {
                            context.emit(&format!("    lw {}, {}(s0)", result_reg, offset));
                        }
                    },
                    StorageLocation::Global(label) => {
                        let addr_reg = context.get_register();
                        context.emit(&format!("    la {}, {}", addr_reg, label));

                        if type_info == "double" {
                            context.emit(&format!("    fld {}, 0({})", result_reg, addr_reg));
                        }
                        else if type_info == "float" {
                            context.emit(&format!("    flw {}, 0({})", result_reg, addr_reg));
                        } else {
                            context.emit(&format!("    lw {}, 0({})", result_reg, addr_reg));
                        }

                        context.free_register(&addr_reg);
                    },
                    StorageLocation::Register(reg) => {
                        if result_reg != reg {
                            if type_info == "double" {
                                context.emit(&format!("    fmv.d {}, {}", result_reg, reg));
                            } else if type_info == "float" {
                                context.emit(&format!("    fmv.s {}, {}", result_reg, reg));
                            } else {
                                context.emit(&format!("    mv {}, {}", result_reg, reg));
                            }
                        }
                    }
                }
                return Ok(result_reg);
            }
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
                            ("double", "double") => {
                                context.emit(&format!("    fsd {}, {}(s0)", rhs_reg, offset));
                                return Ok(rhs_reg)
                            },
                            _ => return Err(CompileError::CodegenError(
                                format!("Type mismatch in assignment: {} = {}", lhs_type, rhs_type)
                            )),
                        }
                    } else {
                        return Err(CompileError::CodegenError(format!("Variable '{}' not found", name)))
                    }
                },
                AstNode::MemberAccess { object, member } => {
                    let member_str = member.clone();

                    match &**object {
                        AstNode::Identifier(var_name) => {
                            let var_name = var_name.clone();
                            let (_type_info, location, field_offset, field_type) = {
                                let symbol = context.lookup_symbol(&var_name).ok_or_else(|| CompileError::CodegenError(format!("Unknown variable: {}", var_name)))?;
                                let type_info = symbol.type_info.clone();
                                if !type_info.starts_with("struct ") {
                                    return Err(CompileError::CodegenError(
                                        format!("Variable {} is not a struct", var_name)
                                    ));
                                }

                                let struct_name = type_info["struct ".len()..].to_string();
                                let struct_def = context.struct_definitions.get(&struct_name).ok_or_else(|| CompileError::CodegenError(format!("Unknown struct type: {}", struct_name)))?;
                                let field_info = struct_def.fields.get(&member_str).ok_or_else(|| CompileError::CodegenError(format!("Struct {} has no member named {}", struct_name, member_str)))?;
                                (
                                    type_info,
                                    symbol.location.clone(),
                                    field_info.offset as i32,
                                    field_info.type_info.clone()
                                )
                            };

                            match location {
                                StorageLocation::Stack(stack_offset) => {
                                    let member_offset = stack_offset + field_offset;
                                    context.emit(&format!("    # Assign to struct member {}.{}", var_name, member_str));

                                    match (field_type.as_str(), rhs_type.as_str()) {
                                        ("int", "int") => {
                                            context.emit(&format!("    sw {}, {}(s0)", rhs_reg, member_offset));
                                        },
                                        ("float", "float") => {
                                            if rhs_reg.starts_with('f') {
                                                context.emit(&format!("    fsw {}, {}(s0)", rhs_reg, member_offset));
                                            } else {
                                                return Err(CompileError::CodegenError(
                                                    "Expected floating-point register for float assignment".to_string()
                                                ));
                                            }
                                        },
                                        ("double", "double") => {
                                            if rhs_reg.starts_with('f') {
                                                context.emit(&format!("    fsd {}, {}(s0)", rhs_reg, member_offset));
                                            } else {
                                                return Err(CompileError::CodegenError(
                                                    "Expected floating-point register for double assignment".to_string()
                                                ));
                                            }
                                        },
                                        _ => {
                                            return Err(CompileError::CodegenError(
                                                format!("Type mismatch in struct member assignment: {} = {}", field_type, rhs_type)
                                            ));
                                        }
                                    }
                                },
                                StorageLocation::Global(label) => {
                                    let addr_reg = context.get_register();
                                    context.emit(&format!("    # Assign to struct member {}.{}", var_name, member_str));

                                    context.emit(&format!("    la {}, {}", addr_reg, label));
                                    match (field_type.as_str(), rhs_type.as_str()) {
                                        ("int", "int") => {
                                            context.emit(&format!("    sw {}, {}({})", rhs_reg, field_offset, addr_reg));
                                        },
                                        ("float", "float") => {
                                            if rhs_reg.starts_with('f') {
                                                context.emit(&format!("    fsw {}, {}({})", rhs_reg, field_offset, addr_reg));
                                            } else {
                                                context.free_register(&addr_reg);
                                                return Err(CompileError::CodegenError(
                                                    "Expected floating-point register for float assignment".to_string()
                                                ));
                                            }
                                        },
                                        ("double", "double") => {
                                            if rhs_reg.starts_with('f') {
                                                context.emit(&format!("    fsd {}, {}({})", rhs_reg, field_offset, addr_reg));
                                            } else {
                                                context.free_register(&addr_reg);
                                                return Err(CompileError::CodegenError(
                                                    "Expected floating-point register for double assignment".to_string()
                                                ));
                                            }
                                        },
                                        _ => {
                                            context.free_register(&addr_reg);
                                            return Err(CompileError::CodegenError(
                                                format!("Type mismatch in struct member assignment: {} = {}", field_type, rhs_type)
                                            ));
                                        }
                                    }
                                    context.free_register(&addr_reg);
                                },
                                _ => {
                                    return Err(CompileError::CodegenError(
                                        format!("Unsupported storage location for struct variable: {:?}", location)
                                    ));
                                }
                            }

                            return Ok(rhs_reg);
                        },
                        _ => {
                            return Err(CompileError::CodegenError(
                                "Complex struct member access for assignment not supported yet".to_string()
                            ));
                        }
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

            if left_type == "float" || right_type == "float" {
                generate_fp_binary_operation(op, left, right, context)
            } else if left_type == "double" || right_type == "double" {
                generate_double_binary_operation(op, left, right, context)
            } else {
                generate_binary_operation(op, left, right, context)
            }
        },
        AstNode::UnaryOperation { op: _op, operand } => generate_unary_operation(_op, operand, context),
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
        AstNode::TernaryOperation { condition, true_expr, false_expr } => {
            generate_ternary_operation(condition, true_expr, false_expr, context)
        },
        AstNode::MemberAccess { object, member } => {
            generate_member_access(object, member, context)
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
            generate_expression(operand, context)
        },

        // Unary minus: negate the value
        "-" => {
            let operand_type = get_expression_type(operand, context)?;
            let operand_reg = generate_expression(operand, context)?;
            match operand_type.as_str() {
                "int" => {
                    context.emit(&format!("    neg {0}, {0}", operand_reg));
                },
                "float" => {
                    context.emit(&format!("    fneg.s {0}, {0}", operand_reg));
                },
                "double" => {
                    context.emit(&format!("    fneg.d {0}, {0}", operand_reg));
                },
                _ => return Err(CompileError::CodegenError(format!(
                    "Unary minus not supported for type: {}", operand_type
                ))),
            }
            Ok(operand_reg)
        },

        // Bitwise NOT
        "~" => {
            let operand_type = get_expression_type(operand, context)?;
            if operand_type != "int" {
                return Err(CompileError::CodegenError(
                    "Bitwise NOT only supported for integers".to_string()
                ));
            }
            let operand_reg = generate_expression(operand, context)?;
            context.emit(&format!("    not {0}, {0}", operand_reg));
            Ok(operand_reg)
        },

        // Logical NOT
        "!" => {
            let operand_type = get_expression_type(operand, context)?;
            let operand_reg = generate_expression(operand, context)?;
            let result_reg = context.get_register(); // Integer result (0 or 1)
            match operand_type.as_str() {
                "int" => {
                    context.emit(&format!("    seqz {}, {}", result_reg, operand_reg));
                },
                "float" => {
                    context.emit(&format!("    fmv.s.x f0, zero")); // 0.0 in f0
                    context.emit(&format!("    feq.s {}, {}, f0", result_reg, operand_reg));
                },
                "double" => {
                    context.emit(&format!("    fmv.d.x f0, zero")); // 0.0 in f0
                    context.emit(&format!("    feq.d {}, {}, f0", result_reg, operand_reg));
                },
                _ => return Err(CompileError::CodegenError(format!(
                    "Logical NOT not supported for type: {}", operand_type
                ))),
            }
            if operand_type == "int" {
                context.free_register(&operand_reg);
            } else {
                context.free_fp_register(&operand_reg);
            }
            Ok(result_reg)
        },

        // Pre-increment: increment operand, then return new value
        "++" => {
            if let AstNode::Identifier(var_name) = operand {
                if let Some((offset, type_info)) = context.get_variable(var_name) {
                    match type_info.as_str() {
                        "int" => {
                            let result_reg = context.get_register();
                            context.emit(&format!("    lw {}, {}(s0)", result_reg, offset));
                            context.emit(&format!("    addi {}, {}, 1", result_reg, result_reg));
                            context.emit(&format!("    sw {}, {}(s0)", result_reg, offset));
                            return Ok(result_reg);
                        },
                        "float" => {
                            let result_reg = context.get_fp_register();
                            context.emit(&format!("    flw {}, {}(s0)", result_reg, offset));
                            let one_reg = generate_float_constant(1.0, context)?;
                            context.emit(&format!("    fadd.s {}, {}, {}", result_reg, result_reg, one_reg));
                            context.emit(&format!("    fsw {}, {}(s0)", result_reg, offset));
                            context.free_fp_register(&one_reg);
                            return Ok(result_reg);
                        },
                        "double" => {
                            let result_reg = context.get_fp_register();
                            context.emit(&format!("    fld {}, {}(s0)", result_reg, offset));
                            let one_reg = generate_double_constant(1.0, context)?;
                            context.emit(&format!("    fadd.d {}, {}, {}", result_reg, result_reg, one_reg));
                            context.emit(&format!("    fsd {}, {}(s0)", result_reg, offset));
                            context.free_fp_register(&one_reg);
                            return Ok(result_reg);
                        },
                        _ => return Err(CompileError::CodegenError(format!(
                            "Increment not supported for type: {}", type_info
                        ))),
                    }
                }
            }
            Err(CompileError::CodegenError("Invalid operand for ++ operation".to_string()))
        },

        // Pre-decrement: decrement operand, then return new value
        "--" => {
            if let AstNode::Identifier(var_name) = operand {
                if let Some((offset, type_info)) = context.get_variable(var_name) {
                    match type_info.as_str() {
                        "int" => {
                            let result_reg = context.get_register();
                            context.emit(&format!("    lw {}, {}(s0)", result_reg, offset));
                            context.emit(&format!("    addi {}, {}, -1", result_reg, result_reg));
                            context.emit(&format!("    sw {}, {}(s0)", result_reg, offset));
                            return Ok(result_reg);
                        },
                        "float" => {
                            let result_reg = context.get_fp_register();
                            context.emit(&format!("    flw {}, {}(s0)", result_reg, offset));
                            let minus_one_reg = generate_float_constant(-1.0, context)?;
                            context.emit(&format!("    fadd.s {}, {}, {}", result_reg, result_reg, minus_one_reg));
                            context.emit(&format!("    fsw {}, {}(s0)", result_reg, offset));
                            context.free_fp_register(&minus_one_reg);
                            return Ok(result_reg);
                        },
                        "double" => {
                            let result_reg = context.get_fp_register();
                            context.emit(&format!("    fld {}, {}(s0)", result_reg, offset));
                            let minus_one_reg = generate_double_constant(-1.0, context)?;
                            context.emit(&format!("    fadd.d {}, {}, {}", result_reg, result_reg, minus_one_reg));
                            context.emit(&format!("    fsd {}, {}(s0)", result_reg, offset));
                            context.free_fp_register(&minus_one_reg);
                            return Ok(result_reg);
                        },
                        _ => return Err(CompileError::CodegenError(format!(
                            "Decrement not supported for type: {}", type_info
                        ))),
                    }
                }
            }
            Err(CompileError::CodegenError("Invalid operand for -- operation".to_string()))
        },

        // Post-increment: return original value, then increment
        "post++" => {
            if let AstNode::Identifier(var_name) = operand {
                if let Some((offset, type_info)) = context.get_variable(var_name) {
                    match type_info.as_str() {
                        "int" => {
                            let result_reg = context.get_register();
                            let temp_reg = context.get_register();
                            context.emit(&format!("    lw {}, {}(s0)", result_reg, offset));
                            context.emit(&format!("    mv {}, {}", temp_reg, result_reg));
                            context.emit(&format!("    addi {}, {}, 1", temp_reg, temp_reg));
                            context.emit(&format!("    sw {}, {}(s0)", temp_reg, offset));
                            context.free_register(&temp_reg);
                            return Ok(result_reg);
                        },
                        "float" => {
                            let result_reg = context.get_fp_register();
                            let temp_reg = context.get_fp_register();
                            context.emit(&format!("    flw {}, {}(s0)", result_reg, offset));
                            context.emit(&format!("    fmv.s {}, {}", temp_reg, result_reg));
                            let one_reg = generate_float_constant(1.0, context)?;
                            context.emit(&format!("    fadd.s {}, {}, {}", temp_reg, temp_reg, one_reg));
                            context.emit(&format!("    fsw {}, {}(s0)", temp_reg, offset));
                            context.free_fp_register(&temp_reg);
                            context.free_fp_register(&one_reg);
                            return Ok(result_reg);
                        },
                        "double" => {
                            let result_reg = context.get_fp_register();
                            let temp_reg = context.get_fp_register();
                            context.emit(&format!("    fld {}, {}(s0)", result_reg, offset));
                            context.emit(&format!("    fmv.d {}, {}", temp_reg, result_reg));
                            let one_reg = generate_double_constant(1.0, context)?;
                            context.emit(&format!("    fadd.d {}, {}, {}", temp_reg, temp_reg, one_reg));
                            context.emit(&format!("    fsd {}, {}(s0)", temp_reg, offset));
                            context.free_fp_register(&temp_reg);
                            context.free_fp_register(&one_reg);
                            return Ok(result_reg);
                        },
                        _ => return Err(CompileError::CodegenError(format!(
                            "Post-increment not supported for type: {}", type_info
                        ))),
                    }
                }
            }
            Err(CompileError::CodegenError("Invalid operand for post++ operation".to_string()))
        },

        // Post-decrement: return original value, then decrement
        "post--" => {
            if let AstNode::Identifier(var_name) = operand {
                if let Some((offset, type_info)) = context.get_variable(var_name) {
                    match type_info.as_str() {
                        "int" => {
                            let result_reg = context.get_register();
                            let temp_reg = context.get_register();
                            context.emit(&format!("    lw {}, {}(s0)", result_reg, offset));
                            context.emit(&format!("    mv {}, {}", temp_reg, result_reg));
                            context.emit(&format!("    addi {}, {}, -1", temp_reg, temp_reg));
                            context.emit(&format!("    sw {}, {}(s0)", temp_reg, offset));
                            context.free_register(&temp_reg);
                            return Ok(result_reg);
                        },
                        "float" => {
                            let result_reg = context.get_fp_register();
                            let temp_reg = context.get_fp_register();
                            context.emit(&format!("    flw {}, {}(s0)", result_reg, offset));
                            context.emit(&format!("    fmv.s {}, {}", temp_reg, result_reg));
                            let minus_one_reg = generate_float_constant(-1.0, context)?;
                            context.emit(&format!("    fadd.s {}, {}, {}", temp_reg, temp_reg, minus_one_reg));
                            context.emit(&format!("    fsw {}, {}(s0)", temp_reg, offset));
                            context.free_fp_register(&temp_reg);
                            context.free_fp_register(&minus_one_reg);
                            return Ok(result_reg);
                        },
                        "double" => {
                            let result_reg = context.get_fp_register();
                            let temp_reg = context.get_fp_register();
                            context.emit(&format!("    fld {}, {}(s0)", result_reg, offset));
                            context.emit(&format!("    fmv.d {}, {}", temp_reg, result_reg));
                            let minus_one_reg = generate_double_constant(-1.0, context)?;
                            context.emit(&format!("    fadd.d {}, {}, {}", temp_reg, temp_reg, minus_one_reg));
                            context.emit(&format!("    fsd {}, {}(s0)", temp_reg, offset));
                            context.free_fp_register(&temp_reg);
                            context.free_fp_register(&minus_one_reg);
                            return Ok(result_reg);
                        },
                        _ => return Err(CompileError::CodegenError(format!(
                            "Post-decrement not supported for type: {}", type_info
                        ))),
                    }
                }
            }
            Err(CompileError::CodegenError("Invalid operand for post-- operation".to_string()))
        },

        // Pointer dereference
        "*" => {
            let addr_reg = generate_expression(operand, context)?;
            let result_reg = context.get_register();
            context.emit(&format!("    lw {}, 0({})", result_reg, addr_reg));
            if addr_reg != result_reg {
                context.free_register(&addr_reg);
            }
            Ok(result_reg)
            // Note: Currently assumes integer pointers. For float/double, use flw/fld with type info.
        },

        // Address-of operator
        "&" => {
            if let AstNode::Identifier(var_name) = operand {
                if let Some((offset, _)) = context.get_variable(var_name) {
                    let result_reg = context.get_register();
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

    match left {
        AstNode::Identifier(var_name) => {
            let var_name = var_name.clone(); // Memory safety shoutout rust

            if let Some(symbol) = context.lookup_symbol(&var_name) {
                let type_info = symbol.type_info.clone();
                let location = symbol.location.clone();

                match location {
                    StorageLocation::Stack(offset) => {
                        if type_info == "float" {
                            if !right_reg.starts_with('f') {
                                let fp_reg = context.get_fp_register();
                                context.emit(&format!("    fcvt.s.w {}, {}", fp_reg, right_reg));
                                context.emit(&format!("    fsw {}, {}(s0)", fp_reg, offset));
                                context.free_fp_register(&fp_reg);
                            } else {
                                context.emit(&format!("    fsw {}, {}(s0)", right_reg, offset));
                            }
                        } else if type_info == "double" {
                            if !right_reg.starts_with('f') {
                                let fp_reg = context.get_fp_register();
                                context.emit(&format!("    fcvt.d.w {}, {}", fp_reg, right_reg));
                                context.emit(&format!("    fsd {}, {}(s0)", fp_reg, offset));
                                context.free_fp_register(&fp_reg);
                            } else {
                                context.emit(&format!("    fsd {}, {}(s0)", right_reg, offset));
                            }
                        } else {
                            context.emit(&format!("    sw {}, {}(s0)", right_reg, offset));
                        }
                    },
                    StorageLocation::Global(label) => {
                        let addr_reg = context.get_register();
                        context.emit(&format!("    la {}, {}", addr_reg, label));

                        if type_info == "float" {
                            if !right_reg.starts_with('f') {
                                let fp_reg = context.get_fp_register();
                                context.emit(&format!("    fcvt.s.w {}, {}", fp_reg, right_reg));
                                context.emit(&format!("    fsw {}, 0({})", fp_reg, addr_reg));
                                context.free_fp_register(&fp_reg);
                            } else {
                                context.emit(&format!("    fsw {}, 0({})", right_reg, addr_reg));
                            }
                        } else if type_info == "double" {
                            if !right_reg.starts_with('f') {
                                let fp_reg = context.get_fp_register();
                                context.emit(&format!("    fcvt.d.w {}, {}", fp_reg, right_reg));
                                context.emit(&format!("    fsd {}, 0({})", fp_reg, addr_reg));
                                context.free_fp_register(&fp_reg);
                            } else {
                                context.emit(&format!("    fsd {}, 0({})", right_reg, addr_reg));
                            }
                        } else {
                            context.emit(&format!("    sw {}, 0({})", right_reg, addr_reg));
                        }
                        context.free_register(&addr_reg);
                    },
                    _ => return Err(CompileError::CodegenError(format!("Unsupported storage location: {:?}", location))),
                }
                return Ok(right_reg);
            }
        },
        AstNode::MemberAccess { object, member } => {
            if let AstNode::Identifier(var_name) = object.as_ref() {
                let var_name = var_name.clone();
                let member = member.clone();

                if let Some(symbol) = context.lookup_symbol(&var_name) {
                    let _type_info = symbol.type_info.clone();

                    if !_type_info.starts_with("struct ") {
                        return Err(CompileError::CodegenError(
                            format!("Variable {} is not a struct", var_name)
                        ));
                    }

                    let struct_name = _type_info["struct ".len()..].to_string();

                    let field_info = if let Some(struct_def) = context.struct_definitions.get(&struct_name) {
                        if let Some(field_info) = struct_def.fields.get(&member){
                            field_info.type_info.clone()
                        } else{
                            return Err(CompileError::CodegenError(
                                format!("Struct {} has no member named {}", struct_name, member)
                            ));
                        }
                    } else{
                        return Err(CompileError::CodegenError(
                            format!("Unknown struct type: {}", struct_name)
                        ));
                    };
                    return Ok(field_info);
                } else {
                    return Err(CompileError::CodegenError(format!("Unknown variable: {}", var_name)));
                }
            } else {
                return Err(CompileError::CodegenError("Complex struct member access not supported".to_string()));
            }
        },
        _ => {
            let left_reg = generate_expression(left, context)?;

            context.emit(&format!("    sw {}, 0({})", right_reg, left_reg));

            context.free_register(&left_reg);
        }
    }

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
        println!("Debug arg{}: arg_type {}, for function call {}", i, arg_type, func_name);
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
                    println!("Debug fcvt.w.s a{}, {} for function call {}", i, arg_reg, func_name);
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

fn generate_double_constant(value: f64, context: &mut CodeGenContext) -> Result<String, CompileError> {
    let reg = context.get_fp_register();
    let label = context.generate_label("double_const");
    context.emit_data(&format!("{}:", label));
    context.emit_data(&format!("    .double {}", value));
    let temp_reg = context.get_register();
    context.emit(&format!("    la {}, {}", temp_reg, label));
    context.emit(&format!("    fld {}, 0({})", reg, temp_reg));
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
    let left_type = get_expression_type(left, context)?;  // For debugging or later use
    let right_type = get_expression_type(right, context)?;

    let left_fp_reg = if !left_reg.starts_with('f') {
        let fp_reg = context.get_fp_register();
        context.emit(&format!("    fcvt.s.w {}, {}", fp_reg, left_reg));
        context.free_register(&left_reg);
        fp_reg
    } else {
        left_reg
    };

    let right_fp_reg = if !right_reg.starts_with('f') {
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

fn generate_double_binary_operation(
    op: &str,
    left: &AstNode,
    right: &AstNode,
    context: &mut CodeGenContext
) -> Result<String, CompileError> {
    // Generate code for operands
    let left_reg = generate_expression(left, context)?;
    let right_reg = generate_expression(right, context)?;

    // Determine operand types (e.g., "double" or "float")
    let left_type = get_expression_type(left, context)?;
    let right_type = get_expression_type(right, context)?;

    // Convert left operand to double if needed
    let left_fp_reg = if !left_reg.starts_with('f') && left_type == "double" {
        let fp_reg = context.get_fp_register();
        context.emit(&format!("    fcvt.d.w {}, {}", fp_reg, left_reg));
        context.free_register(&left_reg);
        fp_reg
    } else if !left_reg.starts_with('f') && left_type == "float" {
        // Convert single precision float to double precision
        let fp_reg = context.get_fp_register();
        context.emit(&format!("    fcvt.d.s {}, {}", fp_reg, left_reg));
        context.free_register(&left_reg);
        fp_reg
    } else {
        left_reg
    };

    // Convert right operand to double if needed
    let right_fp_reg = if !right_reg.starts_with('f') && right_type == "double" {
        let fp_reg = context.get_fp_register();
        context.emit(&format!("    fcvt.d.w {}, {}", fp_reg, right_reg));
        context.free_register(&right_reg);
        fp_reg
    } else if !right_reg.starts_with('f') && right_type == "float" {
        // Convert single precision float to double precision
        let fp_reg = context.get_fp_register();
        context.emit(&format!("    fcvt.d.s {}, {}", fp_reg, right_reg));
        context.free_register(&right_reg);
        fp_reg
    } else {
        right_reg
    };

    // Handle floating-point operations using double-precision instructions
    let result_reg = match op {
        "+" => {
            context.emit(&format!("    fadd.d {}, {}, {}", left_fp_reg, left_fp_reg, right_fp_reg));
            left_fp_reg
        },
        "-" => {
            context.emit(&format!("    fsub.d {}, {}, {}", left_fp_reg, left_fp_reg, right_fp_reg));
            left_fp_reg
        },
        "*" => {
            context.emit(&format!("    fmul.d {}, {}, {}", left_fp_reg, left_fp_reg, right_fp_reg));
            left_fp_reg
        },
        "/" => {
            context.emit(&format!("    fdiv.d {}, {}, {}", left_fp_reg, left_fp_reg, right_fp_reg));
            left_fp_reg
        },
        // Comparisons: the result is stored in an integer register
        "==" => {
            let int_reg = context.get_register();
            context.emit(&format!("    feq.d {}, {}, {}", int_reg, left_fp_reg, right_fp_reg));
            context.free_fp_register(&left_fp_reg);
            context.free_fp_register(&right_fp_reg);
            int_reg
        },
        "!=" => {
            let int_reg = context.get_register();
            context.emit(&format!("    feq.d {}, {}, {}", int_reg, left_fp_reg, right_fp_reg));
            context.emit(&format!("    xori {}, {}, 1", int_reg, int_reg));
            context.free_fp_register(&left_fp_reg);
            context.free_fp_register(&right_fp_reg);
            int_reg
        },
        "<" => {
            let int_reg = context.get_register();
            context.emit(&format!("    flt.d {}, {}, {}", int_reg, left_fp_reg, right_fp_reg));
            context.free_fp_register(&left_fp_reg);
            context.free_fp_register(&right_fp_reg);
            int_reg
        },
        ">" => {
            let int_reg = context.get_register();
            context.emit(&format!("    flt.d {}, {}, {}", int_reg, right_fp_reg, left_fp_reg));
            context.free_fp_register(&left_fp_reg);
            context.free_fp_register(&right_fp_reg);
            int_reg
        },
        "<=" => {
            let int_reg = context.get_register();
            context.emit(&format!("    fle.d {}, {}, {}", int_reg, left_fp_reg, right_fp_reg));
            context.free_fp_register(&left_fp_reg);
            context.free_fp_register(&right_fp_reg);
            int_reg
        },
        ">=" => {
            let int_reg = context.get_register();
            context.emit(&format!("    fle.d {}, {}, {}", int_reg, right_fp_reg, left_fp_reg));
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

fn generate_ternary_operation(
    condition: &AstNode,
    true_expr: &AstNode,
    false_expr: &AstNode,
    context: &mut CodeGenContext
) -> Result<String, CompileError> {
    let cond_reg = generate_expression(condition, context)?;
    let true_label = context.generate_label("ternary_true");
    let end_label = context.generate_label("ternary_end");

    context.emit(&format!("    bnez {}, {}", cond_reg, true_label));
    context.free_register(&cond_reg);

    let false_reg = generate_expression(false_expr, context)?;
    context.emit(&format!("    j {}", end_label));

    context.emit(&format!("{}:", true_label));
    let true_reg = generate_expression(true_expr, context)?;

    context.emit(&format!("{}:", end_label));

    let mut result_reg = context.get_register();
    if true_reg != false_reg {
        context.emit(&format!("    mv {}, {}", result_reg, true_reg));
        context.free_register(&true_reg);
        context.free_register(&false_reg);
    } else {
        result_reg = true_reg;
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
        AstNode::MemberAccess { object, member } => {
            if let AstNode::Identifier(var_name) = object.as_ref() {
                let var_name = var_name.clone();
                let member = member.clone();

                let field_type = {
                    if let Some(symbol) = context.lookup_symbol(&var_name){
                        let type_info = symbol.type_info.clone();

                        if !type_info.starts_with("struct "){
                            return Err(CompileError::CodegenError(
                                format!("Variable {} is not a struct", var_name)
                            ));
                        }
                        let struct_name = type_info["struct ".len()..].to_string();

                        if let Some(struct_def) = context.struct_definitions.get(&struct_name){
                            if let Some(field_info) = struct_def.fields.get(&member) {
                                field_info.type_info.clone()
                            } else {
                                return Err(CompileError::CodegenError(
                                    format!("Struct {} has no member named {}", struct_name, member)
                                ));
                            }
                        } else {
                            return Err(CompileError::CodegenError(
                                format!("Unknown struct type: {}", struct_name)
                            ));
                        }
                    } else {
                        return Err(CompileError::CodegenError(format!("Unknown variable: {}", var_name)));
                    }
                };
                return Ok(field_type);
            } else {
                return Err(CompileError::CodegenError("Complex struct member access not supported".to_string()));
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
        AstNode::UnaryOperation { operand, .. } => {
            let operand_type = get_expression_type(operand, context)?;
            if operand_type == "float" {
                Ok("float".to_string())
            } else if operand_type == "double" {
                Ok("double".to_string())
            } else {
                Ok("int".to_string())
            }
        },
        // Handle other expression types
        _ => Ok("int".to_string()), // Default to int for unknown expressions
    }
}

/// Generate code for a struct member access
fn generate_member_access(object: &AstNode, member: &str, context: &mut CodeGenContext) -> Result<String, CompileError> {
    let member_str = member.to_string();

    match object {
        AstNode::Identifier(var_name) => {
            let var_name = var_name.clone();

            let (_type_info, location, field_offset, field_type) = {
                let symbol = context.lookup_symbol(&var_name).ok_or_else(|| CompileError::CodegenError(format!("Unknown variable: {}", var_name)))?;

                let type_info = symbol.type_info.clone();
                if !type_info.starts_with("struct "){
                    return Err(CompileError::CodegenError(
                        format!("Variable {} is not a struct", var_name)
                    ));
                }
                let struct_name = type_info["struct ".len()..].to_string();

                let struct_def = context.struct_definitions.get(&struct_name).ok_or_else(|| CompileError::CodegenError(format!("Unknown struct type: {}", struct_name)))?;
                let field_info = struct_def.fields.get(&member_str).ok_or_else(|| CompileError::CodegenError(format!("Struct {} has no member named {}", struct_name, member_str)))?;

                (
                    type_info,
                    symbol.location.clone(),
                    field_info.offset as i32,
                    field_info.type_info.clone()
                )
            };
            let result_reg = context.get_register();

            match location {
                StorageLocation::Stack(stack_offset) => {
                    let member_offset = stack_offset + field_offset;
                    context.emit(&format!("    # Access struct member {}.{}", var_name, member_str));

                    if field_type == "int" {
                        context.emit(&format!("    lw {}, {}(s0)", result_reg, member_offset));
                    } else if field_type == "float" {
                        let fp_reg = context.get_fp_register();
                        context.emit(&format!("    flw {}, {}(s0)", fp_reg, member_offset));
                        context.free_register(&result_reg);
                        return Ok(fp_reg);
                    } else if field_type == "double" {
                        let fp_reg = context.get_fp_register();
                        context.emit(&format!("    fld {}, {}(s0)", fp_reg, member_offset));
                        context.free_register(&result_reg);
                        return Ok(fp_reg);
                    } else {
                        context.emit(&format!("    lw {}, {}(s0)", result_reg, member_offset));
                    }
                },
                StorageLocation::Global(label) => {
                    let addr_reg = context.get_register();
                    context.emit(&format!("    # Access struct member {}.{}", var_name, member_str));
                    context.emit(&format!("    la {}, {}", addr_reg, label));

                    if field_type == "int" {
                        context.emit(&format!("    lw {}, {}({})", result_reg, field_offset, addr_reg));
                    } else if field_type == "float" {
                        let fp_reg = context.get_fp_register();
                        context.emit(&format!("    flw {}, {}({})", fp_reg, field_offset, addr_reg));
                        context.free_register(&result_reg);
                        context.free_register(&addr_reg);
                        return Ok(fp_reg);
                    } else if field_type == "double" {
                        let fp_reg = context.get_fp_register();
                        context.emit(&format!("    fld {}, {}({})", fp_reg, field_offset, addr_reg));
                        context.free_register(&result_reg);
                        context.free_register(&addr_reg);
                        return Ok(fp_reg);
                    } else {
                        context.emit(&format!("    lw {}, {}({})", result_reg, field_offset, addr_reg));
                    }
                    context.free_register(&addr_reg);
                },
                _ => {
                    context.free_register(&result_reg);
                    return Err(CompileError::CodegenError(
                        format!("Unsupported storage location for struct variable: {:?}", location)
                    ));
                }
            }

            Ok(result_reg)
        },
        _ => {
            Err(CompileError::CodegenError(
                "Complex struct member access not supported yet".to_string()
            ))
        }
    }
}
