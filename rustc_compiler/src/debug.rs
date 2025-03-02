// src/debug.rs
use crate::ast::AstNode;
use crate::codegen::context::CodeGenContext;

pub fn print_ast(ast: &AstNode) -> String {
    format!("{:#?}", ast)
}

pub fn print_symbol_table(context: &CodeGenContext) -> String {
    let mut output = String::new();
    output.push_str("Symbol Table:\n");
    for (name, symbol) in &context.symbols {
        output.push_str(&format!("{}: {:?}\n", name, symbol));
    }
    output
}
