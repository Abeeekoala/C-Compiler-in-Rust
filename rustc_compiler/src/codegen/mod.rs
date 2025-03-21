pub mod context;
pub mod expression;
pub mod function;
pub mod statement;

use crate::ast::AstNode;
use crate::ast::TypeSpecifier;
use crate::error::CompileError;
use crate::codegen::context::CodeGenContext;


pub fn generate_code(ast: &AstNode) -> Result<(String, CodeGenContext), CompileError> {
    let mut context = CodeGenContext::new();
    match ast {
        AstNode::FunctionDefinition { .. } => {
            function::generate_function(ast, &mut context)?;
        },
        AstNode::StructDefinition { name, fields } => {
            let mut struct_fields = Vec::new();
            for field in fields {
                if let AstNode::StructField { type_spec, name } = &**field {
                    let field_type = match type_spec {
                        TypeSpecifier::Int => "int".to_string(),
                        TypeSpecifier::Float => "float".to_string(),
                        TypeSpecifier::Double => "double".to_string(),
                        TypeSpecifier::Char => "char".to_string(),
                        TypeSpecifier::Struct(struct_name) => {
                            if context.struct_definitions.contains_key(struct_name) {
                                format!("struct {}", struct_name)
                            } else {
                                return Err(CompileError::CodegenError(format!(
                                    "Undefined struct: {}", struct_name
                                )));
                            }
                        },
                        TypeSpecifier::TypedefName(typedef_name) => {
                            if let Some(resolved_type) = context.typedef_map.get(typedef_name) {
                                resolved_type.clone().to_string()
                            } else {
                                return Err(CompileError::CodegenError(format!(
                                    "Undefined typedef: {}", typedef_name
                                )));
                            }
                        },
                        _ => {
                            return Err(CompileError::CodegenError(
                                "Unsupported type specifier".to_string()
                            ))
                        }
                    };
                    let field_size = context.get_type_size(&field_type)?;
                    struct_fields.push((name.clone(), field_type, field_size));
                }
            }
            context.register_struct(name.clone(), struct_fields);
        },
        AstNode::Declaration { .. } => {
            statement::generate_statement(ast, &mut context)?;
        },
        AstNode::NodeList(nodes) => {
            for node in nodes {
                match &**node {
                    AstNode::FunctionDefinition { .. } => {
                        function::generate_function(node, &mut context)?;
                    },
                    AstNode::StructDefinition { name, fields } => {
                        let mut struct_fields = Vec::new();
                        for field in fields {
                            if let AstNode::StructField { type_spec, name } = &**field {
                                let field_type = match type_spec {
                                    TypeSpecifier::Int => "int".to_string(),
                                    TypeSpecifier::Float => "float".to_string(),
                                    TypeSpecifier::Double => "double".to_string(),
                                    TypeSpecifier::Char => "char".to_string(),
                                    TypeSpecifier::Struct(struct_name) => {
                                        if context.struct_definitions.contains_key(struct_name) {
                                            format!("struct {}", struct_name)
                                        } else {
                                            return Err(CompileError::CodegenError(format!(
                                                "Undefined struct: {}", struct_name
                                            )));
                                        }
                                    },
                                    TypeSpecifier::TypedefName(typedef_name) => {
                                        if let Some(resolved_type) = context.typedef_map.get(typedef_name) {
                                            resolved_type.clone().to_string()
                                        } else {
                                            return Err(CompileError::CodegenError(format!(
                                                "Undefined typedef: {}", typedef_name
                                            )));
                                        }
                                    },
                                    _ => {
                                        return Err(CompileError::CodegenError(
                                            "Unsupported type specifier".to_string()
                                        ))
                                    }
                                };
                                let field_size = context.get_type_size(&field_type)?;
                                struct_fields.push((name.clone(), field_type, field_size));
                            }
                        }
                        context.register_struct(name.clone(), struct_fields);
                    },
                    _ => {
                        statement::generate_statement(node, &mut context)?;
                    }
                }
            }
        },
        _ => {
            statement::generate_statement(ast, &mut context)?;
        }
    }

    Ok((context.get_assembly(), context))
}
