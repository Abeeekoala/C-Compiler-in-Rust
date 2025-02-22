use crate::lexer::Token;
use std::collections::{HashSet, VecDeque};
use std::iter::Peekable;

/// Abstract Syntax Tree (AST) definitions for expressions.
#[derive(Debug, PartialEq, Clone)]
pub enum Expr {
    /// An identifier (e.g., variable name).
    Identifier(String),
    /// A type identifier (recognized via typedef).
    TypeIdentifier(String),
    /// Integer constant (e.g., 42).
    IntConstant(i32),
    /// Floating-point constant (e.g., 3.14).
    FloatConstant(f64),
    /// String literal (e.g., "hello").
    StringLiteral(String),
    /// Unary expression (e.g., -x, !y).
    Unary { op: String, operand: Box<Expr> },
    /// Binary expression (e.g., a + b).
    Binary {
        left: Box<Expr>,
        op: String,
        right: Box<Expr>,
    },
    /// Assignment expression (e.g., x = 5).
    Assignment {
        left: Box<Expr>,
        op: String,
        right: Box<Expr>,
    },
    /// Function call (e.g., foo(1, 2)).
    FunctionCall { function: Box<Expr>, arguments: Vec<Expr> },
}

/// AST definitions for statements.
#[derive(Debug, PartialEq, Clone)]
pub enum Stmt {
    /// Expression statement (e.g., x = 5;).
    Expression(Option<Expr>),
    /// Compound statement (e.g., { int x; x = 5; }).
    Compound { declarations: Vec<Declaration>, statements: Vec<Stmt> },
    /// If statement (e.g., if (x > 0) y = 1;).
    If { condition: Expr, then_stmt: Box<Stmt>, else_stmt: Option<Box<Stmt>> },
    /// While statement (e.g., while (x < 10) x++;).
    While { condition: Expr, body: Box<Stmt> },
    /// Return statement (e.g., return 0;).
    Return(Option<Expr>),
}

/// Represents a declarator (e.g., x, *p).
#[derive(Debug, PartialEq, Clone)]
pub enum Declarator {
    Identifier(String),
    Pointer(Box<Declarator>),
}

/// Represents an initialized declarator (e.g., x = 5).
#[derive(Debug, PartialEq, Clone)]
pub struct InitDeclarator {
    declarator: Declarator,
    initializer: Option<Expr>,
}

/// Represents a declaration (e.g., int x; or typedef int A;).
#[derive(Debug, PartialEq, Clone)]
pub struct Declaration {
    is_typedef: bool,
    type_specifier: String,
    init_declarators: Vec<InitDeclarator>,
}

/// Parser struct that processes tokens and builds the AST.
pub struct Parser<I>
where
    I: Iterator<Item = Token>,
{
    tokens: Peekable<I>,
    /// Stack of scopes for type names (each scope is a set of typedef'd identifiers).
    type_table: Vec<HashSet<String>>,
}

