use crate::ast::AstNode;
use crate::codegen::context::CodeGenContext;
use crate::codegen::statement::generate_statement;

/// Generate code for a function definition
pub fn generate_function(node: &AstNode, context: &mut CodeGenContext) {
    if let AstNode::FunctionDefinition { declarator, compound_statement, .. } = node {
        // Get function name
        let func_name = if let AstNode::Identifier(name) = &**declarator {
            name
        } else {
            return; // Not a valid function
        };

        // Set current function
        context.current_function = Some(func_name.clone());

        // Emit function label
        context.emit(&format!(".globl {}", func_name));
        context.emit(&format!("{}:", func_name));

        // Prologue: save frame pointer and return address
        context.stack_offset = 16; // Space for saved ra and s0
        context.emit(&format!("    addi sp, sp, -{}", context.stack_offset));
        context.emit("    sw ra, 12(sp)");
        context.emit("    sw s0, 0(sp)");
        context.emit("    addi s0, sp, 0");  // Set frame pointer

        // Generate code for function body
        generate_statement(compound_statement, context);

        // If function doesn't end with return, add one
        if let AstNode::NodeList(statements) = &**compound_statement {
            if statements.is_empty() || !matches!(statements.last().unwrap(), AstNode::ReturnStatement(_)) {
                // Add implicit return
                context.emit("    li a0, 0");  // Return 0 by default
                context.emit("    lw ra, 12(sp)");
                context.emit("    lw s0, 0(sp)");
                context.emit(&format!("    addi sp, sp, {}", context.stack_offset));
                context.emit("    ret");
            }
        }

        // Clear current function
        context.current_function = None;
    }
}
