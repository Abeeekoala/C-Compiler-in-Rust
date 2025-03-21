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
            let struct_fields: Vec<(String, String, usize)> = fields.iter()
                .filter_map(|field| {
                    if let AstNode::StructField { type_spec, name } = &**field {
                        let field_type = match type_spec {
                            TypeSpecifier::Int => "int",
                            TypeSpecifier::Float => "float",
                            TypeSpecifier::Double => "double",
                            TypeSpecifier::Char => "char",
                            _ => "unknown",
                        };
                        let field_size = match type_spec {
                            TypeSpecifier::Double => 8,
                            _ => 4,
                        };
                        Some((name.clone(), field_type.to_string(), field_size))
                    } else {
                        None
                    }
                }).collect();
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
                        let struct_fields: Vec<(String, String, usize)> = fields.iter()
                            .filter_map(|field| {
                                if let AstNode::StructField { type_spec, name } = &**field {
                                    let field_type = match type_spec {
                                        TypeSpecifier::Int => "int",
                                        TypeSpecifier::Float => "float",
                                        TypeSpecifier::Double => "double",
                                        TypeSpecifier::Char => "char",
                                        _ => "unknown",
                                    };
                                    let field_size = match type_spec {
                                        TypeSpecifier::Double => 8,
                                        _ => 4,
                                    };
                                    Some((name.clone(), field_type.to_string(), field_size))
                                } else {
                                    None
                                }
                            }).collect();
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
