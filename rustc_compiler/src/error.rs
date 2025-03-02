// src/error.rs
#[derive(Debug)]
pub enum CompileError {
    LexerError(String),
    ParserError(String),
    CodegenError(String),
    TypeError(String),
    IOError(String),
}

impl From<String> for CompileError {
    fn from(msg: String) -> Self {
        CompileError::ParserError(msg)
    }
}

impl From<&str> for CompileError {
    fn from(msg: &str) -> Self {
        CompileError::ParserError(msg.to_string())
    }
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::LexerError(msg) => write!(f, "Lexer error: {}", msg),
            CompileError::ParserError(msg) => write!(f, "Parser error: {}", msg),
            CompileError::CodegenError(msg) => write!(f, "Code generation error: {}", msg),
            CompileError::TypeError(msg) => write!(f, "Type error: {}", msg),
            CompileError::IOError(msg) => write!(f, "I/O error: {}", msg),
        }
    }
}

impl std::error::Error for CompileError {}
