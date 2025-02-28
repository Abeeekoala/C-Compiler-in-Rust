mod context;
mod expression;
mod statement;
mod function;

use crate::ast::AstNode;
use context::CodeGenContext;

/// Generate RISC-V assembly code from an AST
pub fn generate_code(ast: &AstNode) -> String {
    let mut context = CodeGenContext::new();

    // Data section
    context.emit(".data");

    // Text section
    context.emit(".text");

    // Generate code based on AST node type
    match ast {
        AstNode::NodeList(nodes) => {
            for node in nodes {
                match node {
                    AstNode::FunctionDefinition { .. } => {
                        function::generate_function(node, &mut context);
                    },
                    // Add cases for other top-level declarations
                    _ => {}
                }
            }
        },
        AstNode::FunctionDefinition { .. } => {
            function::generate_function(ast, &mut context);
        },
        // Handle single-node AST (usually for testing)
        _ => {}
    }

    context.output
}
