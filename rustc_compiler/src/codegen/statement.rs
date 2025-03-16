use crate::ast::AstNode;
use crate::codegen::context::CodeGenContext;
use crate::codegen::expression::generate_expression;
use crate::error::CompileError;
use crate::ast::SwitchCase;


/// Generate code for a statement
pub fn generate_statement(node: &AstNode, context: &mut CodeGenContext) -> Result<(), CompileError> {
    match node {
        AstNode::ReturnStatement(_) => generate_return_statement(node, context),
        AstNode::WhileStatement { .. } => generate_while_statement(node, context),
        AstNode::IfStatement { .. } => generate_if_statement(node, context),
        AstNode::Assignment { lhs, rhs } => generate_assignment(lhs, rhs, context),
        AstNode::ForLoop { init, condition, increment, body } => {generate_for_loop(init, condition, increment, body, context)},
        AstNode::BlockStatement(stmts) => {
            for stmt in stmts {
                generate_statement(stmt, context)?;
            }
            Ok(())
        },
        AstNode::SwitchStatement { expr, cases, default } => {
            generate_switch_statement(expr, cases, default, context)
        },
        AstNode::BreakStatement => {
            let label = context.get_current_break_label()
                .ok_or(CompileError::CodegenError("break outside loop/switch".into()))?;
            context.emit(&format!("j {}", label));
            Ok(())
        },
        AstNode::ExpressionStatement(_) => generate_expression_statement(node, context),
        AstNode::UnaryOperation { op, operand } => generate_unary_operation(op, operand, context),
        AstNode::Declaration { type_spec, declarator, initializer } => {
            match &**declarator {
                AstNode::Identifier(name) => {
                    let offset = context.add_variable(name.to_string(), type_spec.to_string());
                    if let Some(init_expr) = initializer {
                        let reg = generate_expression(init_expr, context)?;
                        context.emit(&format!("    sw {}, {}(s0)", reg, offset));
                        context.free_register(&reg);
                    }
                    Ok(())
                },
                AstNode::ArrayDeclarator { base, size } => {
                    if let AstNode::Identifier(name) = &**base {
                        let array_size = if let AstNode::IntConstant(size_val) = &**size {
                            *size_val as usize
                        } else {
                            return Err(CompileError::CodegenError("Array size must be a constant".to_string()));
                        };
                        let offset = context.add_array(name.to_string(), type_spec.to_string(), array_size);
                        // Initializers for arrays can be added later if needed
                        if initializer.is_some() {
                            return Err(CompileError::CodegenError("Array initializers not supported yet".to_string()));
                        }
                        Ok(())
                    } else {
                        Err(CompileError::CodegenError("Invalid array name".to_string()))
                    }
                },
                _ => Err(CompileError::CodegenError("Unsupported declarator type".to_string())),
            }
        },
        AstNode::FunctionDeclaration { .. } => {
            // Function declarations are just prototypes and don't generate code
            // We can emit a comment for debugging purposes
            context.emit(&format!("    # Function declaration: {:?}", node));
            Ok(())
        },
        _ => Err(CompileError::CodegenError(format!("Unsupported statement type: {:?}", node))),
    }
}

