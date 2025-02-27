pub mod ast;
pub mod lexer;
pub mod parser;

use ast::Node;

pub fn parse_source(source: &str) -> Result<ast::AstNode, String> {
    // First, tokenize the input using the lexer
    let tokens = lexer::tokenize(source)?;
    println!("Tokens: {:?}", tokens);
    // Then, construct the parser with the tokens.
    let mut parser = parser::Parser::new(tokens);
    // Parse the translation unit (the root of the AST).
    let result = parser.parse_translation_unit()?;

    // For single function tests, unwrap the NodeList if it contains only one item
    if let ast::AstNode::NodeList(nodes) = result {
        if nodes.len() == 1 {
            return Ok(nodes.into_iter().next().unwrap());
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

    #[test]
    fn test_parse_empty_function() {
        let source = "int main() {}";
        let lexer = lexer::tokenize(source).unwrap();
        let mut parser = Parser::new(lexer);

        let result = parser.parse_translation_unit();
        assert!(result.is_ok());

        if let Ok(AstNode::NodeList(declarations)) = result {
            assert_eq!(declarations.len(), 1);
            // Verify it's a function definition
            match &declarations[0] {
                AstNode::FunctionDefinition { decl_specifiers, declarator, .. } => {
                    assert_eq!(*decl_specifiers, TypeSpecifier::Int);

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
    }

    #[test]
    fn test_parse_void_function() {
        let source = "void empty() { return; }";
        let lexer = lexer::tokenize(source).unwrap();
        let mut parser = Parser::new(lexer);

        let result = parser.parse_translation_unit();
        assert!(result.is_ok());
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
    }
}
