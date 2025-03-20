// src/ast.rs

use std::fmt;

/// A trait for AST nodes.
/// Each node should be able to generate code and print itself.
pub trait Node {
    fn emit_risc(&self, context: &mut Context) -> String;
    fn print(&self) -> String;
}

/// The compilation context.
pub struct Context {
    // To be implemented
}

/// The different kinds of AST nodes.
#[derive(Debug, Clone, PartialEq)]
pub enum AstNode {
    Empty,
    SwitchStatement {
        expr: Box<AstNode>,
        cases: Vec<SwitchCase>,
        default: Option<Vec<Box<AstNode>>>,
    },
    BreakStatement,
    ContinueStatement,
    /// A function definition (e.g. int main() { ... }).
    FunctionDefinition {
        decl_specifiers: Vec<Box<AstNode>>,
        declarator: Box<AstNode>,
        parameters: Vec<Box<AstNode>>,
        compound_statement: Box<AstNode>,
    },
    /// A function declaration (e.g. int main(void)).
    FunctionDeclaration {
        decl_specifiers: Vec<Box<AstNode>>,
        declarator: Box<AstNode>,
        parameters: Vec<Box<AstNode>>,
    },
    /// An identifier.
    Identifier(String),
    /// An integer constant.
    IntConstant(i32),
    /// A floating-point constant.
    FloatConstant(f64),
    /// A character constant.
    CharConstant(char),
    /// A string literal.
    StringLiteral(String),
    /// A return statement with an optional expression.
    ReturnStatement(Option<Box<AstNode>>),
    /// A binary operation (e.g., a + b).
    BinaryOp {
        op: BinaryOperator,
        left: Box<AstNode>,
        right: Box<AstNode>,
    },
    /// A unary operation (e.g., !a, -b).
    UnaryOp {
        op: UnaryOperator,
        operand: Box<AstNode>,
    },
    /// An assignment (e.g., a = b).
    Assignment {
        lhs: Box<AstNode>,
        rhs: Box<AstNode>,
    },
    /// A variable declaration (e.g., int x = 5;).
    Declaration {
        type_spec: TypeSpecifier,
        declarator: Box<AstNode>,
        initializer: Option<Box<AstNode>>,
    },
    /// A list of nodes (for compound statements or translation units).
    NodeList(Vec<Box<AstNode>>),
    /// Binary operations (e.g., a + b, a * b)
    BinaryOperation {
        op: String,
        left: Box<AstNode>,
        right: Box<AstNode>,
    },
    /// Unary operations (e.g., !a, -b, ++c)
    UnaryOperation {
        op: String,
        operand: Box<AstNode>,
    },
    /// Ternary/conditional operation (a ? b : c)
    TernaryOperation {
        condition: Box<AstNode>,
        true_expr: Box<AstNode>,
        false_expr: Box<AstNode>,
    },
    /// Array subscript (array[index])
    ArraySubscript {
        array: Box<AstNode>,
        index: Box<AstNode>,
    },
    InitializerList(Vec<Box<AstNode>>),
    ArrayDeclarator {
        base: Box<AstNode>, // Type of the array element (can be nested)
        size: Box<AstNode>,
    },
    /// Function call (func(arg1, arg2))
    FunctionCall {
        function: Box<AstNode>,
        args: Vec<Box<AstNode>>,
    },
    /// A declarator with an optional initializer in a declaration
    InitDeclarator {
        declarator: Box<AstNode>,
        initializer: Option<Box<AstNode>>,
    },
    /// If statement (if condition { then_stmt } else { else_stmt })
    IfStatement {
        condition: Box<AstNode>,
        then_stmt: Box<AstNode>,
        else_stmt: Option<Box<AstNode>>,
    },
    WhileStatement {
        condition: Box<AstNode>,
        body: Box<AstNode>,
    },
    ForLoop {
        init: Box<AstNode>,
        condition: Box<AstNode>,
        increment: Box<AstNode>,
        body: Box<AstNode>,
    },
    /// Expression statement - expression followed by a semicolon
    ExpressionStatement(Box<AstNode>),
    /// Integer literal with a value
    IntegerLiteral(i32),
    /// Block statement (for compound statements)
    BlockStatement(Vec<Box<AstNode>>),
    /// Add TypeSpecifier to hold types
    TypeSpecifier(TypeSpecifier),
    /// Variable declaration
    VariableDeclaration {
        type_spec: TypeSpecifier,
        declarator: Box<AstNode>,
        initializer: Option<Box<AstNode>>,
    },
    /// Struct definition
    StructDefinition {
        name: String,
        fields: Vec<Box<AstNode>>,
    },
    /// Struct field
    StructField {
        type_spec: TypeSpecifier,
        name: String,
    },
    /// Member access for structs
    MemberAccess {
        object: Box<AstNode>,
        member: String,
    },
}

/// Binary operators
#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
    LessThanOrEqual,
    GreaterThanOrEqual,
}

/// Unary operators
#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOperator {
    Negate,
    Not,
}

/// A simple type specifier enum.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeSpecifier {
    Void,
    Char,
    Short,
    Int,
    Long,
    Float,
    Double,
    Struct(String),
}

// Implement Display for TypeSpecifier
impl fmt::Display for TypeSpecifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeSpecifier::Void => write!(f, "void"),
            TypeSpecifier::Char => write!(f, "char"),
            TypeSpecifier::Short => write!(f, "short"),
            TypeSpecifier::Int => write!(f, "int"),
            TypeSpecifier::Long => write!(f, "long"),
            TypeSpecifier::Float => write!(f, "float"),
            TypeSpecifier::Double => write!(f, "double"),
            TypeSpecifier::Struct(name) => write!(f, "struct {}", name),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SwitchCase {
    pub value: Box<AstNode>,
    pub body: Vec<Box<AstNode>>,
}

impl Node for AstNode {
    fn emit_risc(&self, _context: &mut Context) -> String {
        // For now, simply return a debug string.
        format!("Emit RISC for: {:?}", self)
    }

    fn print(&self) -> String {
        // For debugging, print the node structure.
        format!("{:?}", self)
    }
}
