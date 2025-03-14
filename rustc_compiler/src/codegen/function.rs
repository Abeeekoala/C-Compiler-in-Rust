use crate::ast::AstNode;
use crate::codegen::context::{CodeGenContext, StorageLocation, Symbol};
use crate::codegen::statement::generate_statement;
use crate::error::CompileError;

/// Generate code for a function definition
pub fn generate_function(node: &AstNode, context: &mut CodeGenContext) -> Result<(), CompileError> {
    if let AstNode::FunctionDefinition { declarator, parameters, compound_statement, .. } = node {
        // Get function name from declarator
        if let AstNode::Identifier(name) = &**declarator {
            // Set current function name
            context.current_function = Some(name.clone());

            // Generate function label
            context.emit(&format!(".globl {}", name));
            context.emit(&format!("{}:", name));

            // Generate function prologue
            context.generate_function_prologue();

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
                        };

                        context.add_symbol(param_name, symbol);
                        context.variables.insert(param_name.clone(), (stack_offset, type_str));
                    }
                }
            }

            // Generate code for function body
            generate_statement(compound_statement, context)?;

            // If there's no explicit return at the end, add one
            if !context.output.trim().ends_with("ret") {
                context.generate_function_epilogue();
            }

            // Reset temporary registers after function is complete
            context.reset_temp_registers();

            Ok(())
        } else {
            Err(CompileError::CodegenError("Expected function name".to_string()))
        }
    } else {
        Err(CompileError::CodegenError("Expected function definition".to_string()))
    }
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
