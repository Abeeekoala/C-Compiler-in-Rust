use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;
use rustc_compiler::{parse_source, ast::AstNode};
use rustc_compiler::codegen::generate_code;

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        return Err("Usage: rustc_compiler <input.c> <output.s>".to_string());
    }

    let input_file = &args[1];
    let output_file = &args[2];

    // Read input file
    let source = fs::read_to_string(input_file)
        .map_err(|e| format!("Error reading file: {}", e))?;

    // Parse source to AST
    let ast = parse_source(&source)?;

    // Generate assembly code
    let assembly = generate_code(&ast);

    // Write to output file
    let mut file = fs::File::create(output_file)
        .map_err(|e| format!("Error creating output file: {}", e))?;
    file.write_all(assembly.as_bytes())
        .map_err(|e| format!("Error writing to file: {}", e))?;

    println!("Successfully compiled {} to {}", input_file, output_file);
    Ok(())
}
