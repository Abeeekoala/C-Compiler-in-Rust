// src/parser.rs

use crate::ast::SwitchCase;
use crate::lexer::Token;
use crate::ast::{AstNode, TypeSpecifier};
use std::collections::HashMap;
use crate::error::CompileError;

/// The parser struct holds the list of tokens and provides methods to parse them.
pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
    type_defs: HashMap<String, (TypeSpecifier, usize)>,
}

type ParseResult = Result<AstNode, CompileError>;

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            current: 0,
            type_defs: HashMap::new(),
        }
    }

    fn peek(&self) -> Option<&Token> {
        if self.current < self.tokens.len() {
            Some(&self.tokens[self.current])
        } else {
            None
        }
    }

    fn advance(&mut self) -> Option<Token> {
        if self.current < self.tokens.len() {
            let token = self.tokens[self.current].clone();
            self.current += 1;
            Some(token)
        } else {
            None
        }
    }

    pub fn parse_translation_unit(&mut self) -> ParseResult {
        let mut declarations = Vec::new();

        println!("Starting to parse translation unit");

        while self.current < self.tokens.len() {
            println!("Parsing external declaration at token: {:?}", self.peek());
            declarations.push(Box::new(self.parse_external_declaration()?));
        }

        println!("Finished parsing translation unit with {} declarations", declarations.len());

        Ok(AstNode::NodeList(declarations))
    }

    fn parse_external_declaration(&mut self) -> ParseResult {
        let start_pos = self.current;

        if let Some(Token::TypedefKw) = self.peek() {
            return self.parse_typedef_declaration();
        }

        if let Some(Token::EnumKw) = self.peek() {
            return self.parse_enum_declaration();
        }

        if let Some(Token::StructKw) = self.peek() {
            self.advance();
            let struct_name = self.parse_identifier()?;
            if let Some(Token::LBrace) = self.peek() {
                self.advance();
                let fields = self.parse_struct_fields()?;
                self.expect_token(Token::RBrace)?;
                self.expect_token(Token::Semicolon)?;
                return Ok(AstNode::StructDefinition { name: struct_name, fields });
            } else {
                self.current = start_pos;
            }
        }

        let (type_spec, pointer_level) = self.parse_type_specifier()?;
        let identifier = self.parse_identifier()?;
        let declarator = AstNode::Identifier(identifier);

        if let Some(Token::LParen) = self.peek() {
            self.advance();
            let parameters = self.parse_parameter_list()?;
            self.expect_token(Token::RParen)?;

            if let Some(Token::LBrace) = self.peek() {
                let body = self.parse_compound_statement()?;
                let mut final_declarator = declarator;
                return Ok(AstNode::FunctionDefinition {
                    decl_specifiers: vec![Box::new(AstNode::TypeSpecifier(type_spec))],
                    declarator: Box::new(final_declarator),
                    parameters,
                    compound_statement: Box::new(body),
                });
            } else {
                self.expect_token(Token::Semicolon)?;
                return Ok(AstNode::FunctionDeclaration {
                    decl_specifiers: vec![Box::new(AstNode::TypeSpecifier(type_spec))],
                    declarator: Box::new(declarator),
                    parameters,
                });
            }
        } else {
            let mut final_declarator = declarator;
            for _ in 0..pointer_level {
                final_declarator = AstNode::PointerDeclarator { pointee: Box::new(final_declarator) };
            }
            while let Some(Token::LBracket) = self.peek() {
                self.advance();
                let size = self.parse_expression()?;
                self.expect_token(Token::RBracket)?;
                final_declarator = AstNode::ArrayDeclarator {
                    base: Box::new(final_declarator),
                    size: Box::new(size),
                };
            }
            let initializer = if let Some(Token::Assign) = self.peek() {
                self.advance();
                Some(Box::new(self.parse_initializer()?))
            } else {
                None
            };
            self.expect_token(Token::Semicolon)?;
            Ok(AstNode::Declaration {
                type_spec: type_spec,
                declarator: Box::new(final_declarator),
                initializer,
            })
        }
    }

    /// Parse a typedef declaration
    fn parse_typedef_declaration(&mut self) -> ParseResult {
        self.advance();
        let (mut type_spec, mut pointer_level) = self.parse_type_specifier()?;

        while let TypeSpecifier::TypedefName(name) = &type_spec {
            if let Some((base_type, additional_pointers)) = self.type_defs.get(name) {
                type_spec = base_type.clone();
                pointer_level += additional_pointers;
            } else {
                return Err(CompileError::ParserError(format!("Undefined typedef '{}'", name)));
            }
        }

        let alias = self.parse_identifier()?;
        let mut declarator = AstNode::Identifier(alias.clone());

        for _ in 0..pointer_level {
            declarator = AstNode::PointerDeclarator {
                pointee: Box::new(declarator),
            };
        }

        while let Some(Token::LBracket) = self.peek() {
            self.advance();
            let size = self.parse_expression()?;
            self.expect_token(Token::RBracket)?;
            declarator = AstNode::ArrayDeclarator {
                base: Box::new(declarator),
                size: Box::new(size),
            };
        }

        self.expect_token(Token::Semicolon)?;
        self.type_defs.insert(alias.clone(), (type_spec.clone(), pointer_level));
        Ok(AstNode::TypedefDeclaration {
            type_spec,
            declarator: Box::new(declarator),
        })
    }

    fn parse_declaration_specifiers(&mut self) -> Result<TypeSpecifier, CompileError> {
        let token = self.peek().ok_or_else(|| CompileError::ParserError("Unexpected end of file".to_string()))?;

        match token {
            Token::IntKw => {
                self.advance();
                Ok(TypeSpecifier::Int)
            },
            Token::UnsignedKw => {
                self.advance();
                Ok(TypeSpecifier::Unsigned)
            },
            Token::CharKw => {
                self.advance();
                Ok(TypeSpecifier::Char)
            },
            Token::VoidKw => {
                self.advance();
                Ok(TypeSpecifier::Void)
            },
            Token::FloatKw =>{
                self.advance();
                Ok(TypeSpecifier::Float)
            },
            Token::DoubleKw => {
                self.advance();
                Ok(TypeSpecifier::Double)
            },
            Token::StructKw => {
                self.advance();
                if let Some(Token::Identifier(name)) = self.peek() {
                    let struct_name = name.clone();
                    self.advance();
                    if let Some(Token::LBrace) = self.peek() {
                        self.advance();
                        let fields = self.parse_struct_fields()?;
                        self.expect_token(Token::RBrace)?;

                        return Ok(TypeSpecifier::Struct(struct_name));
                    } else {
                        return Ok(TypeSpecifier::Struct(struct_name));
                    }
                } else {
                    return Err(CompileError::ParserError("Expected identifier after 'struct'".to_string()));
                }
            },
            _ => Err(CompileError::ParserError(format!("Expected type specifier, found {:?}", token)))
        }
    }

    /// Parsing fields inside teh struct
    fn parse_struct_fields(&mut self) -> Result<Vec<Box<AstNode>>, CompileError> {
        let mut fields = Vec::new();

        while let Some(token) = self.peek() {
            if *token == Token::RBrace {
                break;
            }
            let (field_type, _) = self.parse_type_specifier()?;
            let field_name = self.parse_identifier()?;
            let field_node = AstNode::StructField {
                type_spec: field_type,
                name: field_name,
            };

            fields.push(Box::new(field_node));
            self.expect_token(Token::Semicolon)?;
        }
        Ok(fields)
    }

    pub fn parse_initializer(&mut self) -> ParseResult {
        match self.peek() {
            Some(Token::LBrace) => self.parse_initializer_list(),
            _ => self.parse_assignment_expression(),
        }
    }

    pub fn parse_initializer_list(&mut self) -> ParseResult {
        self.expect_token(Token::LBrace)?;

        let mut initializers = Vec::new();

        if let Some(Token::RBrace) = self.peek() {
            self.advance();
            return Ok(AstNode::InitializerList(initializers));
        }

        initializers.push(Box::new(self.parse_initializer()?));

        while let Some(Token::Comma) = self.peek() {
            self.advance();
            if let Some(Token::RBrace) = self.peek() {
                break;
            }
            initializers.push(Box::new(self.parse_initializer()?));
        }

        self.expect_token(Token::RBrace)?;

        Ok(AstNode::InitializerList(initializers))
    }

    /// Parse a declarator
    fn parse_declarator(&mut self) -> ParseResult {
        let mut pointers = 0;
        // Count the number of pointer levels
        while let Some(Token::Mul) = self.peek() {
            self.advance();
            pointers += 1;
        }

        let identifier = self.parse_identifier()?;
        let mut declarator = AstNode::Identifier(identifier);

        while let Some(Token::LBracket) = self.peek() {
            self.advance();
            let size = self.parse_expression()?;
            self.expect_token(Token::RBracket)?;
            declarator = AstNode::ArrayDeclarator {
                base: Box::new(declarator),
                size: Box::new(size),
            };
        }

        let mut result = declarator;
        for _ in 0..pointers {
            result = AstNode::PointerDeclarator {
                pointee: Box::new(result),
            };
        }
        Ok(result)
    }

    pub fn parse_declaration(&mut self) -> ParseResult {
        let (type_specifier, pointer_level) = self.parse_type_specifier()?;
        let mut declarator = self.parse_declarator()?;

        while let Some(Token::LBracket) = self.peek() {
            self.advance();
            let size = self.parse_expression()?;
            self.expect_token(Token::RBracket)?;
            declarator = AstNode::ArrayDeclarator {
                base: Box::new(declarator),
                size: Box::new(size),
            };
        }

        let initializer = match self.peek() {
            Some(token) if *token == Token::Assign => {
                self.expect_token(Token::Assign)?;
                Some(Box::new(self.parse_initializer()?))
            },
            _ => None,
        };

        self.expect_token(Token::Semicolon)?;

        let mut final_declarator = declarator;
        for _ in 0..pointer_level {
            final_declarator = AstNode::PointerDeclarator {
                pointee: Box::new(final_declarator),
            };
        }

        Ok(AstNode::Declaration {
            type_spec: type_specifier,
            declarator: Box::new(final_declarator),
            initializer,
        })
    }


    fn parse_compound_statement(&mut self) -> ParseResult {
        self.expect_token(Token::LBrace)?;
        let mut statements = Vec::new();

        while !self.check_token(Token::RBrace) {
            statements.push(Box::new(self.parse_statement()?));
        }

        self.expect_token(Token::RBrace)?;
        Ok(AstNode::BlockStatement(statements))
    }

    pub fn parse_statement(&mut self) -> Result<AstNode, CompileError> {
        match self.peek() {
            Some(Token::EnumKw) => {
                let start_pos = self.current;
                let result = self.parse_enum_declaration();

                match result {
                    Ok(AstNode::TypeSpecifier(_)) => {
                        self.current = start_pos;
                        self.parse_declaration()
                    },
                    _ => result
                }
            },
            Some(Token::StructKw) => {
                self.advance();

                if let Some(Token::Identifier(struct_name)) = self.peek() {
                    let struct_type = struct_name.clone();
                    self.advance();

                    if let Some(Token::LBrace) = self.peek() {
                        self.advance();
                        let fields = self.parse_struct_fields()?;
                        self.expect_token(Token::RBrace)?;
                        self.expect_token(Token::Semicolon)?;

                        Ok(AstNode::StructDefinition {
                            name: struct_type,
                            fields,
                        })
                    } else {
                        if let Some(Token::Identifier(var_name)) = self.peek() {
                            let variable_name = var_name.clone();
                            self.advance();
                            let initializer = None;
                            self.expect_token(Token::Semicolon)?;

                            Ok(AstNode::Declaration {
                                type_spec: TypeSpecifier::Struct(struct_type),
                                declarator: Box::new(AstNode::Identifier(variable_name)),
                                initializer,
                            })
                        } else {
                            Err(CompileError::ParserError("Expected identifier after struct type".to_string()))
                        }
                    }
                } else {
                    Err(CompileError::ParserError("Expected struct name".to_string()))
                }
            },

            Some(token) if self.is_type_specifier(token) => {
                self.parse_declaration()
            },
            Some(Token::Semicolon) => {
                self.advance();
                Ok(AstNode::Empty)
            },
            Some(Token::LBrace) => {
                self.parse_compound_statement()
            },
            Some(Token::IfKw) => {
                self.parse_if_statement()
            },
            Some(Token::WhileKw) => {
                self.parse_while_statement()
            },
            Some(Token::ForKw) => {
                self.parse_for_statement()
            },
            Some(Token::ReturnKw) => {
                self.parse_return_statement()
            },
            Some(Token::BreakKw) => {
                self.advance();
                self.expect_token(Token::Semicolon)?;
                Ok(AstNode::BreakStatement)
            },
            Some(Token::ContinueKw) => {
                self.advance();
                self.expect_token(Token::Semicolon)?;
                Ok(AstNode::ContinueStatement)
            },
            Some(Token::SwitchKw) => {
                self.parse_switch_statement()
            },
            Some(Token::TypedefKw) => {
                self.parse_typedef_declaration()
            },
            _ => {
                self.parse_expression_statement()
            }
        }
    }

    fn parse_switch_statement(&mut self) -> ParseResult {
        self.advance();
        self.expect_token(Token::LParen)?;
        let expr = self.parse_expression()?;
        self.expect_token(Token::RParen)?;
        self.expect_token(Token::LBrace)?;

        let mut cases = Vec::new();
        let mut default = None;

        while !self.check_token(Token::RBrace) {
            match self.peek() {
                Some(Token::CaseKw) => {
                    self.advance();
                    let value = self.parse_constant_expression()?;
                    self.expect_token(Token::Colon)?;
                    let mut body = Vec::new();
                    while !matches!(self.peek(), Some(Token::CaseKw | Token::DefaultKw | Token::RBrace)) {
                        body.push(Box::new(self.parse_statement()?));
                    }
                    cases.push(SwitchCase { value: Box::new(value), body });
                }
                Some(Token::DefaultKw) => {
                    self.advance();
                    self.expect_token(Token::Colon)?;
                    let mut body = Vec::new();
                    while !matches!(self.peek(), Some(Token::CaseKw | Token::DefaultKw | Token::RBrace)) {
                        body.push(Box::new(self.parse_statement()?));
                    }
                    default = Some(body);
                }
                _ => return Err(CompileError::ParserError("Unexpected token in switch statement".into())),
            }
        }

        self.expect_token(Token::RBrace)?;
        Ok(AstNode::SwitchStatement {
            expr: Box::new(expr),
            cases,
            default,
        })
    }

    fn parse_constant_expression(&mut self) -> ParseResult {
        let start_pos = self.current;
        let expr = self.parse_conditional_expression()?;

        if !Self::is_constant_expression(&expr) {
            self.current = start_pos;
            return Err(CompileError::ParserError(
                "Non-constant expression in case label".into()
            ));
        }
        Ok(expr)
    }

    /// Parsing conditions with case
    fn is_constant_expression(node: &AstNode) -> bool {
        match node {
            AstNode::IntConstant(_) | AstNode::CharConstant(_) => true,
            AstNode::UnaryOperation { op: _, operand } =>
                Self::is_constant_expression(operand),
            AstNode::BinaryOperation { op: _, left, right } =>
                Self::is_constant_expression(left) && Self::is_constant_expression(right),
            _ => false
        }
    }

    fn parse_for_statement(&mut self) -> ParseResult {
        self.advance();
        self.expect_token(Token::LParen)?;

        let init = if self.check_token(Token::Semicolon) {
            AstNode::Empty
        } else {
            self.parse_expression_statement()?
        };

        let condition = if self.check_token(Token::Semicolon) {
            AstNode::IntConstant(1) // Default we set to true here
        } else {
            let expr = self.parse_expression()?;
            self.expect_token(Token::Semicolon)?;
            expr
        };

        let increment = if self.check_token(Token::RParen) {
            AstNode::Empty
        } else {
            let expr = self.parse_expression()?;
            self.expect_token(Token::RParen)?;
            expr
        };
        let body = self.parse_statement()?;

        Ok(AstNode::ForLoop {
            init: Box::new(init),
            condition: Box::new(condition),
            increment: Box::new(increment),
            body: Box::new(body),
        })
    }

    fn parse_while_statement(&mut self) -> Result<AstNode, CompileError> {
        self.advance();
        self.expect_token(Token::LParen)?;
        let condition = self.parse_expression()?;
        self.expect_token(Token::RParen)?;
        let body = self.parse_statement()?;

        Ok(AstNode::WhileStatement {
            condition: Box::new(condition),
            body: Box::new(body),
        })
    }

    fn parse_return_statement(&mut self) -> ParseResult {
        self.advance();

        let expr = if let Some(Token::Semicolon) = self.peek() {
            None // return;
        } else {
            Some(Box::new(self.parse_expression()?))
        };

        self.expect_token(Token::Semicolon)?;

        Ok(AstNode::ReturnStatement(expr))
    }

    pub fn parse_expression_statement(&mut self) -> ParseResult {
        if let Some(Token::Semicolon) = self.peek() {
            self.advance();
            return Ok(AstNode::ExpressionStatement(Box::new(AstNode::NodeList(Vec::new()))));
        }
        let expr = self.parse_expression()?;
        self.expect_token(Token::Semicolon)?;

        Ok(AstNode::ExpressionStatement(Box::new(expr)))
    }

    fn parse_expression(&mut self) -> ParseResult {
        let mut expr = self.parse_assignment_expression()?;

        while let Some(Token::Comma) = self.peek() {
            self.advance();
            let right = self.parse_assignment_expression()?;
            expr = AstNode::BinaryOperation {
                op: ",".to_string(),
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_assignment_expression(&mut self) -> ParseResult {
        let lhs = self.parse_conditional_expression()?;

        match self.peek() {
            Some(Token::Assign) => {
                self.advance();
                let rhs = self.parse_assignment_expression()?;
                return Ok(AstNode::Assignment {
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                });
            },
            Some(Token::AddAssign) => {
                self.advance();
                let rhs = self.parse_assignment_expression()?;
                return Ok(AstNode::Assignment {
                    lhs: Box::new(lhs.clone()),
                    rhs: Box::new(AstNode::BinaryOperation {
                        op: "+".to_string(),
                        left: Box::new(lhs),
                        right: Box::new(rhs),
                    }),
                });
            },
            Some(Token::SubAssign) => {
                self.advance();
                let rhs = self.parse_assignment_expression()?;
                return Ok(AstNode::Assignment {
                    lhs: Box::new(lhs.clone()),
                    rhs: Box::new(AstNode::BinaryOperation {
                        op: "-".to_string(),
                        left: Box::new(lhs),
                        right: Box::new(rhs),
                    }),
                });
            },
            Some(Token::MulAssign) => {
                self.advance();
                let rhs = self.parse_assignment_expression()?;
                return Ok(AstNode::Assignment {
                    lhs: Box::new(lhs.clone()),
                    rhs: Box::new(AstNode::BinaryOperation {
                        op: "*".to_string(),
                        left: Box::new(lhs),
                        right: Box::new(rhs),
                    }),
                });
            },
            Some(Token::DivAssign) => {
                self.advance();
                let rhs = self.parse_assignment_expression()?;
                return Ok(AstNode::Assignment {
                    lhs: Box::new(lhs.clone()),
                    rhs: Box::new(AstNode::BinaryOperation {
                        op: "/".to_string(),
                        left: Box::new(lhs),
                        right: Box::new(rhs),
                    }),
                });
            },
            Some(Token::ModAssign) => {
                self.advance();
                let rhs = self.parse_assignment_expression()?;
                return Ok(AstNode::Assignment {
                    lhs: Box::new(lhs.clone()),
                    rhs: Box::new(AstNode::BinaryOperation {
                        op: "%".to_string(),
                        left: Box::new(lhs),
                        right: Box::new(rhs),
                    }),
                });
            },
            _ => {}
        }
        Ok(lhs)
    }

    /// Parse ternary operator
    fn parse_conditional_expression(&mut self) -> ParseResult {
        let condition = self.parse_logical_or_expression()?;

        if let Some(Token::Question) = self.peek() {
            self.advance();
            let true_expr = self.parse_expression()?;
            self.expect_token(Token::Colon)?;
            let false_expr = self.parse_conditional_expression()?;

            return Ok(AstNode::TernaryOperation {
                condition: Box::new(condition),
                true_expr: Box::new(true_expr),
                false_expr: Box::new(false_expr),
            });
        }
        Ok(condition)
    }

    fn parse_logical_or_expression(&mut self) -> ParseResult {
        let mut expr = self.parse_logical_and_expression()?;

        while let Some(Token::LogicOr) = self.peek() {
            self.advance();
            let right = self.parse_logical_and_expression()?;
            expr = AstNode::BinaryOperation {
                op: "||".to_string(),
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_logical_and_expression(&mut self) -> ParseResult {
        let mut expr = self.parse_inclusive_or_expression()?;

        while let Some(Token::LogicAnd) = self.peek() {
            self.advance();
            let right = self.parse_inclusive_or_expression()?;
            expr = AstNode::BinaryOperation {
                op: "&&".to_string(),
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_inclusive_or_expression(&mut self) -> ParseResult {
        let mut expr = self.parse_exclusive_or_expression()?;

        while let Some(Token::BitOr) = self.peek() {
            self.advance();
            let right = self.parse_exclusive_or_expression()?;
            expr = AstNode::BinaryOperation {
                op: "|".to_string(),
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_exclusive_or_expression(&mut self) -> ParseResult {
        let mut expr = self.parse_and_expression()?;

        while let Some(Token::BitXor) = self.peek() {
            self.advance();
            let right = self.parse_and_expression()?;
            expr = AstNode::BinaryOperation {
                op: "^".to_string(),
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_and_expression(&mut self) -> ParseResult {
        let mut expr = self.parse_equality_expression()?;

        while let Some(Token::BitAnd) = self.peek() {
            self.advance();
            let right = self.parse_equality_expression()?;
            expr = AstNode::BinaryOperation {
                op: "&".to_string(),
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_equality_expression(&mut self) -> ParseResult {
        let mut expr = self.parse_relational_expression()?;

        while let Some(token) = self.peek() {
            let op = match token {
                Token::Equal => "==",
                Token::NotEqual => "!=",
                _ => break,
            };

            self.advance();
            let right = self.parse_relational_expression()?;
            expr = AstNode::BinaryOperation {
                op: op.to_string(),
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

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

            self.advance();
            let right = self.parse_shift_expression()?;
            expr = AstNode::BinaryOperation {
                op: op.to_string(),
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_shift_expression(&mut self) -> ParseResult {
        let mut expr = self.parse_additive_expression()?;

        while let Some(token) = self.peek() {
            let op = match token {
                Token::ShiftLeft => "<<",
                Token::ShiftRight => ">>",
                _ => break,
            };

            self.advance();
            let right = self.parse_additive_expression()?;
            expr = AstNode::BinaryOperation {
                op: op.to_string(),
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_additive_expression(&mut self) -> ParseResult {
        let mut expr = self.parse_multiplicative_expression()?;

        while let Some(token) = self.peek() {
            let op = match token {
                Token::Add => "+",
                Token::Sub => "-",
                _ => break,
            };

            self.advance();
            let right = self.parse_multiplicative_expression()?;
            expr = AstNode::BinaryOperation {
                op: op.to_string(),
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_multiplicative_expression(&mut self) -> ParseResult {
        let mut expr = self.parse_unary_expression()?;

        while let Some(token) = self.peek() {
            let op = match token {
                Token::Mul => "*",
                Token::Div => "/",
                Token::Mod => "%",
                _ => break,
            };

            self.advance();
            let right = self.parse_unary_expression()?;
            expr = AstNode::BinaryOperation {
                op: op.to_string(),
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_unary_expression(&mut self) -> ParseResult {
        match self.peek() {
            Some(Token::Add) => {
                self.advance();
                let expr = self.parse_unary_expression()?;
                Ok(AstNode::UnaryOperation {
                    op: "+".to_string(),
                    operand: Box::new(expr),
                })
            },
            Some(Token::Sub) => {
                self.advance();
                let expr = self.parse_unary_expression()?;
                Ok(AstNode::UnaryOperation {
                    op: "-".to_string(),
                    operand: Box::new(expr),
                })
            },
            Some(Token::Tilde) => {
                self.advance();
                let expr = self.parse_unary_expression()?;
                Ok(AstNode::UnaryOperation {
                    op: "~".to_string(),
                    operand: Box::new(expr),
                })
            },
            Some(Token::Not) => {
                self.advance();
                let expr = self.parse_unary_expression()?;
                Ok(AstNode::UnaryOperation {
                    op: "!".to_string(),
                    operand: Box::new(expr),
                })
            },
            Some(Token::PlusPlus) => {
                self.advance();
                let expr = self.parse_unary_expression()?;
                Ok(AstNode::UnaryOperation {
                    op: "++".to_string(),
                    operand: Box::new(expr),
                })
            },
            Some(Token::MinusMinus) => {
                self.advance();
                let expr = self.parse_unary_expression()?;
                Ok(AstNode::UnaryOperation {
                    op: "--".to_string(),
                    operand: Box::new(expr),
                })
            },
            Some(Token::Mul) => { // Pointer dereference
                self.advance();
                let expr = self.parse_unary_expression()?;
                Ok(AstNode::UnaryOperation {
                    op: "*".to_string(),
                    operand: Box::new(expr),
                })
            },
            Some(Token::BitAnd) => {
                self.advance();
                let expr = self.parse_unary_expression()?;
                Ok(AstNode::UnaryOperation {
                    op: "&".to_string(),
                    operand: Box::new(expr),
                })
            },
            Some(Token::SizeofKw) => {
                self.advance();

                if let Some(Token::LParen) = self.peek() {
                    self.advance();

                    if let Ok((type_spec, pointer_level)) = self.parse_type_specifier() {
                        self.expect_token(Token::RParen)?;
                        return Ok(AstNode::SizeofType {
                            type_spec,
                            pointer_level
                        });
                    } else {
                        let expr = self.parse_expression()?;
                        self.expect_token(Token::RParen)?;
                        return Ok(AstNode::SizeofExpr {
                            expr: Box::new(expr)
                        });
                    }
                } else {
                    let operand = self.parse_unary_expression()?;
                    return Ok(AstNode::SizeofExpr {
                        expr: Box::new(operand)
                    });
                }
            },
            _ => self.parse_postfix_expression(),
        }
    }

    fn parse_postfix_expression(&mut self) -> ParseResult {
        let mut expr = self.parse_primary_expression()?;

        loop {
            match self.peek() {
                Some(Token::LBracket) => {
                    self.advance();
                    let index = self.parse_expression()?;
                    self.expect_token(Token::RBracket)?;
                    expr = AstNode::ArraySubscript {
                        array: Box::new(expr),
                        index: Box::new(index),
                    };
                },
                Some(Token::LParen) => {
                    self.advance();

                    let mut args = Vec::new();
                    if let Some(Token::RParen) = self.peek() {
                        // Empty list
                    } else {
                        args.push(Box::new(self.parse_assignment_expression()?));
                        while let Some(Token::Comma) = self.peek() {
                            self.advance();
                            args.push(Box::new(self.parse_assignment_expression()?));
                        }
                    }

                    self.expect_token(Token::RParen)?;
                    expr = AstNode::FunctionCall {
                        function: Box::new(expr),
                        args,
                    };
                },
                Some(Token::PlusPlus) => {
                    self.advance();
                    expr = AstNode::UnaryOperation {
                        op: "post++".to_string(),
                        operand: Box::new(expr),
                    };
                },
                Some(Token::MinusMinus) => {
                    self.advance();
                    expr = AstNode::UnaryOperation {
                        op: "post--".to_string(),
                        operand: Box::new(expr),
                    };
                },
                Some(Token::Dot) => {
                    self.advance();
                    let member = self.parse_identifier()?;
                    expr = AstNode::MemberAccess {
                        object: Box::new(expr),
                        member,
                    };
                },
                Some(Token::Arrow) => {
                    self.advance();
                    let member = self.parse_identifier()?;
                    expr = AstNode::PointerMemberAccess {
                        pointer: Box::new(expr),
                        member,
                    };
                },
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_primary_expression(&mut self) -> ParseResult {
        match self.peek() {
            Some(Token::IntLiteralDec((value, _))) |
            Some(Token::IntLiteralHex((value, _))) |
            Some(Token::IntLiteralOct((value, _))) => {
                let val = *value;
                self.advance();
                Ok(AstNode::IntConstant(val as i32))
            },
            Some(Token::IntLiteralChar(value)) => {
                let val = *value;
                self.advance();
                Ok(AstNode::IntConstant(val as i32))
            },
            Some(Token::FloatLiteral((value, _suffix))) => {
                let val = *value;
                self.advance();
                Ok(AstNode::FloatConstant(val))
            },
            Some(Token::StringLiteral(s)) => {
                let string = s.clone();
                self.advance();
                Ok(AstNode::StringLiteral(string))
            },
            Some(Token::Identifier(name)) => {
                let id = name.clone();
                self.advance();
                Ok(AstNode::Identifier(id))
            },
            Some(Token::LParen) => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect_token(Token::RParen)?;
                Ok(expr)
            },
            Some(token) => Err(CompileError::ParserError(format!("Expected primary expression, found {:?}", token))),
            None => Err(CompileError::ParserError("Unexpected end of file".to_string())),
        }
    }


    fn parse_if_statement(&mut self) -> ParseResult {
        self.advance();
        self.expect_token(Token::LParen)?;
        let condition = self.parse_expression()?;
        self.expect_token(Token::RParen)?;
        let then_stmt = self.parse_statement()?;
        let else_stmt = if let Some(Token::ElseKw) = self.peek() {
            self.advance();
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

    pub fn parse_identifier(&mut self) -> Result<String, CompileError> {
        let token_clone = self.peek().ok_or_else(|| CompileError::ParserError("Unexpected end of file".to_string()))?.clone();

        match token_clone {
            Token::Identifier(name) => {
                self.advance();
                Ok(name)
            },
            _ => Err(CompileError::ParserError(format!("Expected identifier, found {:?}", token_clone)))
        }
    }

    pub fn parse_type_specifier(&mut self) -> Result<(TypeSpecifier, usize), CompileError> {
        let token = self.peek().ok_or_else(|| CompileError::ParserError("Unexpected end of file".to_string()))?;

        let base_type = match token {
            Token::IntKw => {
                self.advance();
                TypeSpecifier::Int
            },
            Token::CharKw => {
                self.advance();
                TypeSpecifier::Char
            },
            Token::VoidKw => {
                self.advance();
                TypeSpecifier::Void
            },
            Token::FloatKw => {
                self.advance();
                TypeSpecifier::Float
            },
            Token::DoubleKw => {
                self.advance();
                TypeSpecifier::Double
            },
            Token::UnsignedKw => {
                self.advance();
                if let Some(Token::IntKw) = self.peek() {
                    self.advance();
                }
                TypeSpecifier::Unsigned
            },
            Token::StructKw => {
                self.advance();
                if let Some(Token::Identifier(name)) = self.peek() {
                    let struct_name = name.clone();
                    self.advance();
                    if let Some(Token::LBrace) = self.peek() {
                        self.advance();
                        let fields = self.parse_struct_fields()?;
                        self.expect_token(Token::RBrace)?;
                        TypeSpecifier::Struct(struct_name)
                    } else {
                        TypeSpecifier::Struct(struct_name)
                    }
                } else {
                    return Err(CompileError::ParserError("Expected identifier after 'struct'".to_string()));
                }
            },
            Token::EnumKw => {
                self.advance();
                if let Some(Token::Identifier(name)) = self.peek() {
                    let enum_name = name.clone();
                    self.advance();

                    if let Some(Token::LBrace) = self.peek() {
                        self.advance();
                        let values = self.parse_enum_values()?;
                        self.expect_token(Token::RBrace)?;
                        TypeSpecifier::Enum(enum_name)
                    } else {
                        TypeSpecifier::Enum(enum_name)
                    }
                } else {
                    return Err(CompileError::ParserError("Expected identifier after 'enum'".to_string()));
                }
            },
            Token::Identifier(name) => {
                if self.type_defs.contains_key(name) {
                    let type_name = name.clone();
                    self.advance();
                    TypeSpecifier::TypedefName(type_name)
                } else {
                    return Err(CompileError::ParserError(format!("Expected type specifier, found identifier '{}'", name)));
                }
            },
            _ => return Err(CompileError::ParserError(format!("Expected type specifier, found {:?}", token))),
        };

        let mut pointer_level = 0;
        while let Some(Token::Mul) = self.peek() {
            self.advance();
            pointer_level += 1;
        }

        Ok((base_type, pointer_level))
    }

    fn parse_parameter_declaration(&mut self) -> Result<AstNode, CompileError> {
        let (type_specifier, pointer_level) = self.parse_type_specifier()?;
        let declarator = self.parse_declarator()?;
        let mut final_declarator = declarator;
        for _ in 0..pointer_level {
            final_declarator = AstNode::PointerDeclarator {
                pointee: Box::new(final_declarator),
            };
        }

        Ok(AstNode::Declaration {
            type_spec: type_specifier,
            declarator: Box::new(final_declarator),
            initializer: None,
        })
    }

    /// Parse a parameter list so if multiple parameters are declared at once
    fn parse_parameter_list(&mut self) -> Result<Vec<Box<AstNode>>, CompileError> {
        let mut params = Vec::new();
        if let Some(Token::RParen) = self.peek() {
            return Ok(params);
        }
        params.push(Box::new(self.parse_parameter_declaration()?));
        while let Some(Token::Comma) = self.peek() {
            self.advance();
            params.push(Box::new(self.parse_parameter_declaration()?));
        }

        Ok(params)
    }

    fn parse_enum_values(&mut self) -> Result<Vec<(String, i32)>, CompileError> {
        let mut values = Vec::new();
        let mut next_implicit_value = 0;

        while !self.check_token(Token::RBrace) {
            if let Some(Token::Identifier(name)) = self.peek() {
                let enum_name = name.clone();
                self.advance();

                let value = if let Some(Token::Assign) = self.peek() {
                    self.advance(); // Consume '='
                    let expr = self.parse_constant_expression()?;

                    match expr {
                        AstNode::IntConstant(val) => {
                            next_implicit_value = val + 1;
                            val
                        },
                        _ => return Err(CompileError::ParserError(
                            "Expected integer constant for enum value".to_string()
                        ))
                    }
                } else {
                    let val = next_implicit_value;
                    next_implicit_value += 1;
                    val
                };

                values.push((enum_name, value));

                match self.peek() {
                    Some(Token::Comma) => {
                        self.advance();
                    }
                    Some(Token::RBrace) => {
                        break;
                    }
                    _ => {
                        return Err(CompileError::ParserError(
                            "Expected comma or closing brace in enum definition".to_string()
                        ));
                    }
                }
            } else {
                return Err(CompileError::ParserError(
                    "Expected identifier in enum definition".to_string()
                ));
            }
        }
        Ok(values)
    }

    fn parse_enum_declaration(&mut self) -> ParseResult {
        self.advance();

        let enum_name = if let Some(Token::Identifier(name)) = self.peek() {
            let name_val = name.clone();
            self.advance();
            name_val
        } else {
            return Err(CompileError::ParserError("Expected identifier after 'enum'".to_string()));
        };

        if let Some(Token::LBrace) = self.peek() {
            self.advance();
            let values = self.parse_enum_values()?;

            self.expect_token(Token::RBrace)?;
            self.expect_token(Token::Semicolon)?;

            Ok(AstNode::EnumDefinition {
                name: enum_name,
                values,
            })
        } else if let Some(Token::Semicolon) = self.peek() {
            self.advance();
            Ok(AstNode::EnumDefinition {
                name: enum_name,
                values: Vec::new(),
            })
        } else {
            Ok(AstNode::TypeSpecifier(TypeSpecifier::Enum(enum_name)))
        }
    }

    fn is_type_specifier(&self, token: &Token) -> bool {
        match token {
            Token::IntKw | Token::FloatKw | Token::CharKw | Token::DoubleKw |
            Token::VoidKw | Token::UnsignedKw | Token::StructKw | Token::EnumKw => true,
            Token::Identifier(name) => self.type_defs.contains_key(name),
            _ => false,
        }
    }
}
