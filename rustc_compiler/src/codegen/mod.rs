pub mod context;
pub mod expression;
pub mod function;
pub mod statement;

use crate::ast::AstNode;
use crate::error::CompileError;
use crate::codegen::context::CodeGenContext;

/// Generate RISC-V assembly code from an AST
pub fn generate_code(ast: &AstNode) -> Result<(String, CodeGenContext), CompileError> {
    let mut context = CodeGenContext::new();

    // Data section
    context.emit(".data");

    // Text section
    context.emit(".text");

    // Generate code for each top-level declaration
    match ast {
        AstNode::FunctionDefinition { .. } => {
            function::generate_function(ast, &mut context)?;
        },
        AstNode::NodeList(nodes) => {
            for node in nodes {
                match &**node {
                    AstNode::FunctionDefinition { .. } => {
                        function::generate_function(node, &mut context)?;
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
