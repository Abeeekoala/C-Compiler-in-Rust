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

    // Print symbols that represent variables
    output.push_str("Variables:\n");
    for (name, symbol) in &context.symbols {
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
