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

    #[test]
    fn test_empty_function() {
        let source = "int main() { }";
        let result = parse_source(source);

        assert!(result.is_ok(), "Parse error: {:?}", result.err());
        let ast = result.unwrap();

        if let ast::AstNode::FunctionDefinition { decl_specifiers, declarator, compound_statement } = ast {
            assert_eq!(decl_specifiers, ast::TypeSpecifier::Int);

            if let ast::AstNode::Identifier(name) = *declarator {
                assert_eq!(name, "main");
            } else {
                panic!("Expected Identifier in declarator");
            }

            if let ast::AstNode::NodeList(statements) = *compound_statement {
                assert!(statements.is_empty(), "Expected empty function body");
            } else {
                panic!("Expected NodeList for compound_statement");
            }
        } else {
            panic!("Expected FunctionDefinition, got: {}", ast.print());
        }
    }

    #[test]
    fn test_function_with_return() {
        let source = "int main() { return 42; }";
        let result = parse_source(source);

        assert!(result.is_ok(), "Parse error: {:?}", result.err());
        let ast = result.unwrap();

        if let ast::AstNode::FunctionDefinition { decl_specifiers, declarator, compound_statement } = ast {
            assert_eq!(decl_specifiers, ast::TypeSpecifier::Int);

            if let ast::AstNode::Identifier(name) = *declarator {
                assert_eq!(name, "main");
            } else {
                panic!("Expected Identifier in declarator");
            }

            if let ast::AstNode::NodeList(statements) = *compound_statement {
                assert_eq!(statements.len(), 1, "Expected one statement in function body");

                if let ast::AstNode::ReturnStatement(Some(expr)) = &statements[0] {
                    if let ast::AstNode::IntConstant(value) = **expr {
                        assert_eq!(value, 42);
                    } else {
                        panic!("Expected IntConstant in return statement");
                    }
                } else {
                    panic!("Expected ReturnStatement with expression");
                }
            } else {
                panic!("Expected NodeList for compound_statement");
            }
        } else {
            panic!("Expected FunctionDefinition, got: {}", ast.print());
        }
    }
}
