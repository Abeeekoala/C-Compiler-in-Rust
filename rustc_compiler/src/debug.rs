// src/debug.rs
use crate::ast::AstNode;
use crate::codegen::context::CodeGenContext;
use std::fmt;

pub fn print_ast(ast: &AstNode) -> String {
    format!("{:#?}", ast)
}

pub fn print_symbol_table(context: &CodeGenContext) -> String {
    let mut output = String::new();

    output.push_str("Symbol Table:\n");
    output.push_str("=============\n");

    // Print active scopes
    output.push_str("Active Scopes:\n");
    for (i, scope) in context.symbols.iter().enumerate() {
        output.push_str(&format!("Scope {}:\n", i));
        for (name, symbol) in scope {
            match &symbol.location {
                crate::codegen::context::StorageLocation::Stack(offset) => {
                    output.push_str(&format!("  {} ({}): stack offset={}\n",
                        name, symbol.type_info, offset));
                },
                crate::codegen::context::StorageLocation::Register(reg) => {
                    output.push_str(&format!("  {} ({}): register={}\n",
                        name, symbol.type_info, reg));
                },
                crate::codegen::context::StorageLocation::Global(label) => {
                    output.push_str(&format!("  {} ({}): global label={}\n",
                        name, symbol.type_info, label));
                }
            }
        }
        output.push_str("\n");
    }

    // Print popped scopes
    output.push_str("Popped Scopes:\n");
    output.push_str("==============\n");
    for (i, scope) in context.scope_history.iter().enumerate() {
        output.push_str(&format!("Popped Scope {}:\n", i + 1));
        for (name, symbol) in scope {
            match &symbol.location {
                crate::codegen::context::StorageLocation::Stack(offset) => {
                    output.push_str(&format!("  {} ({}): stack offset={}\n",
                        name, symbol.type_info, offset));
                },
                crate::codegen::context::StorageLocation::Register(reg) => {
                    output.push_str(&format!("  {} ({}): register={}\n",
                        name, symbol.type_info, reg));
                },
                crate::codegen::context::StorageLocation::Global(label) => {
                    output.push_str(&format!("  {} ({}): global label={}\n",
                        name, symbol.type_info, label));
                }
            }
        }
        output.push_str("\n");
    }

    // Print current function if available
    if let Some(func_name) = &context.current_function {
        output.push_str("\nCurrent Function:\n");
        output.push_str(&format!("  {}\n", func_name));
    }

    // Print stack offset information
    output.push_str(&format!("\nStack Information:\n"));
    output.push_str(&format!("  Current stack offset: {}\n", context.stack_offset));

    output
}
