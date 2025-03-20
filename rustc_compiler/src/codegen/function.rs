use crate::ast::AstNode;
use crate::codegen::context::{CodeGenContext, StorageLocation, Symbol};
use crate::codegen::statement::generate_statement;
use crate::codegen::expression::generate_expression;
use crate::error::CompileError;

/// Generate code for a function definition
pub fn generate_function(node: &AstNode, context: &mut CodeGenContext) -> Result<(), CompileError> {
    match node {
        AstNode::FunctionDefinition { decl_specifiers, declarator, parameters, compound_statement } => {
            if let AstNode::Identifier(name) = &**declarator {
                // Extract return type from decl_specifiers
                let return_type_str = extract_return_type_from_decl_specifiers(decl_specifiers)?;

                // Store function declaration before generating code
                let param_types = parameters.iter()
                    .map(|p| match &**p {
                        AstNode::Declaration { type_spec, .. } => type_spec_to_string(type_spec),
                        _ => "unknown".to_string(),
                    })
                    .collect();
                context.declare_function(name, return_type_str, param_types);

                // Generate function prologue
                context.emit(&format!(".globl {}", name));
                context.emit(&format!("{}:", name));
                context.generate_function_prologue();

                // Create new scope for function body
                context.enter_scope();

                // Process parameters - add them to the symbol table
                let mut int_param_count = 0;
                let mut float_param_count = 0;

                for param in parameters.iter().take(16) { // Support up to 16 params (8 int + 8 float)
                    if let AstNode::Declaration { type_spec, declarator, .. } = &**param {
                        // Check if the declarator is a pointer
                        let (param_name, is_pointer) = match &**declarator {
                            AstNode::Identifier(name) => (name.clone(), false),
                            AstNode::PointerDeclarator { pointee } => {
                                if let AstNode::Identifier(name) = &**pointee {
                                    (name.clone(), true)
                                } else {
                                    return Err(CompileError::CodegenError("Expected identifier in pointer declarator".to_string()));
                                }
                            },
                            _ => return Err(CompileError::CodegenError("Expected identifier or pointer declarator".to_string())),
                        };

                        // Get parameter type as string
                        let mut type_str = type_spec_to_string(type_spec);

                        // Add pointer notation to type
                        if is_pointer {
                            type_str = format!("{}*", type_str);
                        }

                        // Determine if parameter is floating-point
                        let is_float = type_str == "float" || type_str == "double";

                        // Get appropriate register based on type and parameter count
                        let reg_name = if is_float {
                            // Use float param count for fa registers
                            if float_param_count >= 8 {
                                panic!("Too many float parameters");
                            } else {
                                format!("fa{}", float_param_count)
                            }
                        } else {
                            // Use int param count for a registers (pointers use integer registers)
                            if int_param_count >= 8 {
                                panic!("Too many integer parameters");
                            } else {
                                format!("a{}", int_param_count)
                            }
                        };

                        // Increment the appropriate counter
                        if is_float {
                            float_param_count += 1;
                        } else {
                            int_param_count += 1;
                        }

                        // Allocate space on stack for the parameter
                        let size = if type_str == "double" { 8 } else { 4 }; // pointers are 4 bytes on 32-bit
                        let stack_offset = context.allocate_stack_space(size);

                        // Store argument from register to stack
                        if is_float {
                            if type_str == "double" {
                                context.emit(&format!("    fsd {}, {}(s0)", reg_name, stack_offset));
                            } else {
                                context.emit(&format!("    fsw {}, {}(s0)", reg_name, stack_offset));
                            }
                        } else {
                            context.emit(&format!("    sw {}, {}(s0)", reg_name, stack_offset));
                        }

                        // Add to symbol table with pointer information
                        let mut symbol = Symbol {
                            location: StorageLocation::Stack(stack_offset),
                            size,
                            type_info: type_str.clone(),
                            dimensions: Vec::new(),
                            is_pointer,
                        };

                        context.add_symbol(&param_name, symbol);
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
        AstNode::FunctionDeclaration { decl_specifiers, declarator, parameters } => {
            if let AstNode::Identifier(name) = &**declarator {
                // Extract return type from decl_specifiers
                let return_type_str = extract_return_type_from_decl_specifiers(decl_specifiers)?;

                // Just store the declaration
                let param_types = parameters.iter()
                    .map(|p| match &**p {
                        AstNode::Declaration { type_spec, .. } => type_spec_to_string(type_spec),
                        _ => "unknown".to_string(),
                    })
                    .collect();
                context.declare_function(name, return_type_str, param_types);
                Ok(())
            } else {
                Err(CompileError::CodegenError("Expected function name".to_string()))
            }
        },
        _ => Err(CompileError::CodegenError("Expected function definition or declaration".to_string())),
    }
}

/// Extract return type from declaration specifiers
fn extract_return_type_from_decl_specifiers(decl_specifiers: &[Box<AstNode>]) -> Result<String, CompileError> {
    // Find the TypeSpecifier node in the declaration specifiers
    for spec in decl_specifiers {
        if let AstNode::TypeSpecifier(type_spec) = &**spec {
            return Ok(type_spec_to_string(type_spec));
        }
    }

    // If we couldn't find a type specifier, default to "int" (C default)
    Ok("int".to_string())
}

/// Convert TypeSpecifier to string
fn type_spec_to_string(type_spec: &crate::ast::TypeSpecifier) -> String {
    match type_spec {
        crate::ast::TypeSpecifier::Int => "int".to_string(),
        crate::ast::TypeSpecifier::Char => "char".to_string(),
        crate::ast::TypeSpecifier::Float => "float".to_string(),
        crate::ast::TypeSpecifier::Double => "double".to_string(),
        crate::ast::TypeSpecifier::Void => "void".to_string(),
        crate::ast::TypeSpecifier::Struct(name) => format!("struct {}", name),
        _ => "unknown".to_string(),
    }
}

/// Extract function parameters from a function declarator
pub fn get_function_params(declarator: &AstNode) -> Option<Vec<(String, String)>> {
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
                            let type_str = type_spec_to_string(type_spec);
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
