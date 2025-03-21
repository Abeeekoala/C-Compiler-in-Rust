use crate::ast::AstNode;
use crate::ast::TypeSpecifier;
use crate::codegen::context::CodeGenContext;
use crate::codegen::expression::generate_expression;
use crate::error::CompileError;
use crate::ast::SwitchCase;
use crate::codegen::context::StorageLocation;
use crate::codegen::expression::get_expression_type;
use crate::codegen::expression;
use crate::codegen::context::FullType;

/// Generate code for a statement
pub fn generate_statement(node: &AstNode, context: &mut CodeGenContext) -> Result<(), CompileError> {
    match node {
        AstNode::TypedefDeclaration { type_spec, declarator } => {
            let full_type = build_full_type(type_spec, declarator, context)?;
            let base_name = extract_base_identifier(declarator)?;
            context.typedef_map.insert(base_name, full_type);
            Ok(()) // No code generated for typedef
        },
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
                .ok_or(CompileError::CodegenError("break is outside loop/switch".into()))?;
            context.emit(&format!("j {}", label));
            Ok(())
        },
        AstNode::ContinueStatement => {
            let label = context.get_current_continue_label()
                .ok_or(CompileError::CodegenError("continue is outside loop/switch".into()))?;
            context.emit(&format!("j {}", label));
            Ok(())
        },
        AstNode::ExpressionStatement(_) => generate_expression_statement(node, context),
        AstNode::UnaryOperation { op, operand } => generate_unary_operation(op, operand, context),
        AstNode::Declaration { type_spec, declarator, initializer } => {
            generate_declaration_item(node, context)
        },
        AstNode::FunctionDeclaration { .. } => {
            // Function declarations are just prototypes and don't generate code
            // We can emit a comment for debugging purposes
            context.emit(&format!("    # Function declaration: {:?}", node));
            Ok(())
        },
        AstNode::EnumDefinition { name, values } => {
            context.register_enum(name.clone(), values.clone());
            // Enum definitions are just prototypes and don't generate code
            // We can emit a comment for debugging purposes
            context.emit(&format!("    # Enum definition: {:?}", node));
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
    if let AstNode::Identifier(var_name) = &*lhs {
        if let Some((offset, lhs_type)) = context.get_variable(var_name.as_str()) {
            // Get the type and register for the right-hand side
            let rhs_type = get_expression_type(rhs, context)?;
            let rhs_reg = generate_expression(rhs, context)?;
            println!(
                "Assigning to '{}' (type: {}) at offset {} with value (type: {}) in register {}",
                var_name, lhs_type, offset, rhs_type, rhs_reg
            );
            // Handle assignment based on type combinations
            match (lhs_type.as_str(), rhs_type.as_str()) {
                ("int" | "char", "int" | "char") => {
                    // Integer to integer assignment
                    context.emit(&format!("sw {}, {}(s0)", rhs_reg, offset));
                    context.free_register(&rhs_reg);
                },
                ("float", "float") => {
                    // Float to float assignment
                    if rhs_reg.starts_with('f') {
                        context.emit(&format!("fsw {}, {}(s0)", rhs_reg, offset));
                        context.free_fp_register(&rhs_reg);
                    } else {
                        return Err(CompileError::CodegenError(
                            "Expected floating-point register for float assignment".to_string()
                        ));
                    }
                },
                ("double", "double") => {
                    // Double to double assignment
                    if rhs_reg.starts_with('f') {
                        context.emit(&format!("fsd {}, {}(s0)", rhs_reg, offset));
                        context.free_fp_register(&rhs_reg);
                    } else {
                        return Err(CompileError::CodegenError(
                            "Expected floating-point register for double assignment".to_string()
                        ));
                    }
                },
                ("float", "int") => {
                    // Integer to float conversion
                    let fp_reg = context.get_fp_register();
                    context.emit(&format!("fcvt.s.w {}, {}", fp_reg, rhs_reg));
                    context.emit(&format!("fsw {}, {}(s0)", fp_reg, offset));
                    context.free_register(&rhs_reg);
                    context.free_fp_register(&fp_reg);
                },
                ("double", "int") => {
                    // Integer to double conversion
                    let fp_reg = context.get_fp_register();
                    context.emit(&format!("fcvt.d.w {}, {}", fp_reg, rhs_reg));
                    context.emit(&format!("fsd {}, {}(s0)", fp_reg, offset));
                    context.free_register(&rhs_reg);
                    context.free_fp_register(&fp_reg);
                },
                ("int", "float") => {
                    // Float to integer conversion
                    if rhs_reg.starts_with('f') {
                        let int_reg = context.get_register();
                        context.emit(&format!("fcvt.w.s {}, {}", int_reg, rhs_reg));
                        context.emit(&format!("sw {}, {}(s0)", int_reg, offset));
                        context.free_fp_register(&rhs_reg);
                        context.free_register(&int_reg);
                    } else {
                        return Err(CompileError::CodegenError(
                            "Expected floating-point register for float expression".to_string()
                        ));
                    }
                },
                _ => {
                    return Err(CompileError::CodegenError(
                        format!("Type mismatch in assignment: {} = {}", lhs_type, rhs_type)
                    ));
                }
            }
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

        context.push_break_label(end_label.clone());
        context.push_continue_label(start_label.clone());
        // Emit start label
        context.emit(&format!("{}:", start_label));

        // Generate condition expression
        let cond_reg = generate_expression(condition, context)?;

        // If condition is false, exit loop
        context.emit(&format!("    beqz {}, {}", cond_reg, end_label));
        context.free_register(&cond_reg);

        // Generate loop body
        generate_statement(body, context)?;

        context.pop_break_label();
        context.pop_continue_label();

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
    generate_statement(init, context)?;

    // Generate all labels first
    let loop_start = context.generate_label("loop_start");
    let loop_increment = context.generate_label("loop_increment");
    let loop_cond = context.generate_label("loop_cond");
    let loop_end = context.generate_label("loop_end");

    context.emit(&format!("j {}", loop_cond));
    context.emit(&format!("{}:", loop_start));

    // Push labels before generating body
    context.push_break_label(loop_end.clone());
    context.push_continue_label(loop_increment.clone());

    generate_statement(body, context)?;

    // Pop labels after body generation
    context.pop_break_label();
    context.pop_continue_label();

    // Add the increment label and code
    context.emit(&format!("{}:", loop_increment));
    generate_statement(increment, context)?;

    // Condition check
    context.emit(&format!("{}:", loop_cond));
    let cond_reg = generate_expression(condition, context)?;
    context.emit(&format!("bnez {}, {}", cond_reg, loop_start));
    context.free_register(&cond_reg);

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
            let expr_type = get_expression_type(expr, context)?;
            let result_reg = generate_expression(expr, context)?;

            // Handle floating-point vs integer return values
            if expr_type == "float"{
                if result_reg.starts_with('f') {
                    // Already in floating-point register, move to fa0 if needed
                    if result_reg != "fa0" {
                        context.emit(&format!("    fmv.s fa0, {}", result_reg));
                        context.free_fp_register(&result_reg);
                    }
                } else {
                    // Integer register, convert to float in fa0
                    context.emit(&format!("    fcvt.s.w fa0, {}", result_reg));
                    context.free_register(&result_reg);
                }
            } else if expr_type == "double" {
                if result_reg.starts_with('f') {
                    context.emit(&format!("    fmv.d fa0, {}", result_reg));
                    context.free_fp_register(&result_reg);
                }
            } else {
                // Integer return, move to a0 if needed
                if result_reg.starts_with('f') {
                    // Floating-point register, convert to integer
                    context.emit(&format!("    fcvt.w.s a0, {}", result_reg));
                    context.free_fp_register(&result_reg);
                } else if result_reg != "a0" {
                    context.emit(&format!("    mv a0, {}", result_reg));
                    context.free_register(&result_reg);
                }
            }
        }

        // Generate function epilogue
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
    match node {
        AstNode::Declaration { type_spec, declarator, initializer } => {
            // Convert TypeSpecifier to base type string
            let full_type = match type_spec {
                TypeSpecifier::TypedefName(name) => {
                    context.typedef_map.get(name).cloned().ok_or_else(|| {
                        CompileError::CodegenError(format!("Unknown typedef: {}", name))
                    })?
                }
                _ => FullType::Base(type_spec.clone()),
            }.to_string();

            // Handle array declarations
            if let AstNode::ArrayDeclarator { .. } = &**declarator {
                let (name, dimensions) = extract_array_declarator(declarator)?;
                let offset = context.add_array(name.to_string(), full_type, dimensions.clone());

                if let Some(init) = initializer {
                    match &**init {
                        AstNode::InitializerList(elements) => {
                            initialize_array(&name, &dimensions, elements, offset, context)?;
                        },
                        _ => {
                            return Err(CompileError::CodegenError("Array initializer must be an initializer list".to_string()));
                        }
                    }
                }
                return Ok(());
            }

            // Handle simple variables and pointers
            let name = extract_base_identifier(&**declarator)?;
            let type_str = compute_type_string(&**declarator, &full_type);

            // Determine size based on type
            let size = if type_str.ends_with("*") {
                4 // Pointers are 4 bytes
            } else {
                match type_spec {
                    TypeSpecifier::Double => 8,
                    _ => 4, // int, float, char
                }
            };

            // Allocate space
            let stack_offset = context.add_variable(name.to_string(), type_str.clone());

            // Handle initializer
            if let Some(init_expr) = initializer {
                // Special case for char* with string literal
                if type_str == "char*" || type_str.starts_with("char *") {
                    if let AstNode::StringLiteral(string_value) = &**init_expr {
                        // For string literals initializing char pointers
                        if context.in_function {
                            // For local variables, create a string in data section with unique label
                            let string_label = context.generate_label("str");

                            // Store string in data section with null terminator
                            let escaped_string = string_value.replace("\\", "\\\\")
                                                           .replace("\n", "\\n")
                                                           .replace("\t", "\\t")
                                                           .replace("\"", "\\\"");
                            context.emit_data(&format!("{}:", string_label));
                            context.emit_data(&format!("    .string \"{}\"", escaped_string));

                            // Load address of string into a register
                            let addr_reg = context.get_register();
                            context.emit(&format!("    la {}, {}", addr_reg, string_label));

                            // Store register into the pointer variable
                            context.emit(&format!("    sw {}, {}(s0)", addr_reg, stack_offset));
                            context.free_register(&addr_reg);
                        } else {
                            // For global variables, similar but simpler
                            let string_label = context.generate_label("str");
                            let escaped_string = string_value.replace("\\", "\\\\")
                                                           .replace("\n", "\\n")
                                                           .replace("\t", "\\t")
                                                           .replace("\"", "\\\"");

                            // Add string to data section
                            context.emit_data(&format!("{}:", string_label));
                            context.emit_data(&format!("    .string \"{}\"", escaped_string));

                            // Initialize global pointer to string
                            context.emit_data(&format!("{}:", name));
                            context.emit_data(&format!("    .word {}", string_label));
                        }
                        return Ok(());
                    }
                }

                // Regular initialization (existing code)
                if context.in_function {
                    // Local variable/pointer initialization
                    let reg = expression::generate_expression(init_expr, context)?;

                    // Store based on type
                    if type_str == "float" && reg.starts_with('f') {
                        context.emit(&format!("    fsw {}, {}(s0)", reg, stack_offset));
                        context.free_fp_register(&reg);
                    } else if type_str == "double" && reg.starts_with('f') {
                        context.emit(&format!("    fsd {}, {}(s0)", reg, stack_offset));
                        context.free_fp_register(&reg);
                    } else if (type_str == "float" || type_str == "double") && !reg.starts_with('f') {
                        let fp_reg = context.get_fp_register();
                        context.emit(&format!("    fcvt.s.w {}, {}", fp_reg, reg));
                        context.emit(&format!("    fsw {}, {}(s0)", fp_reg, stack_offset));
                        context.free_register(&reg);
                        context.free_fp_register(&fp_reg);
                    } else {
                        // Integers, pointers, and unsigned types use sw
                        context.emit(&format!("    sw {}, {}(s0)", reg, stack_offset));
                        context.free_register(&reg);
                    }
                } else {
                    // Global variable/pointer initialization
                    if type_str == "float" || type_str == "double" {
                        if let AstNode::FloatConstant(value) = &**init_expr {
                            let float_bits = f32::to_bits(*value as f32);
                            context.initialize_global_variable_raw(name.to_string(), float_bits);
                        } else {
                            return Err(CompileError::CodegenError(
                                "Global floating-point variable initializer must be a constant".to_string()
                            ));
                        }
                    } else if let AstNode::IntConstant(value) = &**init_expr {
                        context.initialize_global_variable(name.to_string(), *value);
                    } else {
                        return Err(CompileError::CodegenError(
                            "Global variable initializer must be a constant".to_string()
                        ));
                    }
                }
            }
            Ok(())
        },
        _ => Err(CompileError::CodegenError("Expected declaration".to_string())),
    }
}

fn type_spec_to_string(type_spec: &TypeSpecifier) -> String {
    match type_spec {
        TypeSpecifier::Int => "int".to_string(),
        TypeSpecifier::Char => "char".to_string(),
        TypeSpecifier::Float => "float".to_string(),
        TypeSpecifier::Double => "double".to_string(),
        TypeSpecifier::Void => "void".to_string(),
        TypeSpecifier::Struct(name) => format!("struct {}", name),
        _ => "unknown".to_string(),
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

fn extract_base_identifier(declarator: &AstNode) -> Result<String, CompileError> {
    match declarator {
        AstNode::Identifier(name) => Ok(name.clone()),
        AstNode::PointerDeclarator { pointee } => extract_base_identifier(&**pointee),
        AstNode::ArrayDeclarator { base, .. } => extract_base_identifier(&**base),
        _ => Err(CompileError::CodegenError("Invalid declarator".to_string())),
    }
}

fn compute_type_string(declarator: &AstNode, base_type: &str) -> String {
    match declarator {
        AstNode::Identifier(_) => base_type.to_string(),
        AstNode::PointerDeclarator { pointee } => {
            let pointee_type = compute_type_string(&**pointee, base_type);
            format!("{}*", pointee_type)
        },
        AstNode::ArrayDeclarator { base, size } => {
            let base_type_str = compute_type_string(&**base, base_type);
            if let AstNode::IntConstant(n) = &**size {
                format!("{}[{}]", base_type_str, n)
            } else {
                panic!("Array size must be constant");
            }
        },
        _ => panic!("Unsupported declarator"),
    }
}

fn build_full_type(
    type_spec: &TypeSpecifier,
    declarator: &AstNode,
    context: &mut CodeGenContext,
) -> Result<FullType, CompileError> {
    println!("Building full type for: {:?}", declarator);
    match declarator {
        AstNode::Identifier(_) => {
            // Simple identifier: use the base type_spec
            Ok(FullType::Base(type_spec.clone()))
        }
        AstNode::PointerDeclarator { pointee } => {
            // Pointer: build the pointee's type and wrap it
            let pointee_type = build_full_type(type_spec, pointee, context)?;
            Ok(FullType::Pointer(Box::new(pointee_type)))
        }
        _ => Err(CompileError::CodegenError(
            format!("Unsupported declarator in typedef: {:?}", declarator),
        )),
    }
}
