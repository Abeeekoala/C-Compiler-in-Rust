pub mod ast;
pub mod lexer;
pub mod parser;
pub mod codegen;
pub mod error;
pub mod debug;
use crate::error::CompileError;

// use ast::Node;

pub fn parse_source(source: &str) -> Result<ast::AstNode, CompileError> {
    // First, tokenize the input using the lexer
    let tokens = lexer::tokenize(source)?;
    println!("Tokens: {:?}", tokens);
    // Then, construct the parser with the tokens.
    let mut parser = parser::Parser::new(tokens);
    // Parse the translation unit (the root of the AST).
    let result = parser.parse_translation_unit()?;

    // If we're looking at a NodeList with exactly one element, return that element directly
    if let ast::AstNode::NodeList(nodes) = result {
        if nodes.len() == 1 {
            return Ok(*nodes.into_iter().next().unwrap());
        }
        return Ok(ast::AstNode::NodeList(nodes));
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;
    use crate::ast::{AstNode, TypeSpecifier};
    use crate::lexer::tokenize;

    #[test]
    fn test_parse_empty_function() {
        let source = "int main() {}";
        let lexer = lexer::tokenize(source).unwrap();
        let mut parser = Parser::new(lexer);

        let result = parser.parse_translation_unit();
        assert!(result.is_ok());

        if let Ok(ast) = &result {
            println!("Empty function AST: {:#?}", ast);
        }

        if let Ok(AstNode::NodeList(declarations)) = result {
            assert_eq!(declarations.len(), 1);
            // Verify it's a function definition
            match &*declarations[0] {
                AstNode::FunctionDefinition { decl_specifiers, declarator, .. } => {
                    // Check that the first declaration specifier is a TypeSpecifier::Int
                    if let Some(first_spec) = decl_specifiers.get(0) {
                        if let AstNode::TypeSpecifier(type_spec) = &**first_spec {
                            assert_eq!(*type_spec, TypeSpecifier::Int);
                        } else {
                            panic!("Expected TypeSpecifier in decl_specifiers");
                        }
                    } else {
                        panic!("Expected at least one declaration specifier");
                    }

                    // Check the function name
                    if let AstNode::Identifier(name) = &**declarator {
                        assert_eq!(name, "main");
                    } else {
                        panic!("Expected Identifier in declarator");
                    }
                },
                _ => panic!("Expected function definition"),
            }
        } else {
            panic!("Expected NodeList");
        }
    }

    #[test]
    fn test_parse_function_with_return() {
        let source = "int test() { return 42; }";
        let lexer = lexer::tokenize(source).unwrap();
        let mut parser = Parser::new(lexer);

        let result = parser.parse_translation_unit();
        assert!(result.is_ok());

        if let Ok(ast) = &result {
            println!("Function with return: {:#?}", ast);
        }
    }

    #[test]
    fn test_parse_void_function() {
        let source = "void empty() { return; }";
        let lexer = lexer::tokenize(source).unwrap();
        let mut parser = Parser::new(lexer);

        let result = parser.parse_translation_unit();
        assert!(result.is_ok());

        if let Ok(ast) = &result {
            println!("Void function AST: {:#?}", ast);
        }
    }

    #[test]
    fn test_missing_semicolon() {
        let source = "int broken() { return 42 }"; // Missing semicolon
        let lexer = lexer::tokenize(source).unwrap();
        let mut parser = Parser::new(lexer);

        let result = parser.parse_translation_unit();
        assert!(result.is_err());
    }

    #[test]
    fn test_mismatched_braces() {
        let source = "int broken() { return 42; "; // Missing closing brace
        let lexer = lexer::tokenize(source).unwrap();
        let mut parser = Parser::new(lexer);

        let result = parser.parse_translation_unit();
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_statement() {
        let source = "int test() { ; }"; // Empty statement
        let lexer = lexer::tokenize(source).unwrap();
        let mut parser = Parser::new(lexer);

        let result = parser.parse_translation_unit();
        assert!(result.is_ok());

        if let Ok(ast) = &result {
            println!("Empty statement AST: {:#?}", ast);
        }
    }

    #[test]
    fn test_parse_basic_expression() {
        let source = "1 + 2 * 3;";
        let tokens = tokenize(source).unwrap();
        let mut parser = Parser::new(tokens);

        let result = parser.parse_expression_statement();
        assert!(result.is_ok());

        if let Ok(ast) = &result {
            println!("Basic expression AST: {:#?}", ast);
        }
    }

    #[test]
    fn test_parse_function_definition() {
        let source = "int main() { return 0; }";
        let tokens = tokenize(source).unwrap();
        let mut parser = Parser::new(tokens);

        let result = parser.parse_function_definition();
        assert!(result.is_ok());

        if let Ok(ast) = &result {
            println!("Function definition AST: {:#?}", ast);
        }
    }

    #[test]
    fn test_parse_complex_expression() {
        let source = "a * (b + c) / (d - e);";
        let tokens = tokenize(source).unwrap();
        let mut parser = Parser::new(tokens);

        let result = parser.parse_expression_statement();
        assert!(result.is_ok());

        if let Ok(ast) = &result {
            println!("Complex expression AST: {:#?}", ast);
        }
    }

    #[test]
    fn test_simple_return() {
        let source = "int main() { return 42; }";
        let ast = parse_source(source).expect("Failed to parse source");
        let (assembly, context) = codegen::generate_code(&ast).expect("Failed to generate code");

        assert!(assembly.contains("li t0, 42"));
        assert!(assembly.contains("mv a0, t0"));
        println!("Generated assembly:\n{:?}", assembly);
    }

    #[test]
    fn test_arithmetic() {
        let source = "int main() { return 5 * 10 + 3; }";
        let ast = parse_source(source).expect("Failed to parse source");
        let (assembly, context) = codegen::generate_code(&ast).expect("Failed to generate code");

        assert!(assembly.contains("li "));
        assert!(assembly.contains("mul "));
        assert!(assembly.contains("add "));
        println!("Generated assembly:\n{:?}", assembly);
    }

    #[test]
    fn test_if_else() {
        let source = "int main() { if (1) return 10; else return 20; }";
        let ast = parse_source(source).expect("Failed to parse source");
        let (assembly, context) = codegen::generate_code(&ast).expect("Failed to generate code");

        assert!(assembly.contains("beqz"));
        assert!(assembly.contains("j"));
        println!("Generated assembly for if-else:\n{:?}", assembly);
    }

    #[test]
    fn test_if_no_else() {
        let source = "int main() { if (1) return 10; return 0; }";
        let ast = parse_source(source).expect("Failed to parse source");
        let (assembly, context) = codegen::generate_code(&ast).expect("Failed to generate code");

        assert!(assembly.contains("beqz"));
        println!("Generated assembly for if without else:\n{:?}", assembly);
    }

    #[test]
    fn test_parse_function() {
        let source = "int main() { return 0; }";
        let ast = parse_source(source).unwrap();

        // Check the structure
        match ast {
            AstNode::FunctionDefinition { decl_specifiers, declarator, .. } => {
                // Check declaration specifiers
                assert!(!decl_specifiers.is_empty());

                // Check if it's an int
                if let AstNode::TypeSpecifier(type_spec) = &*decl_specifiers[0] {
                    assert_eq!(*type_spec, TypeSpecifier::Int);
                } else {
                    panic!("Expected TypeSpecifier");
                }

                // Check function name
                if let AstNode::Identifier(name) = &*declarator {
                    assert_eq!(name, "main");
                } else {
                    panic!("Expected Identifier");
                }
            },
            _ => panic!("Expected FunctionDefinition"),
        }
    }
}

#[cfg(test)]
mod codegen_tests {
    use super::*;

    #[test]
    fn test_return_constant() {
        let source = "int main() { return 42; }";
        let ast = parse_source(source).unwrap();
        let (assembly, context) = codegen::generate_code(&ast).expect("Failed to generate code");

        // Verify the assembly contains the expected instruction
        assert!(assembly.contains("li t0, 42"));
        assert!(assembly.contains("mv a0, t0"));
        println!("Generated assembly:\n{}", assembly);
        println!("Context:\n{:?}", context);
    }

    #[test]
    fn test_arithmetic() {
        let source = "int main() { return 3 + 4 * 5; }";
        let ast = parse_source(source).unwrap();
        let (assembly, context) = codegen::generate_code(&ast).expect("Failed to generate code");

        // Basic verification that arithmetic operations are generated
        assert!(assembly.contains("li "));
        assert!(assembly.contains("mul "));
        assert!(assembly.contains("add "));
        println!("Generated assembly:\n{}", assembly);
        println!("Context:\n{:?}", context);
    }

    #[test]
    fn test_if_statement() {
        let source = "int main() { if (1) { return 42; } else { return 24; } }";
        let ast = parse_source(source).unwrap();
        let (assembly, context) = codegen::generate_code(&ast).expect("Failed to generate code");

        // Verify the assembly contains the if structure
        assert!(assembly.contains("beqz"));
        assert!(assembly.contains("j"));
        println!("Generated assembly for if-else:\n{}", assembly);
        println!("Context:\n{:?}", context);
    }

    #[test]
    fn test_if_without_else() {
        let source = "int main() { if (1) { return 42; } return 0; }";
        let ast = parse_source(source).unwrap();
        let (assembly, context) = codegen::generate_code(&ast).expect("Failed to generate code");

        // Verify the assembly contains the if structure
        assert!(assembly.contains("beqz"));
        println!("Generated assembly for if without else:\n{}", assembly);
        println!("Context:\n{:?}", context);
    }
}
