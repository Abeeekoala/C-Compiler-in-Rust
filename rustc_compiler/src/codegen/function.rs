use crate::ast::AstNode;
use crate::codegen::context::FullType;
use crate::ast::TypeSpecifier;
use crate::codegen::context::{CodeGenContext, StorageLocation, Symbol};
use crate::codegen::statement::generate_statement;
use crate::codegen::expression::generate_expression;
use crate::error::CompileError;


pub fn generate_function(node: &AstNode, context: &mut CodeGenContext) -> Result<(), CompileError> {
    match node {
        AstNode::FunctionDefinition { decl_specifiers, declarator, parameters, compound_statement } => {
            if let AstNode::Identifier(name) = &**declarator {
                let return_type_str = extract_return_type_from_decl_specifiers(decl_specifiers)?;

                let param_types = parameters.iter().map(|p| match &**p {
                    AstNode::Declaration { type_spec, declarator, .. } => {
                        println!("type_spec: {:?}", type_spec);
                        get_full_type(context, type_spec, declarator).to_string()
                    }
                    _ => "unknown".to_string(),
                }).collect();
                context.declare_function(name, return_type_str, param_types);

                context.emit(&format!(".globl {}", name));
                context.emit(&format!("{}:", name));
                context.generate_function_prologue();
                context.enter_scope();

                let mut float_param_count = 0;
                let mut int_param_count = 0;

                for param in parameters.iter().take(16) {
                    if let AstNode::Declaration { type_spec, declarator, .. } = &**param {
                        let (param_name, is_pointer) = match &**declarator {
                            AstNode::Identifier(name) => (name.clone(), false),
                            AstNode::PointerDeclarator { pointee } => {
                                if let AstNode::Identifier(name) = &**pointee {
                                    (name.clone(), true)
                                } else {
                                    return Err(CompileError::CodegenError("Expected identifier in pointer declarator".to_string()));
                                }
                            },
                            _ => return Err(CompileError::CodegenError("Expected identifier or pointer declarator".to_string())),
                        };

                        let type_str = get_full_type(context, type_spec, declarator).to_string();
                        let is_float = type_str == "float" || type_str == "double";

                        let reg_name = if is_float {
                            if float_param_count >= 8 {
                                panic!("Too many float parameters");
                            } else {
                                format!("fa{}", float_param_count)
                            }
                        } else {
                            if int_param_count >= 8 {
                                panic!("Too many integer parameters");
                            } else {
                                format!("a{}", int_param_count)
                            }
                        };
                        if is_float {
                            float_param_count += 1;
                        } else {
                            int_param_count += 1;
                        }

                        let size = if type_str == "double" { 8 } else { 4 };
                        let stack_offset = context.allocate_stack_space(size);

                        if is_float {
                            if type_str == "double" {
                                context.emit(&format!("    fsd {}, {}(s0)", reg_name, stack_offset));
                            } else {
                                context.emit(&format!("    fsw {}, {}(s0)", reg_name, stack_offset));
                            }
                        } else {
                            context.emit(&format!("    sw {}, {}(s0)", reg_name, stack_offset));
                        }

                        let mut symbol = Symbol {
                            location: StorageLocation::Stack(stack_offset),
                            size,
                            type_info: type_str.clone(),
                            dimensions: Vec::new(),
                            is_pointer,
                        };
                        context.add_symbol(&param_name, symbol);
                    }
                }

                generate_statement(compound_statement, context)?;
                context.exit_scope();

                if !context.output.trim().ends_with("ret") {
                    context.generate_function_epilogue();
                }
                Ok(())
            } else {
                Err(CompileError::CodegenError("Expected function name".to_string()))
            }
        },
        AstNode::FunctionDeclaration { decl_specifiers, declarator, parameters } => {
            if let AstNode::Identifier(name) = &**declarator {
                let return_type_str = extract_return_type_from_decl_specifiers(decl_specifiers)?;
                let param_types = parameters.iter().map(|p| match &**p {
                    AstNode::Declaration { type_spec, .. } => type_spec_to_string(type_spec),
                    _ => "unknown".to_string(),
                }).collect();
                context.declare_function(name, return_type_str, param_types);
                Ok(())
            } else {
                Err(CompileError::CodegenError("Expected function name".to_string()))
            }
        },
        _ => Err(CompileError::CodegenError("Expected function definition or declaration".to_string())),
    }
}

fn extract_return_type_from_decl_specifiers(decl_specifiers: &[Box<AstNode>]) -> Result<String, CompileError> {
    for spec in decl_specifiers {
        if let AstNode::TypeSpecifier(type_spec) = &**spec {
            return Ok(type_spec_to_string(type_spec));
        }
    }
    Ok("int".to_string())
}

fn get_full_type(context: &CodeGenContext, type_spec: &TypeSpecifier, declarator: &AstNode) -> FullType {
    let base_type = match type_spec {
        TypeSpecifier::TypedefName(name) => {
            if let Some(actual_type) = context.typedef_map.get(name) {
                actual_type.clone()
            }
            else {
                FullType::Base(type_spec.clone())
            }
        },
        _ => FullType::Base(type_spec.clone()),
    };

    match declarator {
        AstNode::PointerDeclarator { pointee } => {
            FullType::Pointer(Box::new(get_full_type(context, type_spec, pointee)))
        }
        _ => base_type,
    }
}

fn type_spec_to_string(type_spec: &crate::ast::TypeSpecifier) -> String {
    match type_spec {
        crate::ast::TypeSpecifier::Int => "int".to_string(),
        crate::ast::TypeSpecifier::Char => "char".to_string(),
        crate::ast::TypeSpecifier::Float => "float".to_string(),
        crate::ast::TypeSpecifier::Double => "double".to_string(),
        crate::ast::TypeSpecifier::Void => "void".to_string(),
        crate::ast::TypeSpecifier::Struct(name) => format!("struct {}", name),
        _ => "unknown".to_string(),
    }
}

pub fn get_function_params(declarator: &AstNode) -> Option<Vec<(String, String)>> {
    match declarator {
        AstNode::Identifier(_) => {
            Some(Vec::new())
        },
        AstNode::FunctionCall { function, args } => {
            if let AstNode::Identifier(_) = **function {
                let mut params = Vec::new();
                for arg in args {
                    if let AstNode::Declaration { type_spec, declarator, .. } = &**arg {
                        if let AstNode::Identifier(param_name) = &**declarator {
                            let type_str = type_spec_to_string(type_spec);
                            params.push((param_name.clone(), type_str));
                        }
                    }
                }
                Some(params)
            } else {
                None
            }
        },
        _ => None,
    }
}