use crate::ast::AstNode;
use crate::codegen::context::CodeGenContext;
use crate::codegen::expression::generate_expression;
use crate::error::CompileError;

/// Generate code for a statement
pub fn generate_statement(node: &AstNode, context: &mut CodeGenContext) -> Result<(), CompileError> {
    match node {
        AstNode::ReturnStatement(_) => generate_return_statement(node, context),
        AstNode::IfStatement { .. } => generate_if_statement(node, context),
        AstNode::BlockStatement(stmts) => {
            for stmt in stmts {
                generate_statement(stmt, context)?;
            }
            Ok(())
        },
        AstNode::ExpressionStatement(_) => generate_expression_statement(node, context),
        _ => Err(CompileError::CodegenError(format!("Unsupported statement type: {:?}", node))),
    }
}

/// Generate code for an if statement
fn generate_if_statement(node: &AstNode, context: &mut CodeGenContext) -> Result<(), CompileError> {
    if let AstNode::IfStatement { condition, then_stmt, else_stmt } = node {
        let end_label = context.generate_label("if_end");
        let else_label = context.generate_label("if_else");

        // Generate code for condition
        let cond_reg = generate_expression(condition, context)?;

        // Generate branch
        if else_stmt.is_some() {
            context.emit(&format!("    beqz {}, {}", cond_reg, else_label));
        } else {
            context.emit(&format!("    beqz {}, {}", cond_reg, end_label));
        }

        // Free the condition register
        context.free_register(&cond_reg);

        // Generate then statement
        generate_statement(then_stmt, context)?;

        // Handle else branch if it exists
        if let Some(else_branch) = else_stmt {
            context.emit(&format!("    j {}", end_label));
            context.emit(&format!("{}:", else_label));
            generate_statement(else_branch, context)?;
        }

        context.emit(&format!("{}:", end_label));
        Ok(())
    } else {
        Err(CompileError::CodegenError("Expected if statement".to_string()))
    }
}

/// Generate code for a return statement
fn generate_return_statement(node: &AstNode, context: &mut CodeGenContext) -> Result<(), CompileError> {
    if let AstNode::ReturnStatement(expr) = node {
        if let Some(expr) = expr {
            // Generate the expression and move result to a0
            let reg = generate_expression(expr, context)?;
            context.emit(&format!("    mv a0, {}", reg));
            context.free_register(&reg);
        }

        // Generate epilogue
        context.generate_function_epilogue();

        Ok(())
    } else {
        Err(CompileError::CodegenError("Expected return statement".to_string()))
    }
}

/// Generate code for an expression statement
fn generate_expression_statement(node: &AstNode, context: &mut CodeGenContext) -> Result<(), CompileError> {
    if let AstNode::ExpressionStatement(expr) = node {
        let reg = generate_expression(expr, context)?;
        // Free the register used by the expression
        context.free_register(&reg);
        Ok(())
    } else {
        Err(CompileError::CodegenError("Expected expression statement".to_string()))
    }
}
