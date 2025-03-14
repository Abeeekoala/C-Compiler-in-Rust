use crate::ast::AstNode;
use crate::codegen::context::CodeGenContext;
use crate::codegen::expression::generate_expression;
use crate::error::CompileError;

/// Generate code for a statement
pub fn generate_statement(node: &AstNode, context: &mut CodeGenContext) -> Result<(), CompileError> {
    match node {
        AstNode::ReturnStatement(_) => generate_return_statement(node, context),
        AstNode::WhileStatement { .. } => generate_while_statement(node, context),
        AstNode::IfStatement { .. } => generate_if_statement(node, context),
        AstNode::BlockStatement(stmts) => {
            for stmt in stmts {
                generate_statement(stmt, context)?;
            }
            Ok(())
        },
        AstNode::ExpressionStatement(_) => generate_expression_statement(node, context),
        AstNode::Declaration { type_spec, declarator, initializer } => {
            // Generate code for a single declaration
            if let AstNode::Identifier(name) = &**declarator {
                // Allocate space for the variable
                let offset = context.add_variable(name.to_string(), "int".to_string());

                // Initialize if an initializer is present
                if let Some(init_expr) = initializer {
                    let reg = generate_expression(init_expr, context)?;
                    context.emit(&format!("    sw {}, {}(s0)", reg, offset));
                    context.free_register(&reg);
                }
            }
            Ok(())
        },
        _ => Err(CompileError::CodegenError(format!("Unsupported statement type: {:?}", node))),
    }
}

/// Generate code for a while loop
fn generate_while_statement(node: &AstNode, context: &mut CodeGenContext) -> Result<(), CompileError> {
    if let AstNode::WhileStatement { condition, body } = node {
        let start_label = context.generate_label("while_start");
        let end_label = context.generate_label("while_end");

        // Emit start label
        context.emit(&format!("{}:", start_label));

        // Generate condition expression
        let cond_reg = generate_expression(condition, context)?;

        // If condition is false, exit loop
        context.emit(&format!("    beqz {}, {}", cond_reg, end_label));
        context.free_register(&cond_reg);

        // Generate loop body
        generate_statement(body, context)?;

        // Jump back to start
        context.emit(&format!("    j {}", start_label));

        // Emit end label
        context.emit(&format!("{}:", end_label));

        Ok(())
    } else {
        Err(CompileError::CodegenError("Expected while statement".to_string()))
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

fn generate_declaration_item(node: &AstNode, context: &mut CodeGenContext) -> Result<(), CompileError> {
    if let AstNode::VariableDeclaration { declarator, initializer, .. } = node {
        if let AstNode::Identifier(name) = &**declarator {
            // Allocate space for the variable
            // Pass a String for the name and "int" as the type name
            let offset = context.add_variable(name.to_string(), "int".to_string());

            // Initialize if an initializer is present
            if let Some(init_expr) = initializer {
                let reg = generate_expression(init_expr, context)?;
                context.emit(&format!("    sw {}, {}(s0)", reg, offset));
                context.free_register(&reg);
            }

            Ok(())
        } else {
            Err(CompileError::CodegenError("Expected identifier in variable declaration".to_string()))
        }
    } else {
        Err(CompileError::CodegenError("Expected variable declaration".to_string()))
    }
}
