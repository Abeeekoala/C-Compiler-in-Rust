use crate::ast::AstNode;
use crate::codegen::context::CodeGenContext;
use crate::codegen::statement::generate_statement;
use crate::error::CompileError;

/// Generate code for a function definition
pub fn generate_function(node: &AstNode, context: &mut CodeGenContext) -> Result<(), CompileError> {
    if let AstNode::FunctionDefinition { decl_specifiers: _, declarator, compound_statement } = node {
        // Get function name
        let func_name = match &**declarator {
            AstNode::Identifier(name) => name,
            _ => return Err(CompileError::CodegenError("Invalid function declarator".to_string())),
        };

        // Generate function prologue
        context.emit(&format!("    .text"));
        context.emit(&format!("    .globl {}", func_name));
        context.emit(&format!("{}:", func_name));

        // Generate function prologue
        context.generate_function_prologue();

        // Generate function body
        generate_statement(compound_statement, context)?;

        // If we reach here without a return, add a default return
        context.generate_function_epilogue();

        Ok(())
    } else {
        Err(CompileError::CodegenError("Expected function definition".to_string()))
    }
}
