use std::env;
use std::fs;
use std::io::Write;
use rustc_compiler::{parse_source, codegen::generate_code, error::CompileError, debug::{print_ast, print_symbol_table}};

fn main() -> Result<(), CompileError> {
    let args: Vec<String> = env::args().collect();

    // Get the input and output files based on command-line arguments
    let (input_file, output_file, debug_mode) = match args.len() {
        3 => {
            (args[1].clone(), args[2].clone(), false)
        },
        4 => {
            if args[3] == "--debug" {
                (args[1].clone(), args[2].clone(), true)
            } else {
                return Err(CompileError::IOError("Unexpected argument format".to_string()));
            }
        },
        _ => {
            return Err(CompileError::IOError(
                "Usage: rustc_compiler <input.c> <output.s> [--debug]".to_string()
            ));
        }
    };

    // Read input file
    let source = fs::read_to_string(&input_file)
        .map_err(|e| CompileError::IOError(format!("Error reading file: {}", e)))?;

    // Parse source to AST
    let ast = parse_source(&source)?;

    // Always print AST for debugging during development
    eprintln!("--- AST for {} ---", input_file);
    eprintln!("{}", print_ast(&ast));

    // Generate assembly code
    let (assembly, context) = generate_code(&ast)?;

    if debug_mode {
        eprintln!("--- Debug: Symbol Table ---");
        eprintln!("{}", print_symbol_table(&context));
    }

    // Write to output file
    let mut file = fs::File::create(&output_file)
        .map_err(|e| CompileError::IOError(format!("Error creating output file: {}", e)))?;
    file.write_all(assembly.as_bytes())
        .map_err(|e| CompileError::IOError(format!("Error writing to file: {}", e)))?;

    println!("Successfully compiled {} to {}", input_file, output_file);
    Ok(())
}
