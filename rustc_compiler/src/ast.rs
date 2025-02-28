// src/ast.rs

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
#[derive(Debug)]
pub enum AstNode {
    /// A function definition (e.g. int main() { ... }).
    FunctionDefinition {
        decl_specifiers: TypeSpecifier,
        declarator: Box<AstNode>,
        compound_statement: Box<AstNode>,
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
    NodeList(Vec<AstNode>),
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
}

/// Binary operators
#[derive(Debug)]
pub enum BinaryOperator {
    Add, Sub, Mul, Div, Mod,
    BitAnd, BitOr, BitXor,
    LogicAnd, LogicOr,
    Equal, NotEqual,
    Less, LessEqual, Greater, GreaterEqual,
}

/// Unary operators
#[derive(Debug)]
pub enum UnaryOperator {
    Negate, // -
    Not,    // !
    BitNot, // ~
}

/// A simple type specifier enum.
#[derive(Debug, PartialEq)]
pub enum TypeSpecifier {
    Void,
    Int,
    Float,
    Double,
    Char,
    // Add other types as needed
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
