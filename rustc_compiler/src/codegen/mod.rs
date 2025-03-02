pub mod context;
pub mod expression;
pub mod function;
pub mod statement;

use crate::ast::AstNode;
use crate::error::CompileError;

/// Generate RISC-V assembly code from an AST
pub fn generate_code(ast: &AstNode) -> Result<String, CompileError> {
    let mut context = context::CodeGenContext::new();

    // Data section
    context.emit(".data");

    // Text section
    context.emit(".text");

    // Generate code for each top-level declaration
    match ast {
        AstNode::NodeList(nodes) => {
            for node in nodes {
                match &**node {
                    AstNode::FunctionDefinition { .. } => {
                        function::generate_function(node, &mut context)?;
                    },
                    // Handle other top-level declarations
                    _ => return Err(CompileError::CodegenError("Unsupported top-level declaration".to_string())),
                }
            }
        },
        // Handle single function case
        AstNode::FunctionDefinition { .. } => {
            function::generate_function(ast, &mut context)?;
        },
        _ => return Err(CompileError::CodegenError("Expected translation unit".to_string())),
    }

    Ok(context.get_assembly())
}
