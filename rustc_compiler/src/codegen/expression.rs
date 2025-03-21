use crate::ast::AstNode;
use crate::codegen::context::CodeGenContext;
use crate::codegen::context::StorageLocation;
use crate::ast::TypeSpecifier;
use crate::error::CompileError;

pub fn generate_expression(node: &AstNode, context: &mut CodeGenContext) -> Result<String, CompileError> {
    match node {
        AstNode::Identifier(name) => {
            if let Some(enum_value) = context.lookup_enum_value(name) {
                let reg = context.get_register();
                context.emit(&format!("    li {}, {}", reg, enum_value));
                return Ok(reg);
            } else if let Some(symbol) = context.lookup_symbol(name) {
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
            let rhs_reg = generate_expression(rhs, context)?;
            let rhs_type = get_expression_type(rhs, context)?;

            match &**lhs {
                AstNode::Identifier(name) => {
                    if let Some((offset, lhs_type)) = context.get_variable(name) {
                        if lhs_type.ends_with('*') && rhs_type.ends_with('*') {
                            context.emit(&format!("    sw {}, {}(s0)", rhs_reg, offset));
                            return Ok(rhs_reg);
                        }

                        match (lhs_type.as_str(), rhs_type.as_str()) {
                            ("int", "int") => {
                                context.emit(&format!("    sw {}, {}(s0)", rhs_reg, offset));
                                return Ok(rhs_reg)
                            },
                            ("float", "float") => {
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
                                let fp_reg = context.get_fp_register();
                                context.emit(&format!("    fcvt.s.w {}, {}", fp_reg, rhs_reg));
                                context.emit(&format!("    fsw {}, {}(s0)", fp_reg, offset));
                                context.free_register(&rhs_reg);
                                context.free_fp_register(&fp_reg);
                                return Ok(fp_reg)
                            },
                            ("int", "float") => {
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

                            return Ok(rhs_reg); // Using return here to force out the mismatched error
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
                    let (addr_reg, element_size) = calculate_array_element_address(&base, &indices, context)?;
                    if element_size == 1 {
                        context.emit(&format!("    sb {}, 0({})", rhs_reg, addr_reg));
                    } else {
                        context.emit(&format!("    sw {}, 0({})", rhs_reg, addr_reg));
                    }
                    context.free_register(&addr_reg);
                    return Ok(rhs_reg);
                },
                AstNode::UnaryOperation { op, operand } if op == "*" => {
                    match &**operand {
                        AstNode::Identifier(name) => {
                            if let Some((offset, lhs_type)) = context.get_variable(name) {
                                if lhs_type.ends_with('*') {
                                    let target_type = lhs_type.trim_end_matches('*');
                                    let addr_reg = context.get_register();
                                    context.emit(&format!("    lw {}, {}(s0)", addr_reg, offset));

                                    match (target_type, rhs_type.as_str()) {
                                        ("int", "int") => {
                                            context.emit(&format!("    sw {}, 0({})", rhs_reg, addr_reg));
                                        },
                                        ("float", "float") => {
                                            if rhs_reg.starts_with('f') {
                                                context.emit(&format!("    fsw {}, 0({})", rhs_reg, addr_reg));
                                            } else {
                                                return Err(CompileError::CodegenError(
                                                    "Expected floating-point register for float assignment".to_string()
                                                ));
                                            }
                                        },
                                        ("double", "double") => {
                                            context.emit(&format!("    fsd {}, 0({})", rhs_reg, addr_reg));
                                        },
                                        _ => {
                                            context.free_register(&addr_reg);
                                            return Err(CompileError::CodegenError(
                                                format!("Type mismatch in dereference assignment: *{} = {}", lhs_type, rhs_type)
                                            ));
                                        }
                                    }
                                    context.free_register(&addr_reg);
                                    return Ok(rhs_reg);
                                } else {
                                    return Err(CompileError::CodegenError(
                                        format!("Variable '{}' is not a pointer for dereference", name)
                                    ));
                                }
                            } else {
                                return Err(CompileError::CodegenError(format!("Variable '{}' not found", name)));
                            }
                        },
                        _ => return Err(CompileError::CodegenError(
                            "Dereference operator can only be applied to identifiers".to_string()
                        )),
                    }
                },
                _ => {},
            }
            Err(CompileError::CodegenError(format!("Invalid assignment target attempt to assign {} from register {}", rhs_type, rhs_reg)))
        },
        AstNode::IntegerLiteral(value) => generate_int_constant(*value, context),
        AstNode::BinaryOperation { op, left, right } => {
            let left_type = get_expression_type(left, context)?;
            let right_type = get_expression_type(right, context)?;

            if left_type == "float" || right_type == "float" {
                generate_fp_binary_operation(op, left, right, context)
            } else if left_type == "double" || right_type == "double" {
                generate_double_binary_operation(op, left, right, context)
            } else {
                generate_binary_operation(op, left, right, &left_type, &right_type, context)
            }
        },
        AstNode::UnaryOperation { op: _op, operand } => generate_unary_operation(_op, operand, context),
        AstNode::IntConstant(value) => {
            let reg = context.get_register();
            context.emit(&format!("    li {}, {}", reg, value));
            Ok(reg)
        },
        AstNode::FunctionCall { function, args } => {
            generate_function_call(function, args, context)
        },
        // Array subscript so like x[y] kind of thing
        AstNode::ArraySubscript { array, index } => {
            let (base, indices) = collect_array_access(node)?;
            let (addr_reg, element_size) = calculate_array_element_address(&base, &indices, context)?;
            let result_reg = context.get_register();

            if element_size == 1 {
                context.emit(&format!("    lb {}, 0({})", result_reg, addr_reg));
                context.emit(&format!("    # Sign-extend char to word"));
                context.emit(&format!("    slli {0}, {0}, 24", result_reg));
                context.emit(&format!("    srai {0}, {0}, 24", result_reg));
            } else {
                context.emit(&format!("    lw {}, 0({})", result_reg, addr_reg));
            }

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
        AstNode::SizeofType { type_spec, pointer_level } => {
            let size = get_type_size(type_spec, *pointer_level, context);
            let result_reg = context.get_register();
            context.emit(&format!("    # sizeof type {}{}", type_spec, "*".repeat(*pointer_level)));
            context.emit(&format!("    li {}, {}", result_reg, size));
            Ok(result_reg)
        },

        AstNode::SizeofExpr { expr } => {
            let expr_type = get_expression_type(expr, context)?;
            let size = get_size_from_type_string(&expr_type, context);
            let result_reg = context.get_register();
            context.emit(&format!("    # sizeof expression with type {}", expr_type));
            context.emit(&format!("    li {}, {}", result_reg, size));
            Ok(result_reg)
        },
        AstNode::StringLiteral(value) => {
            let str_label = context.generate_label("str");

            context.emit_data(&format!("{}:", str_label));
            context.emit_data(&format!("    .string \"{}\"", value));
            let reg = context.get_register();
            context.emit(&format!("    la {}, {}", reg, str_label));
            Ok(reg)
        },
        _ => Err(CompileError::CodegenError(format!("Unsupported expression type: {:?}", node))),
    }
}


fn generate_int_constant(value: i32, context: &mut CodeGenContext) -> Result<String, CompileError> {
    let reg = context.get_register();
    context.emit(&format!("    li {}, {}", reg, value));
    Ok(reg)
}

fn generate_identifier(name: &str, context: &mut CodeGenContext) -> Result<String, CompileError> {
    let location = match context.lookup_symbol(name) {
        Some(symbol) => symbol.location.clone(),
        None => return Err(CompileError::CodegenError(format!("Identifier '{}' not found", name))),
    };

    let result_reg = context.get_register();

    match location {
        crate::codegen::context::StorageLocation::Register(reg) => {
            context.emit(&format!("    mv {}, {}", result_reg, reg));
        },
        crate::codegen::context::StorageLocation::Stack(offset) => {
            context.emit(&format!("    lw {}, {}(s0)", result_reg, offset));
        },
        StorageLocation::Global(label) => {
            context.emit(&format!("    la {}, {}", result_reg, label));
            context.emit(&format!("    lw {}, 0({})", result_reg, result_reg));
        },
    }

    Ok(result_reg)
}

fn generate_binary_operation(op: &str, left: &AstNode, right: &AstNode, left_type: &str, right_type: &str, context: &mut CodeGenContext) -> Result<String, CompileError> {
    if op == "&&" || op == "||" {
        return generate_logical_operation(op, left, right, context);
    }

    if (op == "+" || op == "-") && (left_type.ends_with('*') || right_type.ends_with('*')) {
        return generate_pointer_arithmetic(op, left, right, left_type.to_string(), right_type.to_string(), context);
    }

    let common_type = if left_type == "unsigned" || right_type == "unsigned" {
        "unsigned"
    } else{
        "int" // Defaulting to signed just in case
    };

    let left_reg = generate_expression(left, context)?;
    let right_reg = generate_expression(right, context)?;

    let result_reg = match op {
        "+" => {
            context.emit(&format!("    add {0}, {1}, {2}", left_reg, left_reg, right_reg));
            left_reg
        }
        "-" => {
            context.emit(&format!("    sub {0}, {1}, {2}", left_reg, left_reg, right_reg));
            left_reg
        }
        "*" => {
            context.emit(&format!("    mul {0}, {1}, {2}", left_reg, left_reg, right_reg));
            left_reg
        }
        "/" => {
            if common_type == "unsigned" {
                context.emit(&format!("    divu {0}, {1}, {2}", left_reg, left_reg, right_reg));
            } else {
                context.emit(&format!("    div {0}, {1}, {2}", left_reg, left_reg, right_reg));
            }
            left_reg
        }
        "%" => {
            if common_type == "unsigned" {
                context.emit(&format!("    remu {0}, {1}, {2}", left_reg, left_reg, right_reg));
            } else {
                context.emit(&format!("    rem {0}, {1}, {2}", left_reg, left_reg, right_reg));
            }
            left_reg
        }
        "<<" => {
            context.emit(&format!("    sll {0}, {1}, {2}", left_reg, left_reg, right_reg));
            left_reg
        }
        ">>" => {
            if left_type == "unsigned" {
                context.emit(&format!("    srl {0}, {1}, {2}", left_reg, left_reg, right_reg));
            } else {
                context.emit(&format!("    sra {0}, {1}, {2}", left_reg, left_reg, right_reg));
            }
            left_reg
        }
        "!=" => {
            context.emit(&format!("    xor {0}, {1}, {2}", left_reg, left_reg, right_reg));
            context.emit(&format!("    snez {0}, {0}", left_reg));
            left_reg
        }
        "==" => {
            context.emit(&format!("    xor {0}, {1}, {2}", left_reg, left_reg, right_reg));
            context.emit(&format!("    seqz {0}, {0}", left_reg));
            left_reg
        }
        "<" => {
            if common_type == "unsigned" {
                context.emit(&format!("    sltu {0}, {1}, {2}", left_reg, left_reg, right_reg));
            } else {
                context.emit(&format!("    slt {0}, {1}, {2}", left_reg, left_reg, right_reg));
            }
            left_reg
        }
        ">" => {
            if common_type == "unsigned" {
                context.emit(&format!("    sltu {0}, {1}, {2}", left_reg, right_reg, left_reg));
            } else{
                context.emit(&format!("    slt {0}, {1}, {2}", left_reg, right_reg, left_reg));
            }
            left_reg
        }
        "<=" => {
            let temp_reg = context.get_register();
            if common_type == "unsigned" {
                context.emit(&format!("    sltu {0}, {1}, {2}", temp_reg, right_reg, left_reg));
            } else {
                context.emit(&format!("    slt {0}, {1}, {2}", temp_reg, right_reg, left_reg));
            }
            context.emit(&format!("    xori {0}, {0}, 1", temp_reg));
            context.emit(&format!("    mv {0}, {1}", left_reg, temp_reg));
            context.free_register(&temp_reg);
            left_reg
        }
        ">=" => {
            let temp_reg = context.get_register();
            if common_type == "unsigned" {
                context.emit(&format!("    sltu {0}, {1}, {2}", temp_reg, left_reg, right_reg));
            } else{
                context.emit(&format!("    slt {0}, {1}, {2}", temp_reg, left_reg, right_reg));
            }
            context.emit(&format!("    xori {0}, {0}, 1", temp_reg));
            context.emit(&format!("    mv {0}, {1}", left_reg, temp_reg));
            context.free_register(&temp_reg);
            left_reg
        }
        "&" => {
            context.emit(&format!("    and {0}, {1}, {2}", left_reg, left_reg, right_reg));
            left_reg
        }
        "|" => {
            context.emit(&format!("    or {0}, {1}, {2}", left_reg, left_reg, right_reg));
            left_reg
        }
        "^" => {
            context.emit(&format!("    xor {0}, {1}, {2}", left_reg, left_reg, right_reg));
            left_reg
        }
        _ => return Err(CompileError::CodegenError(format!("Invalid binary operation: {}", op))),
    };
    context.free_register(&right_reg);
    Ok(result_reg)
}

fn generate_pointer_arithmetic(op: &str, left: &AstNode, right: &AstNode, left_type: String, right_type: String, context: &mut CodeGenContext) -> Result<String, CompileError> {
    let left_reg = generate_expression(left, context)?;
    let right_reg = generate_expression(right, context)?;

    let (ptr_reg, int_reg, ptr_type, is_left_ptr) = if left_type.ends_with('*') {
        (left_reg.clone(), right_reg.clone(), left_type.trim_end_matches('*').to_string(), true)
    } else {
        (right_reg.clone(), left_reg.clone(), right_type.trim_end_matches('*').to_string(), false)
    };

    let element_size = if ptr_type.contains("char") {
        1
    } else if ptr_type.contains("double") {
        8
    } else {
        4
    };

    let result_reg = if op == "+" || (op == "-" && is_left_ptr) {
        let scaled_reg = context.get_register();

        if element_size == 1 {
            context.emit(&format!("    mv {}, {}", scaled_reg, int_reg));
        } else {
            context.emit(&format!("    li {}, {}", scaled_reg, element_size));
            context.emit(&format!("    mul {}, {}, {}", scaled_reg, int_reg, scaled_reg));
        }

        if op == "+" {
            context.emit(&format!("    add {}, {}, {}", ptr_reg, ptr_reg, scaled_reg));
        } else{
            context.emit(&format!("    sub {}, {}, {}", ptr_reg, ptr_reg, scaled_reg));
        }

        context.free_register(&scaled_reg);
        context.free_register(&int_reg);
        ptr_reg
    } else if op == "-" && left_type.ends_with('*') && right_type.ends_with('*') {
        let result_reg = context.get_register();
        context.emit(&format!("    sub {}, {}, {}", result_reg, left_reg, right_reg));

        if element_size > 1 {
            let size_reg = context.get_register();
            context.emit(&format!("    li {}, {}", size_reg, element_size));
            context.emit(&format!("    div {}, {}, {}", result_reg, result_reg, size_reg));
            context.free_register(&size_reg);
        }

        context.free_register(&right_reg);
        context.free_register(&left_reg);
        result_reg
    } else {
        return Err(CompileError::CodegenError(
            format!("Invalid pointer arithmetic: {} {} {}", left_type, op, right_type)
        ));
    };

    Ok(result_reg)
}

fn generate_unary_operation(op: &str, operand: &AstNode, context: &mut CodeGenContext) -> Result<String, CompileError> {
    match op{
        "+" => {
            generate_expression(operand, context)
        },

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

        "!" => {
            let operand_type = get_expression_type(operand, context)?;
            let operand_reg = generate_expression(operand, context)?;
            let result_reg = context.get_register();
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
                        type_str if type_str.ends_with('*') => {
                            let pointed_type = type_str.trim_end_matches('*');
                            let pointed_size = match pointed_type {
                                "char" => 1,
                                "double" => 8,
                                _ => 4,
                            };

                            let result_reg = context.get_register();
                            context.emit(&format!("    # Pre-increment pointer"));
                            context.emit(&format!("    lw {}, {}(s0)", result_reg, offset));
                            context.emit(&format!("    addi {}, {}, {}", result_reg, result_reg, pointed_size));
                            context.emit(&format!("    sw {}, {}(s0)", result_reg, offset));
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
                        type_str if type_str.ends_with('*') => {
                            let pointed_type = type_str.trim_end_matches('*');
                            let pointed_size = match pointed_type {
                                "char" => 1,
                                "double" => 8,
                                _ => 4,
                            };

                            let result_reg = context.get_register();
                            context.emit(&format!("    # Pre-decrement pointer"));
                            context.emit(&format!("    lw {}, {}(s0)", result_reg, offset));
                            context.emit(&format!("    addi {}, {}, -{}", result_reg, result_reg, pointed_size));
                            context.emit(&format!("    sw {}, {}(s0)", result_reg, offset));
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
                        type_str if type_str.ends_with('*') => {
                            let pointed_type = type_str.trim_end_matches('*');
                            let pointed_size = match pointed_type {
                                "char" => 1,
                                "double" => 8,
                                _ => 4,
                            };

                            let result_reg = context.get_register();
                            let temp_reg = context.get_register();
                            context.emit(&format!("    # Post-increment pointer"));
                            context.emit(&format!("    lw {}, {}(s0)", result_reg, offset));
                            context.emit(&format!("    mv {}, {}", temp_reg, result_reg));
                            context.emit(&format!("    addi {}, {}, {}", temp_reg, temp_reg, pointed_size));
                            context.emit(&format!("    sw {}, {}(s0)", temp_reg, offset));
                            context.free_register(&temp_reg);
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
                        type_str if type_str.ends_with('*') => {
                            let pointed_type = type_str.trim_end_matches('*');
                            let pointed_size = match pointed_type {
                                "char" => 1,
                                "double" => 8,
                                _ => 4,
                            };

                            let result_reg = context.get_register();
                            let temp_reg = context.get_register();
                            context.emit(&format!("    # Post-decrement pointer"));
                            context.emit(&format!("    lw {}, {}(s0)", result_reg, offset));
                            context.emit(&format!("    mv {}, {}", temp_reg, result_reg));
                            context.emit(&format!("    addi {}, {}, -{}", temp_reg, temp_reg, pointed_size));
                            context.emit(&format!("    sw {}, {}(s0)", temp_reg, offset));
                            context.free_register(&temp_reg);
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

        "*" => {
            let addr_reg = generate_expression(operand, context)?;
            let operand_type = get_expression_type(operand, context)?;

            if operand_type == "char*" {
                let result_reg = context.get_register();
                context.emit(&format!("    lb {}, 0({})", result_reg, addr_reg));
                context.emit(&format!("    # Sign-extend char to word"));
                context.emit(&format!("    slli {0}, {0}, 24", result_reg));
                context.emit(&format!("    srai {0}, {0}, 24", result_reg));

                if addr_reg != result_reg {
                    context.free_register(&addr_reg);
                }
                Ok(result_reg)
            } else if operand_type == "float*" {
                let result_reg = context.get_fp_register();
                context.emit(&format!("    flw {}, 0({})", result_reg, addr_reg));
                context.free_register(&addr_reg);
                Ok(result_reg)
            } else if operand_type == "double*" {
                let result_reg = context.get_fp_register();
                context.emit(&format!("    fld {}, 0({})", result_reg, addr_reg));
                context.free_register(&addr_reg);
                Ok(result_reg)
            } else{
                let result_reg = context.get_register();
                context.emit(&format!("    lw {}, 0({})", result_reg, addr_reg));
                if addr_reg != result_reg {
                    context.free_register(&addr_reg);
                }
                Ok(result_reg)
            }
        },

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

fn generate_logical_operation(op: &str, left: &AstNode, right: &AstNode, context: &mut CodeGenContext) -> Result<String, CompileError> {
    let result_reg = generate_expression(left, context)?;
    let short_circuit_label = context.generate_label("short_circuit");
    let end_label = context.generate_label("logical_end");

    context.emit(&format!("    snez {0}, {0}", result_reg));

    if op == "&&"{
        context.emit(&format!("    beqz {}, {}", result_reg, short_circuit_label));
        let right_reg = generate_expression(right, context)?;
        context.emit(&format!("    snez {0}, {0}", right_reg));
        if result_reg != right_reg {
            context.emit(&format!("    mv {}, {}", result_reg, right_reg));
            context.free_register(&right_reg);
        }

        context.emit(&format!("    j {}", end_label));
        context.emit(&format!("{}:", short_circuit_label));

    } else if op == "||" {
        context.emit(&format!("    bnez {}, {}", result_reg, short_circuit_label));
        let right_reg = generate_expression(right, context)?;
        context.emit(&format!("    snez {0}, {0}", right_reg));

        if result_reg != right_reg{
            context.emit(&format!("    mv {}, {}", result_reg, right_reg));
            context.free_register(&right_reg);
        }
        context.emit(&format!("    j {}", end_label));
        context.emit(&format!("{}:", short_circuit_label));
    } else {
        return Err(CompileError::CodegenError(format!("Invalid logical operation: {}", op)));
    }

    context.emit(&format!("{}:", end_label));
    Ok(result_reg)
}

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

                    if !_type_info.starts_with("struct "){
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
                return Err(CompileError::CodegenError(format!("Complex struct member access not supported; failed at {:?}, {:?}", object, member)));
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

fn generate_function_call(function: &AstNode, args: &[Box<AstNode>], context: &mut CodeGenContext) -> Result<String, CompileError> {
    let func_name = match function {
        AstNode::Identifier(name) => name,
        _ => return Err(CompileError::CodegenError(format!("Expected function name, found {:?}", function))),
    };

    let int_registers = context.save_temp_registers();
    let fp_registers = context.save_fp_registers();

    for (i, arg) in args.iter().enumerate().take(8) {
        let arg_type = get_expression_type(arg, context)?;
        let is_float = arg_type == "float" || arg_type == "double";
        println!("Debug arg{}: arg_type {}, for function call {}", i, arg_type, func_name);
        if is_float {
            if let AstNode::FloatConstant(value) = &**arg {
                let const_label = context.generate_label("float_const");
                context.emit_data(&format!("{}:", const_label));
                context.emit_data(&format!("    .word 0x{:08x}  # float {}", f32::to_bits(*value as f32), value));

                let temp_reg = context.get_register();
                context.emit(&format!("    la {}, {}", temp_reg, const_label));
                context.emit(&format!("    flw fa{}, 0({})", i, temp_reg));
                context.free_register(&temp_reg);
            } else {
                let arg_reg = generate_expression(arg, context)?;

                if arg_reg.starts_with('f') {
                    if arg_reg != format!("fa{}", i) {
                        context.emit(&format!("    fmv.s fa{}, {}", i, arg_reg));
                        context.free_fp_register(&arg_reg);
                    }
                } else {
                    context.emit(&format!("    fcvt.s.w fa{}, {}", i, arg_reg));
                    context.free_register(&arg_reg);
                }
            }
        } else {
            match &**arg {
                AstNode::IntConstant(value) => {
                    context.emit(&format!("    li a{}, {}", i, value));
                },
                AstNode::StringLiteral(value) => {
                    let str_label = context.generate_label("str");
                    context.emit_data(&format!("{}:", str_label));
                    context.emit_data(&format!("    .string \"{}\"", value));
                    context.emit(&format!("    la a{}, {}", i, str_label));
                },
                _ => {
                    let arg_reg = generate_expression(arg, context)?;
                    if arg_reg.starts_with('f') {
                        context.emit(&format!("    fcvt.w.s a{}, {}", i, arg_reg));
                        context.free_fp_register(&arg_reg);
                    } else if arg_reg != format!("a{}", i) {
                        context.emit(&format!("    mv a{}, {}", i, arg_reg));
                        context.free_register(&arg_reg);
                    }
                },
            }
        }
    }
    context.emit(&format!("    call {}", func_name));

    let return_type = context.get_function_return_type(func_name).unwrap_or_else(|| {
        context.emit(&format!("    # Warning: Unknown return type for function {}, assuming int", func_name));
        "int".to_string()
    });

    let result_reg = if return_type == "float" || return_type == "double" {
        let fp_reg = context.get_fp_register();

        if fp_reg != "fa0" {
            context.emit(&format!("    fmv.s {}, fa0", fp_reg));
        }
        fp_reg
    } else {
        let reg = context.get_register();

        if reg != "a0" {
            context.emit(&format!("    mv {}, a0", reg));
        }
        reg
    };

    context.restore_fp_registers(fp_registers);
    context.restore_temp_registers(int_registers);
    Ok(result_reg)
}

fn calculate_array_element_address(array_name: &str, indices: &[AstNode], context: &mut CodeGenContext,) -> Result<(String, usize), CompileError> {
    let (base_offset, dimensions, is_global, global_label, is_pointer, type_info) = {
        if let Some(symbol) = context.lookup_symbol(array_name) {
            match &symbol.location {
                StorageLocation::Stack(offset) => {
                    (*offset, symbol.dimensions.clone(), false, String::new(),
                     symbol.is_pointer, symbol.type_info.clone())
                },
                StorageLocation::Global(label) => {
                    (0, symbol.dimensions.clone(), true, label.clone(),
                     symbol.is_pointer, symbol.type_info.clone())
                },
                _ => {
                    return Err(CompileError::CodegenError("Array has invalid storage type".to_string()));
                }
            }
        } else {
            return Err(CompileError::CodegenError(format!("Array '{}' not found", array_name)));
        }
    };

    let element_size = if type_info.contains("char") {
        1
    } else if type_info.contains("double") {
        8
    } else {
        4
    };

    println!("Array/pointer access: name={}, is_pointer={}, type={}",array_name, is_pointer, type_info);

    if is_pointer {
        if indices.len() != 1 {
            return Err(CompileError::CodegenError(format!(
                "Expected 1 index for pointer '{}', got {}", array_name, indices.len()
            )));
        }

        let index_reg = generate_expression(&indices[0], context)?;
        let base_reg = context.get_register();
        context.emit(&format!("    lw {}, {}(s0)", base_reg, base_offset));
        let addr_reg = context.get_register();

        if element_size == 1 {
            context.emit(&format!("    add {0}, {1}, {2}", addr_reg, base_reg, index_reg));
        } else {
            context.emit(&format!("    slli {0}, {1}, {2}", addr_reg, index_reg, if element_size == 8 { 3 } else { 2 })); // *4 or *8
            context.emit(&format!("    add {0}, {0}, {1}", addr_reg, base_reg));
        }

        context.free_register(&index_reg);
        context.free_register(&base_reg);
        return Ok((addr_reg, element_size));
    }

    if indices.len() != dimensions.len() {
        return Err(CompileError::CodegenError(format!(
            "Number of indices ({}) does not match array dimensions ({})", // SHAME ON YOU PROGRAMMER!
            indices.len(),
            dimensions.len()
        )));
    }

    let mut index_regs = Vec::new();
    for index in indices {
        let reg = generate_expression(index, context)?;
        index_regs.push(reg);
    }

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
    let addr_reg = context.get_register();
    context.emit(&format!("    slli {0}, {1}, 2", addr_reg, offset_reg));

    if is_global {
        let temp_reg = context.get_register();
        context.emit(&format!("    la {}, {}", temp_reg, global_label));
        context.emit(&format!("    add {}, {}, {}", addr_reg, addr_reg, temp_reg));
        context.free_register(&temp_reg);
    } else {
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

    for reg in index_regs {
        context.free_register(&reg);
    }
    context.free_register(&offset_reg);

    Ok((addr_reg, element_size))
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
                indices.reverse();
                return Ok((name.clone(), indices));
            },
            _ => return Err(CompileError::CodegenError("Invalid array access".to_string())),
        }
    }
}

fn generate_float_constant(value: f64, context: &mut CodeGenContext) -> Result<String, CompileError> {
    let label = context.generate_label("float_const");
    let reg = context.get_fp_register();
    context.emit_data(&format!("{}:", label));
    context.emit_data(&format!("    .word 0x{:08x}  # float {}", f32::to_bits(value as f32), value));

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

fn generate_fp_binary_operation(op: &str, left: &AstNode, right: &AstNode, context: &mut CodeGenContext) -> Result<String, CompileError> {
    let left_reg = generate_expression(left, context)?;
    let right_reg = generate_expression(right, context)?;

    let left_type = get_expression_type(left, context)?;
    let right_type = get_expression_type(right, context)?;

    let left_fp_reg = if !left_reg.starts_with('f'){
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
        "!=" => {
            let int_reg = context.get_register();
            context.emit(&format!("    feq.s {}, {}, {}", int_reg, left_fp_reg, right_fp_reg));
            context.emit(&format!("    xori {}, {}, 1", int_reg, int_reg));
            context.free_fp_register(&left_fp_reg);
            context.free_fp_register(&right_fp_reg);
            int_reg
        },
        "==" => {
            let int_reg = context.get_register();
            context.emit(&format!("    feq.s {}, {}, {}", int_reg, left_fp_reg, right_fp_reg));
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

    if right_fp_reg != result_reg {
        if right_fp_reg.starts_with('f') {
            context.free_fp_register(&right_fp_reg);
        } else {
            context.free_register(&right_fp_reg);
        }
    }
    Ok(result_reg)
}

fn generate_double_binary_operation(op: &str, left: &AstNode, right: &AstNode, context: &mut CodeGenContext) -> Result<String, CompileError> {
    let left_reg = generate_expression(left, context)?;
    let left_type = get_expression_type(left, context)?;
    let right_reg = generate_expression(right, context)?;
    let right_type = get_expression_type(right, context)?;

    let left_fp_reg = if !left_reg.starts_with('f') && left_type == "double" {
        let fp_reg = context.get_fp_register();
        context.emit(&format!("    fcvt.d.w {}, {}", fp_reg, left_reg));
        context.free_register(&left_reg);
        fp_reg
    } else if !left_reg.starts_with('f') && left_type == "float" {
        let fp_reg = context.get_fp_register();
        context.emit(&format!("    fcvt.d.s {}, {}", fp_reg, left_reg));
        context.free_register(&left_reg);
        fp_reg
    } else {
        left_reg
    };

    let right_fp_reg = if !right_reg.starts_with('f') && right_type == "double" {
        let fp_reg = context.get_fp_register();
        context.emit(&format!("    fcvt.d.w {}, {}", fp_reg, right_reg));
        context.free_register(&right_reg);
        fp_reg
    } else if !right_reg.starts_with('f') && right_type == "float" {
        let fp_reg = context.get_fp_register();
        context.emit(&format!("    fcvt.d.s {}, {}", fp_reg, right_reg));
        context.free_register(&right_reg);
        fp_reg
    } else {
        right_reg
    };

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
        "!=" => {
            let int_reg = context.get_register();
            context.emit(&format!("    feq.d {}, {}, {}", int_reg, left_fp_reg, right_fp_reg));
            context.emit(&format!("    xori {}, {}, 1", int_reg, int_reg));
            context.free_fp_register(&left_fp_reg);
            context.free_fp_register(&right_fp_reg);
            int_reg
        },
        "==" => {
            let int_reg = context.get_register();
            context.emit(&format!("    feq.d {}, {}, {}", int_reg, left_fp_reg, right_fp_reg));
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

    if right_fp_reg != result_reg {
        if right_fp_reg.starts_with('f') {
            context.free_fp_register(&right_fp_reg);
        } else {
            context.free_register(&right_fp_reg);
        }
    }
    Ok(result_reg)
}

fn generate_ternary_operation(condition: &AstNode, true_expr: &AstNode, false_expr: &AstNode, context: &mut CodeGenContext) -> Result<String, CompileError> {
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
    if true_reg != false_reg{
        context.emit(&format!("    mv {}, {}", result_reg, true_reg));
        context.free_register(&true_reg);
        context.free_register(&false_reg);
    } else {
        result_reg = true_reg;
    }

    Ok(result_reg)
}

pub fn get_binary_operation_type(op: &str, left: &AstNode, right: &AstNode, context: &mut CodeGenContext) -> Result<String, CompileError> {
    let left_type = get_expression_type(left, context)?;
    let right_type = get_expression_type(right, context)?;

    if (op == "+" || op == "-") && (left_type.ends_with('*') || right_type.ends_with('*')) {
        if left_type.ends_with('*') && (op == "+" || op == "-") && !right_type.ends_with('*') {
            return Ok(left_type);
        } else if right_type.ends_with('*') && op == "+" && !left_type.ends_with('*') {
            return Ok(right_type);
        } else if left_type.ends_with('*') && right_type.ends_with('*') && op == "-" {
            return Ok("int".to_string());
        }
    }

    match op {
        _ => { // Basically indirect type checking for binary ops but very basic
            if left_type == right_type {
                Ok(left_type)
            } else {
                Err(CompileError::TypeError(
                    format!("Type mismatch in binary operation: {} {} {}", left_type, op, right_type)
                ))
            }
        }
    }
}

pub fn get_expression_type(node: &AstNode, context: &mut CodeGenContext) -> Result<String, CompileError> {
    println!("Debug node: {:?}", node);

    let result = match node {
        AstNode::IntConstant(_) => Ok("int".to_string()),
        AstNode::FloatConstant(_) => Ok("float".to_string()),

        AstNode::StringLiteral(_) => Ok("char*".to_string()),
        AstNode::Identifier(name) => {
            if let Some(symbol) = context.lookup_symbol(name) {
                Ok(symbol.type_info.clone())
            } else {
                if let Some(return_type) = context.get_function_return_type(name) {
                    Ok(return_type)
                } else {
                    Err(CompileError::CodegenError(format!("Unknown identifier: {}", name)))
                }
            }
        },
        AstNode::MemberAccess { object, member } => {
            // Recursively get the type of the object
            let object_type = get_expression_type(object, context)?;
            println!("Object type for {:?}: {:?}", object, object_type);
            // Check if the object type is a struct or a pointer to a struct
            let struct_name = if object_type.starts_with("struct ") {
                object_type["struct ".len()..].to_string()
            } else if object_type.ends_with('*') && object_type.starts_with("struct ") {
                // Handle pointer to struct
                object_type["struct ".len()..object_type.len() - 1].to_string()
            } else {
                return Err(CompileError::CodegenError(format!(
                    "Member access on non-struct type at {:?}: {}", object, object_type
                )));
            };

            // Look up the struct definition and its field
            if let Some(struct_def) = context.struct_definitions.get(&struct_name) {
                if let Some(field_info) = struct_def.fields.get(member) {
                    Ok(field_info.type_info.clone())
                } else {
                    Err(CompileError::CodegenError(format!(
                        "Struct {} has no member named {}", struct_name, member
                    )))
                }
            } else {
                Err(CompileError::CodegenError(format!(
                    "Unknown struct type: {}", struct_name
                )))
            }
        },
        AstNode::PointerMemberAccess { pointer, member } => {
            // Recursively get the type of the pointer
            let pointer_type = get_expression_type(pointer, context)?;
            println!("Pointer type for {:?}: {:?}", pointer, pointer_type);
            // Ensure it's a pointer type
            if !pointer_type.ends_with('*') {
                return Err(CompileError::CodegenError(format!(
                    "Pointer member access (->) on non-pointer type: {}", pointer_type
                )));
            }

            // Extract the base struct type (e.g., "struct Date" from "struct Date*")
            let base_type = pointer_type.trim_end_matches('*');
            if !base_type.starts_with("struct ") {
                return Err(CompileError::CodegenError(format!(
                    "Pointer member access on non-struct pointer type: {}", base_type
                )));
            }
            let struct_name = base_type["struct ".len()..].to_string();
            // Look up the struct definition and its field
            if let Some(struct_def) = context.struct_definitions.get(&struct_name) {
                if let Some(field_info) = struct_def.fields.get(member) {
                    println!("Field info for {:?}: {:?}", member, field_info.type_info);
                    Ok(field_info.type_info.clone())
                } else {
                    Err(CompileError::CodegenError(format!(
                        "Struct {} has no member named {}", struct_name, member
                    )))
                }
            } else {
                Err(CompileError::CodegenError(format!(
                    "Unknown struct type: {}", struct_name
                )))
            }
        },
        AstNode::FunctionCall { function, .. } => {
            if let AstNode::Identifier(func_name) = &**function {
                if let Some(return_type) = context.get_function_return_type(func_name) {
                    Ok(return_type)
                } else {
                    Ok("int".to_string())
                }
            } else {
                Ok("int".to_string())
            }
        },
        AstNode::BinaryOperation { op, left, right, .. } => {
            get_binary_operation_type(op, left, right, context)
        },
        AstNode::UnaryOperation { op, operand} => {
            if op == "&" {
                let mut operand_type = get_expression_type(operand, context)?;
                operand_type = {operand_type} + "*";
                return Ok(operand_type.to_string());
            }
            let operand_type = get_expression_type(operand, context)?;
            if operand_type == "float" {
                Ok("float".to_string())
            } else if operand_type == "double" {
                Ok("double".to_string())
            } else {
                Ok("int".to_string())
            }
        },
        AstNode::ArraySubscript { array, index } => {
            let array_type = get_expression_type(array, context)?;
            println!("Array Name: {:?}, ArraySubscript base type: {}", array, array_type);

            if let AstNode::Identifier(name) = &**array {
                if let Some(symbol) = context.lookup_symbol(name) {
                    println!("Symbol info for {}: is_pointer={}, type_info={}", name, symbol.is_pointer, symbol.type_info);

                    if !symbol.dimensions.is_empty() && !symbol.is_pointer {
                        let base_type = symbol.type_info.split('[').next().unwrap_or(&symbol.type_info).to_string();
                        return Ok(base_type);
                    }

                    if symbol.type_info.ends_with('*') {
                        return Ok(symbol.type_info.trim_end_matches('*').to_string());
                    }

                    Ok(symbol.type_info.clone())
                } else {
                    Ok("int".to_string())
                }
            } else {
                if array_type.ends_with('*') {
                    return Ok(array_type.trim_end_matches('*').to_string());
                }

                Ok("int".to_string())
            }
        },
        AstNode::SizeofType { .. } | AstNode::SizeofExpr { .. } => {
            Ok("int".to_string())
        },
        _ => {
            println!("Unhandled node type for type checking: {:?}", node);
            Ok("int".to_string())
        }
    };

    if let Ok(ref type_str) = result {
        println!("Expression type result: {}", type_str);
    } else if let Err(ref error) = result {
        println!("Expression type error: {:?}", error);
    }
    result
}

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

fn get_type_size(type_spec: &TypeSpecifier, pointer_level: usize, context: &mut CodeGenContext) -> i32 {
    if pointer_level > 0 {
        return 4;
    }

    match type_spec {
        TypeSpecifier::Char => 1,
        TypeSpecifier::Int => 4,
        TypeSpecifier::Unsigned => 4,
        TypeSpecifier::Float => 4,
        TypeSpecifier::Double => 8,
        TypeSpecifier::Void => 1,
        TypeSpecifier::Struct(name) => {
            if let Some(struct_def) = context.struct_definitions.get(name) {
                struct_def.total_size as i32
            } else {
                4
            }
        },
        TypeSpecifier::Enum(name) => 4,
        _ => 4,
    }
}

fn get_size_from_type_string(type_str: &str, context: &mut CodeGenContext) -> i32 {
    if type_str.ends_with('*') {
        return 4;
    }

    match type_str {
        "char" => 1,
        "int" => 4,
        "unsigned" => 4,
        "float" => 4,
        "double" => 8,
        "void" => 1,
        _ if type_str.starts_with("struct ") => {
            let struct_name = type_str["struct ".len()..].to_string();
            if let Some(struct_def) = context.struct_definitions.get(&struct_name) {
                struct_def.total_size as i32
            } else {
                4
            }
        },
        _ => 4,
    }
}