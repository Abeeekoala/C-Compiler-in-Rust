// src/parser.rs

use crate::lexer::Token;
use crate::ast::{AstNode, TypeSpecifier, Context};
use std::iter::Peekable;
use std::vec::IntoIter;

/// The parser struct holds the list of tokens and provides methods to parse them.
pub struct Parser {
    tokens: Peekable<IntoIter<Token>>,
}

type ParseResult = Result<AstNode, String>;

impl Parser {
    /// Create a new parser from a vector of tokens.
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens: tokens.into_iter().peekable(),
        }
    }

    /// Consume the current token if it matches the expected token
    fn consume(&mut self, expected: &Token) -> Result<(), String> {
        if let Some(token) = self.tokens.peek() {
            if token == expected {
                self.tokens.next();
                return Ok(());
            }
            return Err(format!("Expected {:?}, found {:?}", expected, token));
        }
        Err("Unexpected end of input".to_string())
    }

    /// Peek at the current token without consuming it
    fn peek(&mut self) -> Option<&Token> {
        self.tokens.peek()
    }

    /// Advance to the next token and return the previous token
    fn advance(&mut self) -> Option<Token> {
        self.tokens.next()
    }

    /// Parse a translation unit (the root of the AST)
    pub fn parse_translation_unit(&mut self) -> ParseResult {
        let mut declarations = Vec::new();

        while self.peek().is_some() {
            declarations.push(self.parse_external_declaration()?);
        }

        Ok(AstNode::NodeList(declarations))
    }

    /// Parse an external declaration (function or global variable)
    fn parse_external_declaration(&mut self) -> ParseResult {
        // Try to parse a function definition first
        if let Ok(func) = self.parse_function_definition() {
            return Ok(func);
        }

        // Otherwise, try to parse a declaration
        self.parse_declaration()
    }

    /// Parse a function definition
    fn parse_function_definition(&mut self) -> ParseResult {
        // Parse declaration specifiers
        let decl_spec = self.parse_declaration_specifiers()?;

        // Parse declarator
        let declarator = self.parse_declarator()?;

        // Parse compound statement (function body)
        let body = self.parse_compound_statement()?;

        Ok(AstNode::FunctionDefinition {
            decl_specifiers: decl_spec,
            declarator: Box::new(declarator),
            compound_statement: Box::new(body),
        })
    }

    /// Parse declaration specifiers
    fn parse_declaration_specifiers(&mut self) -> Result<TypeSpecifier, String> {
        match self.peek() {
            Some(Token::IntKw) => {
                self.advance();
                Ok(TypeSpecifier::Int)
            },
            Some(Token::VoidKw) => {
                self.advance();
                Ok(TypeSpecifier::Void)
            },
            Some(token) => Err(format!("Expected type specifier, found {:?}", token)),
            None => Err("Unexpected end of input".to_string()),
        }
    }

    /// Parse a declarator
    fn parse_declarator(&mut self) -> ParseResult {
        // Check for identifier
        if let Some(Token::Identifier(name)) = self.advance() {
            let id_node = AstNode::Identifier(name.clone());

            // Check if this is a function declarator (has parentheses)
            if let Some(Token::LParen) = self.peek() {
                self.advance(); // Consume '('

                // Parse parameter list (for simplicity, just expect ')')
                if let Some(Token::RParen) = self.peek() {
                    self.advance(); // Consume ')'
                    return Ok(id_node);
                } else {
                    return Err("Expected ')' after parameter list".to_string());
                }
            }

            return Ok(id_node);
        }

        Err("Expected identifier in declarator".to_string())
    }

    /// Parse a compound statement
    fn parse_compound_statement(&mut self) -> ParseResult {
        // Expect '{'
        if let Some(Token::LBrace) = self.peek() {
            self.advance(); // Consume '{'

            let mut statements = Vec::new();

            // Parse statements until '}'
            while let Some(token) = self.peek() {
                if *token == Token::RBrace {
                    break;
                }

                statements.push(self.parse_statement()?);
            }

            // Expect '}'
            if let Some(Token::RBrace) = self.peek() {
                self.advance(); // Consume '}'
                return Ok(AstNode::NodeList(statements));
            } else {
                return Err("Expected '}' to close compound statement".to_string());
            }
        }

        Err("Expected '{' to start compound statement".to_string())
    }

    /// Parse a statement
    fn parse_statement(&mut self) -> ParseResult {
        match self.peek() {
            Some(Token::ReturnKw) => self.parse_return_statement(),
            Some(Token::LBrace) => self.parse_compound_statement(),
            Some(Token::Semicolon) => {
                self.advance(); // Consume ';'
                Ok(AstNode::NodeList(Vec::new())) // Empty statement
            },
            _ => self.parse_expression_statement(),
        }
    }

    /// Parse a return statement
    fn parse_return_statement(&mut self) -> ParseResult {
        self.advance(); // Consume 'return'

        let expr = if let Some(Token::Semicolon) = self.peek() {
            None // return; (no expression)
        } else {
            Some(Box::new(self.parse_expression()?))
        };

        // Expect ';'
        if let Some(Token::Semicolon) = self.peek() {
            self.advance(); // Consume ';'
            return Ok(AstNode::ReturnStatement(expr));
        }

        Err("Expected ';' after return statement".to_string())
    }

    /// Parse an expression statement
    fn parse_expression_statement(&mut self) -> ParseResult {
        let expr = self.parse_expression()?;

        // Expect ';'
        if let Some(Token::Semicolon) = self.peek() {
            self.advance(); // Consume ';'
            return Ok(expr);
        }

        Err("Expected ';' after expression".to_string())
    }

    /// Parse an expression
    fn parse_expression(&mut self) -> ParseResult {
        // For simplicity, just handle integer literals for now
        if let Some(Token::IntLiteralDec((value, _))) = self.advance() {
            return Ok(AstNode::IntConstant(value as i32));
        }

        Err("Expected expression".to_string())
    }

    /// Parse a declaration
    fn parse_declaration(&mut self) -> ParseResult {
        Err("Declaration parsing not yet implemented".to_string())
    }
}
