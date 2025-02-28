use crate::ast::AstNode;
use crate::codegen::context::CodeGenContext;
use crate::codegen::expression::generate_expression;

/// Generate code for a statement
pub fn generate_statement(node: &AstNode, context: &mut CodeGenContext) {
    match node {
        AstNode::ReturnStatement(expr) => generate_return_statement(expr, context),
        AstNode::NodeList(statements) => {
            for stmt in statements {
                generate_statement(stmt, context);
            }
        },
        AstNode::IfStatement { condition, then_stmt, else_stmt } => {
            generate_if_statement(condition, then_stmt, else_stmt, context);
        },
        _ => {
            // For expressions used as statements, evaluate but discard result
            if let Some(reg) = generate_expression(node, context) {
                context.free_register(&reg);
            }
        }
    }
}

/// Generate code for an if statement
fn generate_if_statement(
    condition: &AstNode,
    then_stmt: &AstNode,
    else_stmt: &Option<Box<AstNode>>,
    context: &mut CodeGenContext
) {
    // Generate unique labels for this if statement
    let else_label = context.generate_label("else");
    let end_if_label = context.generate_label("endif");

    // Generate code for the condition expression
    let cond_reg = match generate_expression(condition, context) {
        Some(reg) => reg,
        None => return, // If we can't generate the condition, just return
    };

    // Branch to else or end if condition is false
    if else_stmt.is_some() {
        context.emit(&format!("    beqz {}, {}", cond_reg, else_label));
    } else {
        context.emit(&format!("    beqz {}, {}", cond_reg, end_if_label));
    }

    // Free the condition register
    context.free_register(&cond_reg);

    // Generate code for the then clause
    generate_statement(then_stmt, context);

    // If there's an else clause, add a jump to skip it after executing the then clause
    if else_stmt.is_some() {
        context.emit(&format!("    j {}", end_if_label));
        context.emit(&format!("{}:", else_label));

        // Generate code for the else clause
        if let Some(else_statement) = else_stmt {
            generate_statement(else_statement, context);
        }
    }

    // End of if statement
    context.emit(&format!("{}:", end_if_label));
}

/// Generate code for a return statement
fn generate_return_statement(expr: &Option<Box<AstNode>>, context: &mut CodeGenContext) {
    if let Some(expr) = expr {
        // Generate code to evaluate the expression and put result in a0
        if let Some(reg) = generate_expression(expr, context) {
            if reg != "a0" {
                context.emit(&format!("    mv a0, {}", reg));
            }
            context.free_register(&reg);
        }
    }

    // Epilogue: restore frame pointer and return
    context.emit("    lw ra, 12(sp)");
    context.emit("    lw s0, 0(sp)");
    context.emit(&format!("    addi sp, sp, {}", context.stack_offset));
    context.emit("    ret");
}