fn generate_assignment(
    lhs: &AstNode,
    rhs: &AstNode,
    context: &mut CodeGenContext,
) -> Result<(), CompileError> {
    let rhs_reg = generate_expression(rhs, context)?;

    if let AstNode::Identifier(var_name) = &*lhs {
        if let Some((offset, _)) = context.get_variable(var_name.as_str()) {
            context.emit(&format!("sw {}, {}(s0)", rhs_reg, offset));
            context.free_register(&rhs_reg);
            return Ok(());
        }
    }
    Err(CompileError::CodegenError("Invalid assignment target".to_string()))
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

/// Generate code for a for statement
fn generate_for_loop(
    init: &Box<AstNode>,
    condition: &Box<AstNode>,
    increment: &Box<AstNode>,
    body: &Box<AstNode>,
    context: &mut CodeGenContext,
) -> Result<(), CompileError> {
    // Generate initialization code
    generate_statement(init, context)?;

    let loop_start = context.generate_label("loop_start");
    let loop_cond = context.generate_label("loop_cond");
    let loop_end = context.generate_label("loop_end");

    // Jump to condition check first
    context.emit(&format!("j {}", loop_cond));

    // Loop body label
    context.emit(&format!("{}:", loop_start));

    // Generate loop body
    generate_statement(body, context)?;

    // Generate increment code
    generate_statement(increment, context)?;

    // Condition check label
    context.emit(&format!("{}:", loop_cond));

    // Generate condition check
    let cond_reg = generate_expression(condition, context)?;
    context.emit(&format!("bnez {}, {}", cond_reg, loop_start));
    context.free_register(&cond_reg);

    // End label
    context.emit(&format!("{}:", loop_end));

    Ok(())
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

/// Generate code for a unary operation statement
fn generate_unary_operation(
    op: &str,
    operand: &AstNode,
    context: &mut CodeGenContext,
) -> Result<(), CompileError> {
    // Handle post-increment/decrement and pre-increment/decrement
    match op {
        "post++" | "post--" | "++" | "--" => {
            if let AstNode::Identifier(var_name) = operand {
                if let Some((offset, _)) = context.get_variable(var_name.as_str()) {
                    // Get a register for computation
                    let reg = context.get_register();

                    // Load the current value
                    context.emit(&format!("    lw {}, {}(s0)", reg, offset));

                    // Increment or decrement
                    match op {
                        "post++" | "++" => context.emit(&format!("    addi {}, {}, 1", reg, reg)),
                        "post--" | "--" => context.emit(&format!("    addi {}, {}, -1", reg, reg)),
                        _ => unreachable!(),
                    }

                    // Store the updated value back
                    context.emit(&format!("    sw {}, {}(s0)", reg, offset));

                    // Free the register
                    context.free_register(&reg);

                    return Ok(());
                }
            }
            Err(CompileError::CodegenError(format!("Invalid operand for {} operation", op)))
        },
        _ => Err(CompileError::CodegenError(format!("Unsupported unary operation: {}", op))),
    }
}

fn generate_switch_statement(
    expr: &AstNode,
    cases: &[SwitchCase],
    default: &Option<Vec<Box<AstNode>>>,
    context: &mut CodeGenContext,
) -> Result<(), CompileError> {
    let end_label = context.generate_label("switch_end");
    let default_label = context.generate_label("switch_default");

    // Evaluate switch expression
    let expr_reg = generate_expression(expr, context)?;

    // Generate case comparisons
    let mut case_labels = Vec::new();
    for case in cases {
        let label = context.generate_label("case");
        case_labels.push(label.clone());

        // Compare with case value
        let case_value_reg = generate_expression(&case.value, context)?;
        context.emit(&format!("    beq {0}, {1}, {2}", expr_reg, case_value_reg, label));
        context.free_register(&case_value_reg);
    }

    // Handle default case
    if default.is_some() {
        context.emit(&format!("    j {}", default_label));
    } else {
        context.emit(&format!("    j {}", end_label));
    }

    // Generate case bodies
    for (i, case) in cases.iter().enumerate() {
        context.emit(&format!("{}:", case_labels[i]));
        context.push_break_label(end_label.clone());

        for stmt in &case.body {
            generate_statement(stmt, context)?;
        }

        context.pop_break_label();
    }

    // Generate default body
    if let Some(default_body) = default {
        context.emit(&format!("{}:", default_label));
        context.push_break_label(end_label.clone());

        for stmt in default_body {
            generate_statement(stmt, context)?;
        }

        context.pop_break_label();
    }

    context.emit(&format!("{}:", end_label));
    context.free_register(&expr_reg);
    Ok(())
}
