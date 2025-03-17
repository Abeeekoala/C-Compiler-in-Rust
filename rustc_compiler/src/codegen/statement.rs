use crate::ast::AstNode;
use crate::codegen::context::CodeGenContext;
use crate::codegen::expression::generate_expression;
use crate::error::CompileError;
use crate::ast::SwitchCase;
use crate::codegen::context::StorageLocation;


/// Generate code for a statement
pub fn generate_statement(node: &AstNode, context: &mut CodeGenContext) -> Result<(), CompileError> {
    match node {
        AstNode::ReturnStatement(_) => generate_return_statement(node, context),
        AstNode::WhileStatement { .. } => generate_while_statement(node, context),
        AstNode::IfStatement { .. } => generate_if_statement(node, context),
        AstNode::Assignment { lhs, rhs } => generate_assignment(lhs, rhs, context),
        AstNode::ForLoop { init, condition, increment, body } => {generate_for_loop(init, condition, increment, body, context)},
        AstNode::BlockStatement(stmts) => {
            context.enter_scope();
            for stmt in stmts {
                generate_statement(stmt, context)?;
            }
            context.exit_scope();
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
                    let (name, dimensions) = extract_array_declarator(declarator)?;
                    let offset = context.add_array(name.to_string(), type_spec.to_string(), dimensions.clone());
                    // Initializers for arrays can be added later if needed
                    if let Some(init) = initializer {
                        match &**init {
                            AstNode::InitializerList(elements) => {
                                // Initialize array with initializer list
                                initialize_array(name.as_str(), &dimensions, elements, offset, context)?;
                            },
                            _ => {
                                return Err(CompileError::CodegenError("Array initializer must be an initializer list".to_string()));
                            }
                        }
                    }
                    Ok(())
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

fn extract_array_declarator(declarator: &AstNode) -> Result<(String, Vec<usize>), CompileError> {
    let mut sizes = Vec::new();
    let mut current = declarator;
    loop {
        match current {
            AstNode::ArrayDeclarator { base, size } => {
                if let AstNode::IntConstant(val) = **size {
                    sizes.push(val as usize);
                    current = base;
                } else {
                    return Err(CompileError::CodegenError("Array size must be a constant integer".to_string()));
                }
            },
            AstNode::Identifier(name) => {
                sizes.reverse();
                return Ok((name.clone(), sizes));
            },
            _ => return Err(CompileError::CodegenError("Invalid array declarator".to_string())),
        }
    }
}

fn initialize_array(
    array_name: &str,
    dimensions: &[usize],
    elements: &[Box<AstNode>],
    base_offset: i32,
    context: &mut CodeGenContext,
) -> Result<(), CompileError> {
    // Check if this is a global array
    let is_global = if let Some(symbol) = context.lookup_symbol(array_name) {
        match symbol.location {
            StorageLocation::Global(_) => true,
            _ => false,
        }
    } else {
        return Err(CompileError::CodegenError(format!("Array '{}' not found", array_name)));
    };

    if is_global {
        let array_decl = format!("{}:", array_name);
        let mut data_lines: Vec<&str> = context.data_section.lines().collect();
        let mut i = 0;
        while i < data_lines.len() {
            if data_lines[i].trim() == array_decl {
                data_lines.remove(i);
                if i < data_lines.len() {
                    data_lines.remove(i);
                }
                break;
            }
            i += 1;
        }
        context.data_section = data_lines.join("\n") + "\n";

        // Add array with initialization
        context.emit_data(&format!("{}:", array_name));

        // For multi-dimensional arrays, flatten all values into a single list
        let flat_values = flatten_initializer_list(elements, dimensions)?;

        // Combine the directive and values on one line
        context.emit_data(&format!("    .word {}", flat_values.join(", ")));
    } else {
        // For local arrays, use the existing implementation
        if dimensions.len() > 1 && !elements.is_empty() {
            match &*elements[0] {
                AstNode::InitializerList(_) => {
                    return initialize_multi_dimensional_array(array_name, dimensions, elements, base_offset, context);
                },
                _ => {
                    return initialize_flat_array(array_name, dimensions, elements, base_offset, context);
                }
            }
        }

        return initialize_flat_array(array_name, dimensions, elements, base_offset, context);
    }

    Ok(())
}

/// Recursively flatten a nested initializer list into a single vector of constant values
fn flatten_initializer_list(
    elements: &[Box<AstNode>],
    dimensions: &[usize],
) -> Result<Vec<String>, CompileError> {
    let mut result = Vec::new();

    // Calculate the total expected elements for padding
    let total_elements: usize = dimensions.iter().product();

    if dimensions.len() <= 1 {
        // Base case: process the 1D array elements
        for element in elements {
            match &**element {
                AstNode::IntConstant(value) => {
                    result.push(value.to_string());
                },
                _ => {
                    return Err(CompileError::CodegenError(
                        "Array initializers must be constants or nested initializer lists".to_string()
                    ));
                }
            }
        }

        // Pad with zeros if needed
        while result.len() < total_elements {
            result.push("0".to_string());
        }
    } else {
        // Recursive case: handle nested dimensions
        let sub_array_size: usize = dimensions[1..].iter().product();
        let mut processed_elements = 0;

        for (i, element) in elements.iter().enumerate() {
            if i >= dimensions[0] {
                break; // Don't process more elements than the first dimension allows
            }

            match &**element {
                AstNode::InitializerList(sub_elements) => {
                    // Convert Box<AstNode> to a slice of references
                    let sub_elements_ref: Vec<&Box<AstNode>> = sub_elements.iter().collect();
                    let sub_elements_boxed: Vec<Box<AstNode>> =
                        sub_elements_ref.iter().map(|e| (**e).clone().into()).collect();

                    // Recursively process this sub-array
                    let mut sub_results = flatten_initializer_list(
                        &sub_elements_boxed,
                        &dimensions[1..]
                    )?;

                    result.append(&mut sub_results);
                    processed_elements += 1;
                },
                _ => {
                    return Err(CompileError::CodegenError(
                        "Expected nested initializer list for multi-dimensional array".to_string()
                    ));
                }
            }
        }

        // Pad with empty sub-arrays if needed
        while processed_elements < dimensions[0] {
            // Add a full sub-array of zeros
            for _ in 0..sub_array_size {
                result.push("0".to_string());
            }
            processed_elements += 1;
        }
    }

    Ok(result)
}

/// Initialize a flat array from a list of expressions
fn initialize_flat_array(
    array_name: &str,
    dimensions: &[usize],
    elements: &[Box<AstNode>],
    base_offset: i32,
    context: &mut CodeGenContext,
) -> Result<(), CompileError> {
    let addr_reg = context.get_register();

    // Initialize each provided element
    for (i, element) in elements.iter().enumerate() {
        // Evaluate the initializer expression
        let value_reg = generate_expression(element, context)?;

        // Calculate element address: base_offset + i * 4
        if i == 0 {
            // First element: just set base address
            if base_offset >= -2048 && base_offset <= 2047 {
                context.emit(&format!("    addi {}, s0, {}", addr_reg, base_offset));
            } else {
                let temp_reg = context.get_register();
                context.emit(&format!("    li {}, {}", temp_reg, base_offset));
                context.emit(&format!("    add {}, s0, {}", addr_reg, temp_reg));
                context.free_register(&temp_reg);
            }
        } else {
            // Subsequent elements: increment address by 4
            context.emit(&format!("    addi {}, {}, 4", addr_reg, addr_reg));
        }

        // Store the value to the calculated address
        context.emit(&format!("    sw {}, 0({})", value_reg, addr_reg));

        // Free value register
        context.free_register(&value_reg);
    }

    // Free address register
    context.free_register(&addr_reg);

    Ok(())
}

/// Initialize a multi-dimensional array from nested initializer lists
fn initialize_multi_dimensional_array(
    array_name: &str,
    dimensions: &[usize],
    elements: &[Box<AstNode>],
    base_offset: i32,
    context: &mut CodeGenContext,
) -> Result<(), CompileError> {
    // Calculate size of each sub-array
    let sub_array_elements: usize = dimensions.iter().skip(1).product();

    // Process each nested initializer list
    for (i, element) in elements.iter().enumerate() {
        match &**element {
            AstNode::InitializerList(sub_elements) => {
                // Calculate offset for this sub-array
                let sub_array_offset = base_offset + (i * sub_array_elements * 4) as i32;

                // Recursively initialize this sub-array
                initialize_array(
                    array_name,
                    &dimensions[1..],
                    sub_elements,
                    sub_array_offset,
                    context,
                )?;
            },
            _ => {
                return Err(CompileError::CodegenError(
                    "Expected nested initializer list for multi-dimensional array".to_string()
                ));
            }
        }
    }

    Ok(())
}
