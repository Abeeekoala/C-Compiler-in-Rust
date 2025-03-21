use std::fmt;

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
    FunctionDefinition {
        decl_specifiers: Vec<Box<AstNode>>,
        declarator: Box<AstNode>,
        parameters: Vec<Box<AstNode>>,
        compound_statement: Box<AstNode>,
    },
    FunctionDeclaration {
        decl_specifiers: Vec<Box<AstNode>>,
        declarator: Box<AstNode>,
        parameters: Vec<Box<AstNode>>,
    },
    Identifier(String),
    IntConstant(i32),
    FloatConstant(f64),
    CharConstant(char),
    StringLiteral(String),
    ReturnStatement(Option<Box<AstNode>>),
    BinaryOp {
        op: BinaryOperator,
        left: Box<AstNode>,
        right: Box<AstNode>,
    },
    UnaryOp {
        op: UnaryOperator,
        operand: Box<AstNode>,
    },
    Assignment {
        lhs: Box<AstNode>,
        rhs: Box<AstNode>,
    },
    Declaration {
        type_spec: TypeSpecifier,
        declarator: Box<AstNode>,
        initializer: Option<Box<AstNode>>,
    },
    NodeList(Vec<Box<AstNode>>),
    BinaryOperation {
        op: String,
        left: Box<AstNode>,
        right: Box<AstNode>,
    },
    UnaryOperation {
        op: String,
        operand: Box<AstNode>,
    },
    TernaryOperation {
        condition: Box<AstNode>,
        true_expr: Box<AstNode>,
        false_expr: Box<AstNode>,
    },
    ArraySubscript {
        array: Box<AstNode>,
        index: Box<AstNode>,
    },
    InitializerList(Vec<Box<AstNode>>),
    ArrayDeclarator {
        base: Box<AstNode>,
        size: Box<AstNode>,
    },
    FunctionCall {
        function: Box<AstNode>,
        args: Vec<Box<AstNode>>,
    },
    InitDeclarator {
        declarator: Box<AstNode>,
        initializer: Option<Box<AstNode>>,
    },
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
    ExpressionStatement(Box<AstNode>),
    IntegerLiteral(i32),
    BlockStatement(Vec<Box<AstNode>>),
    TypeSpecifier(TypeSpecifier),
    VariableDeclaration {
        type_spec: TypeSpecifier,
        declarator: Box<AstNode>,
        initializer: Option<Box<AstNode>>,
    },
    StructDefinition {
        name: String,
        fields: Vec<Box<AstNode>>,
    },
    StructField {
        type_spec: TypeSpecifier,
        name: String,
    },
    MemberAccess {
        object: Box<AstNode>,
        member: String,
    },
    PointerDeclarator {
        pointee: Box<AstNode>,
    },
    SizeofType {
        type_spec: TypeSpecifier,
        pointer_level: usize,
    },
    SizeofExpr {
        expr: Box<AstNode>,
    },
    EnumDefinition {
        name: String,
        values: Vec<(String, i32)>,
    },
    TypedefDeclaration {
        type_spec: TypeSpecifier,
        declarator: Box<AstNode>,
    },
}

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

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOperator {
    Negate,
    Not,
}

/// Type specifier enum.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeSpecifier {
    Void,
    Char,
    Int,
    Unsigned,
    Float,
    Double,
    Struct(String),
    Enum(String),
    TypedefName(String),
}

impl fmt::Display for TypeSpecifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeSpecifier::Void => write!(f, "void"),
            TypeSpecifier::Char => write!(f, "char"),
            TypeSpecifier::Int => write!(f, "int"),
            TypeSpecifier::Unsigned => write!(f, "unsigned"),
            TypeSpecifier::Float => write!(f, "float"),
            TypeSpecifier::Double => write!(f, "double"),
            TypeSpecifier::Struct(name) => write!(f, "struct {}", name),
            TypeSpecifier::Enum(name) => write!(f, "enum {}", name),
            TypeSpecifier::TypedefName(name) => write!(f, "{}", name),
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
        format!("Emit RISC for: {:?}", self)
    }

    fn print(&self) -> String {
        format!("{:?}", self)
    }
}