impl<I> Parser<I>
where
    I: Iterator<Item = Token>,
{
    /// Creates a new parser with a token iterator.
    pub fn new(tokens: I) -> Self {
        let mut type_table = Vec::new();
        type_table.push(HashSet::new()); // Global scope
        Parser {
            tokens: tokens.peekable(),
            type_table,
        }
    }

    /// Parses a full program (translation unit), returning a list of statements.
    pub fn parse_program(&mut self) -> Vec<Stmt> {
        let mut stmts = Vec::new();
        while self.tokens.peek().is_some() {
            if self.is_declaration_start() {
                let decl = self.parse_declaration();
                if decl.init_declarators.is_empty() {
                    // Handle empty declarations (e.g., int;)
                    continue;
                }
                stmts.push(Stmt::Compound {
                    declarations: vec![decl],
                    statements: Vec::new(),
                });
            } else {
                stmts.push(self.parse_statement());
            }
        }
        stmts
    }

    // --- Expression Parsing ---

    /// Parses an expression (top-level, starts with assignment).
    fn parse_expression(&mut self) -> Expr {
        self.parse_assignment_expression()
    }

    /// Parses assignment expressions (right-associative).
    fn parse_assignment_expression(&mut self) -> Expr {
        let expr = self.parse_additive_expression(); // Simplified to additive for now
        if let Some(token) = self.tokens.peek() {
            if self.is_assignment_operator(token) {
                let op = self.tokens.next().unwrap().to_string();
                let right = self.parse_assignment_expression();
                return Expr::Assignment {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                };
            }
        }
        expr
    }

    /// Parses additive expressions (+, -).
    fn parse_additive_expression(&mut self) -> Expr {
        let mut expr = self.parse_multiplicative_expression();
        while let Some(token) = self.tokens.peek() {
            if matches!(token, Token::Plus | Token::Minus) {
                let op = self.tokens.next().unwrap().to_string();
                let right = self.parse_multiplicative_expression();
                expr = Expr::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }
        expr
    }

    /// Parses multiplicative expressions (*, /, %).
    fn parse_multiplicative_expression(&mut self) -> Expr {
        let mut expr = self.parse_unary_expression();
        while let Some(token) = self.tokens.peek() {
            if matches!(token, Token::Star | Token::Slash | Token::Percent) {
                let op = self.tokens.next().unwrap().to_string();
                let right = self.parse_unary_expression();
                expr = Expr::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }
        expr
    }

    /// Parses unary expressions (e.g., -x, *x).
    fn parse_unary_expression(&mut self) -> Expr {
        if let Some(token) = self.tokens.peek() {
            match token {
                Token::Minus | Token::Star | Token::And | Token::Tilde | Token::Not => {
                    let op = self.tokens.next().unwrap().to_string();
                    let operand = self.parse_unary_expression();
                    return Expr::Unary {
                        op,
                        operand: Box::new(operand),
                    };
                }
                _ => {}
            }
        }
        self.parse_postfix_expression()
    }

    /// Parses postfix expressions (e.g., a(), a[0]).
    fn parse_postfix_expression(&mut self) -> Expr {
        let mut expr = self.parse_primary_expression();
        while let Some(token) = self.tokens.peek() {
            match token {
                Token::LeftParen => {
                    self.tokens.next(); // Consume '('
                    let arguments = if self.tokens.peek() != Some(&Token::RightParen) {
                        self.parse_argument_expression_list()
                    } else {
                        Vec::new()
                    };
                    self.expect_token(Token::RightParen);
                    expr = Expr::FunctionCall {
                        function: Box::new(expr),
                        arguments,
                    };
                }
                _ => break,
            }
        }
        expr
    }

    /// Parses a list of arguments in a function call.
    fn parse_argument_expression_list(&mut self) -> Vec<Expr> {
        let mut args = Vec::new();
        args.push(self.parse_assignment_expression());
        while self.tokens.peek() == Some(&Token::Comma) {
            self.tokens.next(); // Consume ','
            args.push(self.parse_assignment_expression());
        }
        args
    }

    /// Parses primary expressions (e.g., identifiers, constants).
    fn parse_primary_expression(&mut self) -> Expr {
        match self.tokens.next().expect("Expected primary expression") {
            Token::Identifier(s) => {
                if self.is_type_name(&s) {
                    Expr::TypeIdentifier(s)
                } else {
                    Expr::Identifier(s)
                }
            }
            Token::IntConstant(n) => Expr::IntConstant(n),
            Token::FloatConstant(f) => Expr::FloatConstant(f),
            Token::StringLiteral(s) => Expr::StringLiteral(s),
            Token::LeftParen => {
                let expr = self.parse_expression();
                self.expect_token(Token::RightParen);
                expr
            }
            token => panic!("Unexpected token in primary expression: {:?}", token),
        }
    }

    // --- Statement Parsing ---

    /// Parses a statement based on the next token.
    fn parse_statement(&mut self) -> Stmt {
        match self.tokens.peek() {
            Some(Token::LeftBrace) => self.parse_compound_statement(),
            Some(Token::IfKw) => self.parse_if_statement(),
            Some(Token::WhileKw) => self.parse_while_statement(),
            Some(Token::ReturnKw) => self.parse_return_statement(),
            _ => self.parse_expression_statement(),
        }
    }

    /// Parses a compound statement (e.g., { ... }).
    fn parse_compound_statement(&mut self) -> Stmt {
        self.type_table.push(HashSet::new()); // Enter new scope
        self.expect_token(Token::LeftBrace);
        let mut declarations = Vec::new();
        let mut statements = Vec::new();
        while self.is_declaration_start() {
            declarations.push(self.parse_declaration());
        }
        while self.tokens.peek() != Some(&Token::RightBrace) {
            statements.push(self.parse_statement());
        }
        self.expect_token(Token::RightBrace);
        self.type_table.pop(); // Exit scope
        Stmt::Compound { declarations, statements }
    }

    /// Parses an if statement.
    fn parse_if_statement(&mut self) -> Stmt {
        self.expect_token(Token::IfKw);
        self.expect_token(Token::LeftParen);
        let condition = self.parse_expression();
        self.expect_token(Token::RightParen);
        let then_stmt = Box::new(self.parse_statement());
        let else_stmt = if self.tokens.peek() == Some(&Token::ElseKw) {
            self.tokens.next(); // Consume 'else'
            Some(Box::new(self.parse_statement()))
        } else {
            None
        };
        Stmt::If { condition, then_stmt, else_stmt }
    }

    /// Parses a while statement.
    fn parse_while_statement(&mut self) -> Stmt {
        self.expect_token(Token::WhileKw);
        self.expect_token(Token::LeftParen);
        let condition = self.parse_expression();
        self.expect_token(Token::RightParen);
        let body = Box::new(self.parse_statement());
        Stmt::While { condition, body }
    }

    /// Parses a return statement.
    fn parse_return_statement(&mut self) -> Stmt {
        self.expect_token(Token::ReturnKw);
        let expr = if self.tokens.peek() != Some(&Token::Semicolon) {
            Some(self.parse_expression())
        } else {
            None
        };
        self.expect_token(Token::Semicolon);
        Stmt::Return(expr)
    }

    /// Parses an expression statement.
    fn parse_expression_statement(&mut self) -> Stmt {
        let expr = if self.tokens.peek() != Some(&Token::Semicolon) {
            Some(self.parse_expression())
        } else {
            None
        };
        self.expect_token(Token::Semicolon);
        Stmt::Expression(expr)
    }

    // --- Declaration Parsing ---

    /// Parses a declaration (e.g., int x; or typedef int A;).
    fn parse_declaration(&mut self) -> Declaration {
        let mut is_typedef = false;
        let mut type_specifier = String::new();

        // Parse declaration specifiers
        while let Some(token) = self.tokens.peek() {
            match token {
                Token::TypedefKw => {
                    is_typedef = true;
                    self.tokens.next();
                }
                Token::IntKw => {
                    type_specifier = "int".to_string();
                    self.tokens.next();
                    break;
                }
                Token::FloatKw => {
                    type_specifier = "float".to_string();
                    self.tokens.next();
                    break;
                }
                Token::Identifier(ref s) if self.is_type_name(s) => {
                    type_specifier = s.clone();
                    self.tokens.next();
                    break;
                }
                _ => break,
            }
        }

        let mut init_declarators = Vec::new();
        if self.tokens.peek() != Some(&Token::Semicolon) {
            loop {
                let decl = self.parse_init_declarator();
                init_declarators.push(decl);
                if self.tokens.peek() != Some(&Token::Comma) {
                    break;
                }
                self.tokens.next(); // Consume ','
            }
        }
        self.expect_token(Token::Semicolon);

        if is_typedef {
            for init_decl in &init_declarators {
                if let Declarator::Identifier(ref name) = init_decl.declarator {
                    self.type_table.last_mut().unwrap().insert(name.clone());
                }
            }
        }

        Declaration { is_typedef, type_specifier, init_declarators }
    }

    /// Parses an initialized declarator (e.g., x = 5).
    fn parse_init_declarator(&mut self) -> InitDeclarator {
        let declarator = self.parse_declarator();
        let initializer = if self.tokens.peek() == Some(&Token::Equal) {
            self.tokens.next(); // Consume '='
            Some(self.parse_assignment_expression())
        } else {
            None
        };
        InitDeclarator { declarator, initializer }
    }

    /// Parses a declarator (simplified to identifiers and pointers).
    fn parse_declarator(&mut self) -> Declarator {
        if self.tokens.peek() == Some(&Token::Star) {
            self.tokens.next(); // Consume '*'
            Declarator::Pointer(Box::new(self.parse_declarator()))
        } else {
            Declarator::Identifier(self.consume_identifier())
        }
    }

    // --- Helper Functions ---

    /// Checks if the next token starts a declaration.
    fn is_declaration_start(&mut self) -> bool {
        if let Some(token) = self.tokens.peek() {
            matches!(token, Token::TypedefKw | Token::IntKw | Token::FloatKw) ||
            if let Token::Identifier(ref s) = token { self.is_type_name(s) } else { false }
        } else {
            false
        }
    }

    /// Checks if a string is a type name in the current scope.
    fn is_type_name(&self, s: &str) -> bool {
        self.type_table.iter().rev().any(|scope| scope.contains(s))
    }

    /// Checks if a token is an assignment operator.
    fn is_assignment_operator(&self, token: &Token) -> bool {
        matches!(token, Token::Equal | Token::MulAssign | Token::DivAssign | Token::AddAssign | Token::SubAssign)
    }

    /// Expects and consumes a specific token, panics if not found.
    fn expect_token(&mut self, expected: Token) {
        let token = self.tokens.next().expect("Unexpected end of input");
        if token != expected {
            panic!("Expected {:?}, got {:?}", expected, token);
        }
    }

    /// Consumes an identifier token and returns its string.
    fn consume_identifier(&mut self) -> String {
        match self.tokens.next().expect("Expected identifier") {
            Token::Identifier(s) => s,
            token => panic!("Expected identifier, got {:?}", token),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Token;
    use logos::Logos;

    fn parse_source(source: &str) -> Vec<Stmt> {
        let lex = Token::lexer(source);
        let mut parser = Parser::new(lex);
        parser.parse_program()
    }

    #[test]
    fn test_typedef_and_usage() {
        let source = "typedef int A; A x;";
        let stmts = parse_source(source);
        assert_eq!(stmts.len(), 2);
        assert_eq!(
            stmts[0],
            Stmt::Compound {
                declarations: vec![Declaration {
                    is_typedef: true,
                    type_specifier: "int".to_string(),
                    init_declarators: vec![InitDeclarator {
                        declarator: Declarator::Identifier("A".to_string()),
                        initializer: None,
                    }],
                }],
                statements: Vec::new(),
            }
        );
        assert_eq!(
            stmts[1],
            Stmt::Compound {
                declarations: vec![Declaration {
                    is_typedef: false,
                    type_specifier: "A".to_string(),
                    init_declarators: vec![InitDeclarator {
                        declarator: Declarator::Identifier("x".to_string()),
                        initializer: None,
                    }],
                }],
                statements: Vec::new(),
            }
        );
    }

    #[test]
    fn test_expressions() {
        let source = "a + b * 2;";
        let stmts = parse_source(source);
        assert_eq!(
            stmts[0],
            Stmt::Expression(Some(Expr::Binary {
                left: Box::new(Expr::Identifier("a".to_string())),
                op: "+".to_string(),
                right: Box::new(Expr::Binary {
                    left: Box::new(Expr::Identifier("b".to_string())),
                    op: "*".to_string(),
                    right: Box::new(Expr::IntConstant(2)),
                }),
            }))
        );
    }

    #[test]
    fn test_compound_statement() {
        let source = "{ typedef int A; A x = 5; x + 1; }";
        let stmts = parse_source(source);
        assert_eq!(
            stmts[0],
            Stmt::Compound {
                declarations: vec![Declaration {
                    is_typedef: true,
                    type_specifier: "int".to_string(),
                    init_declarators: vec![InitDeclarator {
                        declarator: Declarator::Identifier("A".to_string()),
                        initializer: None,
                    }],
                }],
                statements: vec![
                    Stmt::Compound {
                        declarations: vec![Declaration {
                            is_typedef: false,
                            type_specifier: "A".to_string(),
                            init_declarators: vec![InitDeclarator {
                                declarator: Declarator::Identifier("x".to_string()),
                                initializer: Some(Expr::IntConstant(5)),
                            }],
                        }],
                        statements: Vec::new(),
                    },
                    Stmt::Expression(Some(Expr::Binary {
                        left: Box::new(Expr::Identifier("x".to_string())),
                        op: "+".to_string(),
                        right: Box::new(Expr::IntConstant(1)),
                    })),
                ],
            }
        );
    }

    #[test]
    fn test_if_statement() {
        let source = "if (a > b) return 1; else x = 2;";
        let stmts = parse_source(source);
        assert_eq!(
            stmts[0],
            Stmt::If {
                condition: Expr::Binary {
                    left: Box::new(Expr::Identifier("a".to_string())),
                    op: ">".to_string(),
                    right: Box::new(Expr::Identifier("b".to_string())),
                },
                then_stmt: Box::new(Stmt::Return(Some(Expr::IntConstant(1)))),
                else_stmt: Some(Box::new(Stmt::Expression(Some(Expr::Assignment {
                    left: Box::new(Expr::Identifier("x".to_string())),
                    op: "=".to_string(),
                    right: Box::new(Expr::IntConstant(2)),
                })))),
            }
        );
    }

    #[test]
    fn test_while_statement() {
        let source = "while (x < 10) x = x + 1;";
        let stmts = parse_source(source);
        assert_eq!(
            stmts[0],
            Stmt::While {
                condition: Expr::Binary {
                    left: Box::new(Expr::Identifier("x".to_string())),
                    op: "<".to_string(),
                    right: Box::new(Expr::IntConstant(10)),
                },
                body: Box::new(Stmt::Expression(Some(Expr::Assignment {
                    left: Box::new(Expr::Identifier("x".to_string())),
                    op: "=".to_string(),
                    right: Box::new(Expr::Binary {
                        left: Box::new(Expr::Identifier("x".to_string())),
                        op: "+".to_string(),
                        right: Box::new(Expr::IntConstant(1)),
                    }),
                }))),
            }
        );
    }
}
