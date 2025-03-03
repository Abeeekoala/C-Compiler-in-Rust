use crate::ast::AstNode;
use crate::codegen::context::CodeGenContext;
use crate::codegen::statement::generate_statement;
use crate::error::CompileError;

/// Generate code for a function definition
pub fn generate_function(node: &AstNode, context: &mut CodeGenContext) -> Result<(), CompileError> {
    if let AstNode::FunctionDefinition { declarator, compound_statement, .. } = node {
        // Get the function name
        if let AstNode::Identifier(name) = &**declarator {
            // Set current function name
            context.current_function = Some(name.clone());

            // Generate function label
            context.emit(&format!(".globl {}", name));
            context.emit(&format!("{}:", name));

            // Generate function prologue
            context.generate_function_prologue();

            // Generate code for function body
            generate_statement(compound_statement, context)?;

            // If there's no explicit return at the end, add one
            if !context.output.trim().ends_with("ret") {
                context.generate_function_epilogue();
            }

            Ok(())
        } else {
            Err(CompileError::CodegenError("Expected function name".to_string()))
        }
    } else {
        Err(CompileError::CodegenError("Expected function definition".to_string()))
    }
}
