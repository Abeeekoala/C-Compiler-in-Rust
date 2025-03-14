// src/parser.rs

use crate::lexer::Token;
use crate::ast::{AstNode, TypeSpecifier, Context};
use std::iter::Peekable;
use std::vec::IntoIter;
use crate::error::CompileError;
/// The parser struct holds the list of tokens and provides methods to parse them.
pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

type ParseResult = Result<AstNode, CompileError>;

impl Parser {
    /// Create a new parser from a vector of tokens.
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            current: 0,
        }
    }

    /// Peek at the current token without consuming it
    fn peek(&self) -> Option<&Token> {
        if self.current < self.tokens.len() {
            Some(&self.tokens[self.current])
        } else {
            None
        }
    }

    /// Advance to the next token and return the previous token
    fn advance(&mut self) -> Option<Token> {
        if self.current < self.tokens.len() {
            let token = self.tokens[self.current].clone();
            self.current += 1;
            Some(token)
        } else {
            None
        }
    }

    /// Parse a translation unit (the root of the AST)
    pub fn parse_translation_unit(&mut self) -> ParseResult {
        let mut declarations = Vec::new();

        while self.current < self.tokens.len() {
            declarations.push(Box::new(self.parse_external_declaration()?));
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
    pub fn parse_function_definition(&mut self) -> ParseResult {
        // Parse declaration specifiers
        let type_spec = self.parse_declaration_specifiers()?;
        let mut decl_spec = Vec::new();
        decl_spec.push(Box::new(AstNode::TypeSpecifier(type_spec)));

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
    fn parse_declaration_specifiers(&mut self) -> Result<TypeSpecifier, CompileError> {
        if let Some(token) = self.advance() {
            match token {
                Token::IntKw => Ok(TypeSpecifier::Int),
                Token::VoidKw => Ok(TypeSpecifier::Void),
                Token::CharKw => Ok(TypeSpecifier::Char),
                // Add other type specifiers as needed
                _ => Err(CompileError::ParserError(format!("Expected type specifier, found {:?}", token))),
            }
        } else {
            Err(CompileError::ParserError("Unexpected end of file".to_string()))
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
                    return Err(CompileError::ParserError("Expected ')' after parameter list".to_string()));
                }
            }

            return Ok(id_node);
        }

        Err(CompileError::ParserError("Expected identifier in declarator".to_string()))
    }

    /// Parse a compound statement
    fn parse_compound_statement(&mut self) -> ParseResult {
        // Expect '{'
        self.expect_token(Token::LBrace)?;

        let mut statements = Vec::new();

        // Parse statements until '}'
        while !self.check_token(Token::RBrace) {
            statements.push(Box::new(self.parse_statement()?));
        }

        // Expect '}'
        self.expect_token(Token::RBrace)?;

        // Return block/compound statement
        Ok(AstNode::BlockStatement(statements))
    }

    /// Parse a statement
    pub fn parse_statement(&mut self) -> Result<AstNode, CompileError> {
        match self.peek() {
            // Empty statement (just a semicolon)
            Some(Token::Semicolon) => {
                self.advance(); // Consume the semicolon
                Ok(AstNode::ExpressionStatement(Box::new(AstNode::NodeList(Vec::new()))))
            },

            // Declaration statements
            Some(Token::IntKw) | Some(Token::CharKw) | Some(Token::VoidKw) => {
                self.parse_declaration()
            },

            // Compound statements
            Some(Token::LBrace) => self.parse_compound_statement(),

            // If statements
            Some(Token::IfKw) => self.parse_if_statement(),

            // Return statements
            Some(Token::ReturnKw) => self.parse_return_statement(),

            // While statements
            Some(Token::WhileKw) => self.parse_while_statement(),

            // For statements
            Some(Token::ForKw) => self.parse_for_statement(),

            // Expression statements (e.g., function calls, assignments)
            _ => {
                let expr = self.parse_expression()?;
                // Expect ';'
                if let Some(Token::Semicolon) = self.peek() {
                    self.advance();
                    Ok(AstNode::ExpressionStatement(Box::new(expr)))
                } else {
                    Err(CompileError::ParserError("Expected ';' after statement".to_string()))
                }
            }
        }
    }

    fn parse_for_statement(&mut self) -> ParseResult {
        self.advance(); // Consume 'for'
        self.expect_token(Token::LParen)?;

        // Parse initialization (can be empty)
        let init = if self.check_token(Token::Semicolon) {
            AstNode::Empty
        } else {
            self.parse_expression_statement()?
        };

        // Parse condition (can be empty, default to true)
        let condition = if self.check_token(Token::Semicolon) {
            AstNode::IntConstant(1) // Default true
        } else {
            let expr = self.parse_expression()?;
            self.expect_token(Token::Semicolon)?;
            expr
        };

        // Parse increment (can be empty)
        let increment = if self.check_token(Token::RParen) {
            AstNode::Empty
        } else {
            let expr = self.parse_expression()?;
            self.expect_token(Token::RParen)?;
            expr
        };

        // Parse body
        let body = self.parse_statement()?;

        Ok(AstNode::ForLoop {
            init: Box::new(init),
            condition: Box::new(condition),
            increment: Box::new(increment),
            body: Box::new(body),
        })
    }

    fn parse_while_statement(&mut self) -> Result<AstNode, CompileError> {
        self.advance(); // Consume the 'while' token

        // Expect '('
        self.expect_token(Token::LParen)?;

        // Parse the loop condition
        let condition = self.parse_expression()?;

        // Expect ')'
        self.expect_token(Token::RParen)?;

        // Parse the loop body (this could be a single statement or a block)
        let body = self.parse_statement()?;

        // Return a WhileStatement AST node
        Ok(AstNode::WhileStatement {
            condition: Box::new(condition),
            body: Box::new(body),
        })
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

        Err(CompileError::ParserError("Expected ';' after return statement".to_string()))
    }

    /// Parse an expression statement
    pub fn parse_expression_statement(&mut self) -> ParseResult {
        let expr = self.parse_expression()?;

        // Expect ';'
        if let Some(Token::Semicolon) = self.peek() {
            self.advance(); // Consume ';'
            return Ok(expr);
        }

        Err(CompileError::ParserError("Expected ';' after expression".to_string()))
    }

    /// Parse an expression
    fn parse_expression(&mut self) -> ParseResult {
        // Start with lowest precedence: comma expressions
        let mut expr = self.parse_assignment_expression()?;

        // Handle comma-separated expressions
        while let Some(Token::Comma) = self.peek() {
            self.advance(); // Consume ','
            let right = self.parse_assignment_expression()?;
            expr = AstNode::BinaryOperation {
                op: ",".to_string(),
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    /// Parse assignment expressions
    fn parse_assignment_expression(&mut self) -> ParseResult {
        let lhs = self.parse_conditional_expression()?;
        if let Some(Token::Assign) = self.peek() {
            self.advance();
            let rhs = self.parse_assignment_expression()?;
            return Ok(AstNode::Assignment {
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            });
        }
        Ok(lhs)
    }

    /// Parse conditional expression (ternary operator)
    fn parse_conditional_expression(&mut self) -> ParseResult {
        // Parse the condition part
        let condition = self.parse_logical_or_expression()?;

        // Check for the '?' operator
        if let Some(Token::Question) = self.peek() {
            self.advance(); // Consume '?'

            // Parse the true expression
            let true_expr = self.parse_expression()?;

            // Expect ':'
            self.expect_token(Token::Colon)?;

            // Parse the false expression
            let false_expr = self.parse_conditional_expression()?;

            return Ok(AstNode::TernaryOperation {
                condition: Box::new(condition),
                true_expr: Box::new(true_expr),
                false_expr: Box::new(false_expr),
            });
        }

        // If no '?', return the logical OR expression
        Ok(condition)
    }

    /// Parse logical OR expression
    fn parse_logical_or_expression(&mut self) -> ParseResult {
        let mut expr = self.parse_logical_and_expression()?;

        while let Some(Token::LogicOr) = self.peek() {
            self.advance(); // Consume '||'
            let right = self.parse_logical_and_expression()?;
            expr = AstNode::BinaryOperation {
                op: "||".to_string(),
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    /// Parse logical AND expression
    fn parse_logical_and_expression(&mut self) -> ParseResult {
        let mut expr = self.parse_inclusive_or_expression()?;

        while let Some(Token::LogicAnd) = self.peek() {
            self.advance(); // Consume '&&'
            let right = self.parse_inclusive_or_expression()?;
            expr = AstNode::BinaryOperation {
                op: "&&".to_string(),
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    /// Parse inclusive OR expression
    fn parse_inclusive_or_expression(&mut self) -> ParseResult {
        let mut expr = self.parse_exclusive_or_expression()?;

        while let Some(Token::BitOr) = self.peek() {
            self.advance(); // Consume '|'
            let right = self.parse_exclusive_or_expression()?;
            expr = AstNode::BinaryOperation {
                op: "|".to_string(),
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    /// Parse exclusive OR expression
    fn parse_exclusive_or_expression(&mut self) -> ParseResult {
        let mut expr = self.parse_and_expression()?;

        while let Some(Token::BitXor) = self.peek() {
            self.advance(); // Consume '^'
            let right = self.parse_and_expression()?;
            expr = AstNode::BinaryOperation {
                op: "^".to_string(),
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    /// Parse AND expression
    fn parse_and_expression(&mut self) -> ParseResult {
        let mut expr = self.parse_equality_expression()?;

        while let Some(Token::BitAnd) = self.peek() {
            self.advance(); // Consume '&'
            let right = self.parse_equality_expression()?;
            expr = AstNode::BinaryOperation {
                op: "&".to_string(),
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    /// Parse equality expression
    fn parse_equality_expression(&mut self) -> ParseResult {
        let mut expr = self.parse_relational_expression()?;

        while let Some(token) = self.peek() {
            let op = match token {
                Token::Equal => "==",
                Token::NotEqual => "!=",
                _ => break,
            };

            self.advance(); // Consume operator
            let right = self.parse_relational_expression()?;
            expr = AstNode::BinaryOperation {
                op: op.to_string(),
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    /// Parse relational expression
    fn parse_relational_expression(&mut self) -> ParseResult {
        let mut expr = self.parse_shift_expression()?;

        while let Some(token) = self.peek() {
            let op = match token {
                Token::Less => "<",
                Token::Greater => ">",
                Token::LessEqual => "<=",
                Token::GreaterEqual => ">=",
                _ => break,
            };

            self.advance(); // Consume operator
            let right = self.parse_shift_expression()?;
            expr = AstNode::BinaryOperation {
                op: op.to_string(),
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    /// Parse shift expression
    fn parse_shift_expression(&mut self) -> ParseResult {
        let mut expr = self.parse_additive_expression()?;

        while let Some(token) = self.peek() {
            let op = match token {
                Token::ShiftLeft => "<<",
                Token::ShiftRight => ">>",
                _ => break,
            };

            self.advance(); // Consume operator
            let right = self.parse_additive_expression()?;
            expr = AstNode::BinaryOperation {
                op: op.to_string(),
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    /// Parse additive expression
    fn parse_additive_expression(&mut self) -> ParseResult {
        let mut expr = self.parse_multiplicative_expression()?;

        while let Some(token) = self.peek() {
            let op = match token {
                Token::Add => "+",
                Token::Sub => "-",
                _ => break,
            };

            self.advance(); // Consume operator
            let right = self.parse_multiplicative_expression()?;
            expr = AstNode::BinaryOperation {
                op: op.to_string(),
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    /// Parse multiplicative expression
    fn parse_multiplicative_expression(&mut self) -> ParseResult {
        let mut expr = self.parse_cast_expression()?;

        while let Some(token) = self.peek() {
            let op = match token {
                Token::Mul => "*",
                Token::Div => "/",
                Token::Mod => "%",
                _ => break,
            };

            self.advance(); // Consume operator
            let right = self.parse_cast_expression()?;
            expr = AstNode::BinaryOperation {
                op: op.to_string(),
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    /// Parse cast expression (type casting)
    fn parse_cast_expression(&mut self) -> ParseResult {
        // For now, we'll skip cast expressions and just go to unary
        // Later: Add support for type casting
        self.parse_unary_expression()
    }

    /// Parse unary expression
    fn parse_unary_expression(&mut self) -> ParseResult {
        match self.peek() {
            Some(Token::Add) => {
                self.advance(); // Consume '+'
                let expr = self.parse_cast_expression()?;
                Ok(AstNode::UnaryOperation {
                    op: "+".to_string(),
                    operand: Box::new(expr),
                })
            },
            Some(Token::Sub) => {
                self.advance(); // Consume '-'
                let expr = self.parse_cast_expression()?;
                Ok(AstNode::UnaryOperation {
                    op: "-".to_string(),
                    operand: Box::new(expr),
                })
            },
            Some(Token::Tilde) => {
                self.advance(); // Consume '~'
                let expr = self.parse_cast_expression()?;
                Ok(AstNode::UnaryOperation {
                    op: "~".to_string(),
                    operand: Box::new(expr),
                })
            },
            Some(Token::Not) => {
                self.advance(); // Consume '!'
                let expr = self.parse_cast_expression()?;
                Ok(AstNode::UnaryOperation {
                    op: "!".to_string(),
                    operand: Box::new(expr),
                })
            },
            Some(Token::PlusPlus) => {
                self.advance(); // Consume '++'
                let expr = self.parse_unary_expression()?;
                Ok(AstNode::UnaryOperation {
                    op: "++".to_string(),
                    operand: Box::new(expr),
                })
            },
            Some(Token::MinusMinus) => {
                self.advance(); // Consume '--'
                let expr = self.parse_unary_expression()?;
                Ok(AstNode::UnaryOperation {
                    op: "--".to_string(),
                    operand: Box::new(expr),
                })
            },
            Some(Token::Mul) => { // Pointer dereference
                self.advance(); // Consume '*'
                let expr = self.parse_cast_expression()?;
                Ok(AstNode::UnaryOperation {
                    op: "*".to_string(),
                    operand: Box::new(expr),
                })
            },
            Some(Token::BitAnd) => { // Address-of operator
                self.advance(); // Consume '&'
                let expr = self.parse_cast_expression()?;
                Ok(AstNode::UnaryOperation {
                    op: "&".to_string(),
                    operand: Box::new(expr),
                })
            },
            _ => self.parse_postfix_expression(),
        }
    }

    /// Parse postfix expression
    fn parse_postfix_expression(&mut self) -> ParseResult {
        let mut expr = self.parse_primary_expression()?;

        loop {
            match self.peek() {
                Some(Token::LBracket) => {
                    self.advance(); // Consume '['
                    let index = self.parse_expression()?;
                    self.expect_token(Token::RBracket)?; // Expect ']'
                    expr = AstNode::ArraySubscript {
                        array: Box::new(expr),
                        index: Box::new(index),
                    };
                },
                Some(Token::LParen) => {
                    self.advance(); // Consume '('

                    // Parse function arguments
                    let mut args = Vec::new();
                    if let Some(Token::RParen) = self.peek() {
                        // Empty argument list
                    } else {
                        // Parse at least one argument
                        args.push(Box::new(self.parse_assignment_expression()?));

                        // Parse additional arguments
                        while let Some(Token::Comma) = self.peek() {
                            self.advance(); // Consume ','
                            args.push(Box::new(self.parse_assignment_expression()?));
                        }
                    }

                    self.expect_token(Token::RParen)?; // Expect ')'
                    expr = AstNode::FunctionCall {
                        function: Box::new(expr),
                        args,
                    };
                },
                Some(Token::PlusPlus) => {
                    self.advance(); // Consume '++'
                    expr = AstNode::UnaryOperation {
                        op: "post++".to_string(),
                        operand: Box::new(expr),
                    };
                },
                Some(Token::MinusMinus) => {
                    self.advance(); // Consume '--'
                    expr = AstNode::UnaryOperation {
                        op: "post--".to_string(),
                        operand: Box::new(expr),
                    };
                },
                // Later: Add support for member access (. and ->)
                _ => break,
            }
        }

        Ok(expr)
    }

    /// Parse primary expression (literals, identifiers, and parenthesized expressions)
    fn parse_primary_expression(&mut self) -> ParseResult {
        match self.peek() {
            Some(Token::IntLiteralDec((value, _))) => {
                let val = *value;
                self.advance();
                Ok(AstNode::IntConstant(val as i32))
            },
            Some(Token::Identifier(name)) => {
                let id = name.clone();
                self.advance();
                Ok(AstNode::Identifier(id))
            },
            Some(Token::LParen) => {
                self.advance(); // Consume '('
                let expr = self.parse_expression()?;
                self.expect_token(Token::RParen)?; // Expect ')'
                Ok(expr)
            },
            Some(token) => Err(CompileError::ParserError(format!("Expected primary expression, found {:?}", token))),
            None => Err(CompileError::ParserError("Unexpected end of file".to_string())),
        }
    }

    /// Parse a declaration
    pub fn parse_declaration(&mut self) -> Result<AstNode, CompileError> {
        // Parse the type specifier (int, char, etc.)
        let type_specifier = self.parse_type_specifier()?;

        // Parse the variable identifier
        let identifier = self.parse_identifier()?;
        let declarator = Box::new(AstNode::Identifier(identifier));

        // Check for initializer
        let initializer = match self.peek() {
            Some(token) if *token == Token::Assign => {
                self.expect_token(Token::Assign)?; // Consume the '=' token
                // Parse the initializer expression
                Some(Box::new(self.parse_expression()?))
            },
            _ => None
        };

        // Expect semicolon at the end
        self.expect_token(Token::Semicolon)?;

        // Return Declaration node
        Ok(AstNode::Declaration {
            type_spec: type_specifier,
            declarator,
            initializer,
        })
    }

    /// Parse an if statement
    fn parse_if_statement(&mut self) -> ParseResult {
        self.advance(); // Consume 'if'

        // Expect '('
        self.expect_token(Token::LParen)?;

        // Parse condition
        let condition = self.parse_expression()?;

        // Expect ')'
        self.expect_token(Token::RParen)?;

        // Parse then-statement
        let then_stmt = self.parse_statement()?;

        // Check for 'else'
        let else_stmt = if let Some(Token::ElseKw) = self.peek() {
            self.advance(); // Consume 'else'
            Some(Box::new(self.parse_statement()?))
        } else {
            None
        };

        Ok(AstNode::IfStatement {
            condition: Box::new(condition),
            then_stmt: Box::new(then_stmt),
            else_stmt,
        })
    }

    fn expect_token(&mut self, expected: Token) -> Result<(), CompileError> {
        if let Some(token) = self.advance() {
            if token == expected {
                Ok(())
            } else {
                Err(CompileError::ParserError(format!("Expected {:?}, found {:?}", expected, token)))
            }
        } else {
            Err(CompileError::ParserError("Unexpected end of file".to_string()))
        }
    }

    fn check_token(&mut self, expected: Token) -> bool {
        if let Some(token) = self.peek() {
            token == &expected
        } else {
            false
        }
    }

    /// Parses an identifier
    pub fn parse_identifier(&mut self) -> Result<String, CompileError> {
        // Clone token first to avoid borrowing issues
        let token_clone = self.peek()
            .ok_or_else(|| CompileError::ParserError("Unexpected end of file".to_string()))?
            .clone();

        match token_clone {
            Token::Identifier(name) => {
                self.advance(); // Now we can advance
                Ok(name)
            },
            _ => Err(CompileError::ParserError(format!("Expected identifier, found {:?}", token_clone)))
        }
    }

    /// Parses a type specifier (int, char, void, etc.)
    pub fn parse_type_specifier(&mut self) -> Result<TypeSpecifier, CompileError> {
        let token = self.peek().ok_or_else(|| CompileError::ParserError("Unexpected end of file".to_string()))?;

        match token {
            Token::IntKw => {
                self.advance();
                Ok(TypeSpecifier::Int)
            },
            Token::CharKw => {
                self.advance();
                Ok(TypeSpecifier::Char)
            },
            Token::VoidKw => {
                self.advance();
                Ok(TypeSpecifier::Void)
            },
            _ => Err(CompileError::ParserError("Expected type specifier".to_string()))
        }
    }
}
