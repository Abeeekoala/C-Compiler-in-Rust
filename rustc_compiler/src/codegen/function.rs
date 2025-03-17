use crate::ast::AstNode;
use crate::codegen::context::{CodeGenContext, StorageLocation, Symbol};
use crate::codegen::statement::generate_statement;
use crate::codegen::expression::generate_expression;
use crate::error::CompileError;

/// Generate code for a function definition
pub fn generate_function(node: &AstNode, context: &mut CodeGenContext) -> Result<(), CompileError> {
    match node {
        AstNode::FunctionDefinition { declarator, parameters, compound_statement, .. } => {
            if let AstNode::Identifier(name) = &**declarator {
                // Store function declaration before generating code
                let param_types = parameters.iter()
                    .map(|p| match &**p {
                        AstNode::Declaration { type_spec, .. } => type_spec.to_string(),
                        _ => "unknown".to_string(),
                    })
                    .collect();
                context.declare_function(name, param_types);

                // Generate function prologue
                context.emit(&format!(".globl {}", name));
                context.emit(&format!("{}:", name));
                context.generate_function_prologue();

                // Create new scope for function body
                context.enter_scope();

                // Process parameters - add them to the symbol table
                for (i, param) in parameters.iter().enumerate().take(8) {
                    if let AstNode::Declaration { type_spec, declarator, .. } = &**param {
                        if let AstNode::Identifier(param_name) = &**declarator {
                            // RISC-V convention: first 8 args in a0-a7
                            let reg_name = format!("a{}", i);

                            // Allocate space on stack for the parameter
                            let stack_offset = context.allocate_stack_space(4); // Int size = 4 bytes

                            // Store argument from register to stack
                            context.emit(&format!("    sw {}, {}(s0)", reg_name, stack_offset));

                            // Get parameter type as string
                            let type_str = match type_spec {
                                crate::ast::TypeSpecifier::Int => "int".to_string(),
                                crate::ast::TypeSpecifier::Char => "char".to_string(),
                                crate::ast::TypeSpecifier::Void => "void".to_string(),
                                _ => todo!(),
                            };

                            // Add to symbol table
                            let symbol = Symbol {
                                location: StorageLocation::Stack(stack_offset),
                                size: 4, // Assuming int
                                type_info: type_str.clone(),
                                dimensions: Vec::new(),
                            };

                            context.add_symbol(param_name, symbol);
                        }
                    }
                }

                // Generate function body
                generate_statement(compound_statement, context)?;

                // Exit function scope
                context.exit_scope();

                // Generate epilogue if needed
                if !context.output.trim().ends_with("ret") {
                    context.generate_function_epilogue();
                }

                Ok(())
            } else {
                Err(CompileError::CodegenError("Expected function name".to_string()))
            }
        },
        AstNode::FunctionDeclaration { declarator, parameters, .. } => {
            if let AstNode::Identifier(name) = &**declarator {
                // Just store the declaration
                let param_types = parameters.iter()
                    .map(|p| match &**p {
                        AstNode::Declaration { type_spec, .. } => type_spec.to_string(),
                        _ => "unknown".to_string(),
                    })
                    .collect();
                context.declare_function(name, param_types);
                Ok(())
            } else {
                Err(CompileError::CodegenError("Expected function name".to_string()))
            }
        },
        _ => Err(CompileError::CodegenError("Expected function definition or declaration".to_string())),
    }
}

pub fn generate_function_call(
    function: &AstNode,
    args: &[Box<AstNode>],
    context: &mut CodeGenContext
) -> Result<String, CompileError> {
    // Get new register for result before saving registers
    let result_reg = context.get_register();

    // Save all caller-saved registers
    context.save_registers_for_call();

    // Evaluate and pass arguments in reverse order to handle recursive evaluation
    for (i, arg) in args.iter().enumerate().take(8) {
        let arg_reg = generate_expression(arg, context)?;
        if arg_reg != format!("a{}", i) {
            context.emit(&format!("    mv a{}, {}", i, arg_reg));
        }
        context.free_register(&arg_reg);
    }

    // Make the function call
    if let AstNode::Identifier(name) = function {
        context.emit(&format!("    call {}", name));
    } else {
        return Err(CompileError::CodegenError("Expected function name".to_string()));
    }

    if result_reg != "a0" {
        context.emit(&format!("    mv {}, a0", result_reg));
    }
    context.restore_registers_after_call();

    Ok(result_reg)
}

/// Extract function parameters from a function declarator
fn get_function_params(declarator: &AstNode) -> Option<Vec<(String, String)>> {
    match declarator {
        AstNode::Identifier(_) => {
            // No parameters defined in simple identifier
            Some(Vec::new())
        },
        AstNode::FunctionCall { function, args } => {
            // Function declarator might be represented as a FunctionCall node
            // where function is the identifier and args are parameter declarations
            if let AstNode::Identifier(_) = **function {
                // Extract parameter information from args
                let mut params = Vec::new();
                for arg in args {
                    if let AstNode::Declaration { type_spec, declarator, .. } = &**arg {
                        if let AstNode::Identifier(param_name) = &**declarator {
                            // Get parameter type as string
                            let type_str = match type_spec {
                                crate::ast::TypeSpecifier::Int => "int".to_string(),
                                crate::ast::TypeSpecifier::Char => "char".to_string(),
                                // Add other types as needed
                                _ => "unknown".to_string(),
                            };
                            params.push((param_name.clone(), type_str));
                        }
                    }
                }
                Some(params)
            } else {
                None
            }
        },
        _ => None,
    }
}
