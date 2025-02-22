// src/lexer.rs

use logos::Logos;

/// The set of tokens that our C90 lexer will recognize.
///
/// Keyword tokens are suffixed with `Kw` to differentiate them from literal tokens.
#[derive(Logos, Debug, PartialEq)]
pub enum Token {
    // --- Keywords (expanded to match the Flex file) ---
    #[token("auto")]
    AutoKw,
    #[token("break")]
    BreakKw,
    #[token("case")]
    CaseKw,
    #[token("char")]
    CharKw,
    #[token("const")]
    ConstKw,
    #[token("continue")]
    ContinueKw,
    #[token("default")]
    DefaultKw,
    #[token("do")]
    DoKw,
    #[token("double")]
    DoubleKw,
    #[token("else")]
    ElseKw,
    #[token("enum")]
    EnumKw,
    #[token("extern")]
    ExternKw,
    #[token("float")]
    FloatKw,
    #[token("for")]
    ForKw,
    #[token("goto")]
    GotoKw,
    #[token("if")]
    IfKw,
    #[token("int")]
    IntKw,
    #[token("long")]
    LongKw,
    #[token("register")]
    RegisterKw,
    #[token("return")]
    ReturnKw,
    #[token("short")]
    ShortKw,
    #[token("signed")]
    SignedKw,
    #[token("sizeof")]
    SizeofKw,
    #[token("static")]
    StaticKw,
    #[token("struct")]
    StructKw,
    #[token("switch")]
    SwitchKw,
    #[token("typedef")]
    TypedefKw,
    #[token("union")]
    UnionKw,
    #[token("unsigned")]
    UnsignedKw,
    #[token("void")]
    VoidKw,
    #[token("volatile")]
    VolatileKw,
    #[token("while")]
    WhileKw,

    // --- Identifiers ---
    #[regex("[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Identifier(String),


    // --- Numeric Literals ---
    // Hexadecimal literal: e.g. 0x1F
    #[regex("0[xX][0-9a-fA-F]+", |lex| parse_int(lex.slice()))]
    IntLiteral(i64),
    // Octal literal: e.g. 0755 (Note: "0" alone is handled by the decimal rule below)
    #[regex("0[0-7]+", |lex| parse_int(lex.slice()))]
    IntLiteralOct(i64),

    // Floating point literal:
    // Supports: 3.14, .5, 2., 1e10, 3.14E-2, etc., with optional [fFlL] suffix.
    #[regex(r"[0-9]+\.[0-9]*([Ee][+-]?[0-9]+)?[fFlL]?", |lex| parse_float(lex.slice()))]
    #[regex(r"\.[0-9]+([Ee][+-]?[0-9]+)?[fFlL]?", |lex| parse_float(lex.slice()))]
    #[regex(r"[0-9]+([Ee][+-]?[0-9]+)[fFlL]?", |lex| parse_float(lex.slice()))]
    FloatLiteral(f64),

    // Decimal literal: e.g. 123 or 0
    #[regex("[0-9]+", |lex| parse_int(lex.slice()))]
    IntLiteralDec(i64),


    // --- Character and String Literals ---
    // Character literal (supports simple escape sequences)
    #[regex(r"'([^'\\]|\\.)'", |lex| {
        let s = lex.slice();
        // Remove the surrounding single quotes.
        let inner = &s[1..s.len()-1];
        if inner.starts_with('\\') {
            match inner.chars().nth(1).unwrap() {
                'n' => '\n',
                't' => '\t',
                'r' => '\r',
                '\\' => '\\',
                '\'' => '\'',
                '\"' => '\"',
                other => other,
            }
        } else {
            inner.chars().next().unwrap()
        }
    })]
    CharLiteral(char),
    // String literal (does not process escapes beyond stripping the quotes)
    #[regex(r#""([^"\\]|\\.)*""#, |lex| {
        let s = lex.slice();
        s[1..s.len()-1].to_string()
    })]
    StringLiteral(String),

    // --- Punctuation and Operators ---
    #[token("...")]
    Ellipsis,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token(";")]
    Semicolon,
    #[token(",")]
    Comma,
    #[token(".")]
    Dot,
    #[token("->")]
    Arrow,
    #[token("++")]
    PlusPlus,
    #[token("--")]
    MinusMinus,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,
    #[token("&")]
    Ampersand,
    #[token("|")]
    Pipe,
    #[token("^")]
    Caret,
    #[token("!")]
    Bang,
    #[token("~")]
    Tilde,
    #[token("=")]
    Assign,
    #[token("==")]
    EqualEqual,
    #[token("!=")]
    NotEqual,
    #[token("<")]
    Less,
    #[token("<=")]
    LessEqual,
    #[token(">")]
    Greater,
    #[token(">=")]
    GreaterEqual,
    #[token("&&")]
    AndAnd,
    #[token("||")]
    OrOr,
    #[token("?")]
    Question,
    #[token(":")]
    Colon,
    // Compound assignment operators
    #[token("+=")]
    PlusEqual,
    #[token("-=")]
    MinusEqual,
    #[token("*=")]
    StarEqual,
    #[token("/=")]
    SlashEqual,
    #[token("%=")]
    PercentEqual,
    #[token("&=")]
    AmpersandEqual,
    #[token("|=")]
    PipeEqual,
    #[token("^=")]
    CaretEqual,
    #[token("<<")]
    ShiftLeft,
    #[token(">>")]
    ShiftRight,
    #[token("<<=")]
    ShiftLeftEqual,
    #[token(">>=")]
    ShiftRightEqual,

    // --- Whitespace and Comments (skipped) ---
    #[regex(r"[ \t\n\f]+", logos::skip)]
    Whitespace,
    #[regex(r"//[^\n]*", logos::skip)]
    LineComment,
    #[regex(r"/\*([^*]|\*[^/])*\*/", logos::skip)]
    BlockComment,

    // Catch any error
    #[error]
    Error,
}

/// Helper function to parse an integer literal string into an `i64`.
/// This function distinguishes hexadecimal, octal, and decimal forms.
fn parse_int(slice: &str) -> i64 {
    if slice.starts_with("0x") || slice.starts_with("0X") {
        i64::from_str_radix(&slice[2..], 16).unwrap()
    } else if slice.starts_with("0") && slice.len() > 1 {
        i64::from_str_radix(&slice[1..], 8).unwrap()
    } else {
        slice.parse::<i64>().unwrap()
    }
}

fn parse_float(slice: &str) -> f64 {
    // Check if the last character is a suffix we want to remove.
    let trimmed = if let Some(last) = slice.chars().last() {
        if last == 'f' || last == 'F' || last == 'l' || last == 'L' {
            &slice[..slice.len()-1]
        } else {
            slice
        }
    } else {
        slice
    };
    trimmed.parse::<f64>().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use logos::Logos;

    #[test]
    fn test_keywords() {
        let source = "auto break case char const continue default do double else enum extern float for goto if int long register return short signed sizeof static struct switch typedef union unsigned void volatile while";
        let mut lex = Token::lexer(source);
        let expected = [
            Token::AutoKw,
            Token::BreakKw,
            Token::CaseKw,
            Token::CharKw,
            Token::ConstKw,
            Token::ContinueKw,
            Token::DefaultKw,
            Token::DoKw,
            Token::DoubleKw,
            Token::ElseKw,
            Token::EnumKw,
            Token::ExternKw,
            Token::FloatKw,
            Token::ForKw,
            Token::GotoKw,
            Token::IfKw,
            Token::IntKw,
            Token::LongKw,
            Token::RegisterKw,
            Token::ReturnKw,
            Token::ShortKw,
            Token::SignedKw,
            Token::SizeofKw,
            Token::StaticKw,
            Token::StructKw,
            Token::SwitchKw,
            Token::TypedefKw,
            Token::UnionKw,
            Token::UnsignedKw,
            Token::VoidKw,
            Token::VolatileKw,
            Token::WhileKw,
        ];
        for token in expected.iter() {
            assert_eq!(&lex.next().unwrap(), token);
        }
    }

    #[test]
    fn test_identifiers() {
        let source = "variable _temp myVar";
        let mut lex = Token::lexer(source);
        assert_eq!(lex.next(), Some(Token::Identifier("variable".to_string())));
        assert_eq!(lex.next(), Some(Token::Identifier("_temp".to_string())));
        assert_eq!(lex.next(), Some(Token::Identifier("myVar".to_string())));
    }

    #[test]
    fn test_int_literals() {
        let source = "123 0x1F 0755 0";
        let mut lex = Token::lexer(source);
        // Decimal literal 123
        match lex.next().unwrap() {
            Token::IntLiteral(i) | Token::IntLiteralDec(i) => assert_eq!(i, 123),
            other => panic!("Expected int literal, got {:?}", other),
        }
        // Hexadecimal literal 0x1F => 31
        match lex.next().unwrap() {
            Token::IntLiteral(i) => assert_eq!(i, 31),
            other => panic!("Expected int literal, got {:?}", other),
        }
        // Octal literal 0755 => 493
        match lex.next().unwrap() {
            Token::IntLiteralOct(i) | Token::IntLiteralDec(i) => assert_eq!(i, 493),
            other => panic!("Expected int literal, got {:?}", other),
        }
        // Decimal literal 0
        match lex.next().unwrap() {
            Token::IntLiteralDec(i) => assert_eq!(i, 0),
            other => panic!("Expected int literal, got {:?}", other),
        }
    }

    #[test]
    fn test_float_literals() {
        let source = "3.14 .5 2. 1e10 3.14E-2 2.71f 2.86L";
        let mut lex = Token::lexer(source);

        if let Token::FloatLiteral(f) = lex.next().unwrap() {
            println!("Token 1 (expected 3.14): {}", f);
            assert!((f - 3.14).abs() < 1e-6);
        } else {
            panic!("Expected float literal");
        }
        if let Token::FloatLiteral(f) = lex.next().unwrap() {
            println!("Token 2 (expected 0.5): {}", f);
            assert!((f - 0.5).abs() < 1e-6);
        } else {
            panic!("Expected float literal");
        }
        if let Token::FloatLiteral(f) = lex.next().unwrap() {
            println!("Token 3 (expected 2.0): {}", f);
            assert!((f - 2.0).abs() < 1e-6);
        } else {
            panic!("Expected float literal");
        }
        if let Token::FloatLiteral(f) = lex.next().unwrap() {
            println!("Token 4 (expected 1e10): {}", f);
            assert!((f - 1e10).abs() < 1e-2);
        } else {
            panic!("Expected float literal");
        }
        if let Token::FloatLiteral(f) = lex.next().unwrap() {
            println!("Token 5 (expected 3.14E-2): {}", f);
            assert!((f - 3.14E-2).abs() < 1e-6);
        } else {
            panic!("Expected float literal");
        }
        if let Token::FloatLiteral(f) = lex.next().unwrap() {
            println!("Token 6 (expected 2.71): {}", f);
            // Parsing "2.71f" should produce 2.71
            assert!((f - 2.71).abs() < 1e-6);
        } else {
            panic!("Expected float literal");
        }
        if let Token::FloatLiteral(f) = lex.next().unwrap() {
            println!("Token 7 (expected 2.86): {}", f);
            // Parsing "2.86L" should produce 2.86
            assert!((f - 2.86).abs() < 1e-6);
        } else {
            panic!("Expected float literal");
        }
    }

    #[test]
    fn test_string_literal() {
        let source = r#""Hello, world!""#;
        let mut lex = Token::lexer(source);
        if let Token::StringLiteral(s) = lex.next().unwrap() {
            assert_eq!(s, "Hello, world!");
        } else {
            panic!("Expected string literal");
        }
    }

    #[test]
    fn test_char_literal() {
        let source = r#"'a' '\n'"#;
        let mut lex = Token::lexer(source);
        if let Token::CharLiteral(c) = lex.next().unwrap() {
            assert_eq!(c, 'a');
        } else {
            panic!("Expected char literal");
        }
        if let Token::CharLiteral(c) = lex.next().unwrap() {
            assert_eq!(c, '\n');
        } else {
            panic!("Expected char literal");
        }
    }

    #[test]
    fn test_punctuation_and_operators() {
        let source = "( ) { } [ ] ; , . -> ++ -- + - * / % & | ^ ! ~ = == != < <= > >= && || ? : += -= *= /= %= &= |= ^= << >> <<= >>=";
        let mut lex = Token::lexer(source);
        let expected_tokens = [
            Token::LParen, Token::RParen, Token::LBrace, Token::RBrace,
            Token::LBracket, Token::RBracket, Token::Semicolon, Token::Comma,
            Token::Dot, Token::Arrow, Token::PlusPlus, Token::MinusMinus,
            Token::Plus, Token::Minus, Token::Star, Token::Slash, Token::Percent,
            Token::Ampersand, Token::Pipe, Token::Caret, Token::Bang, Token::Tilde,
            Token::Assign, Token::EqualEqual, Token::NotEqual, Token::Less, Token::LessEqual,
            Token::Greater, Token::GreaterEqual, Token::AndAnd, Token::OrOr, Token::Question,
            Token::Colon, Token::PlusEqual, Token::MinusEqual, Token::StarEqual,
            Token::SlashEqual, Token::PercentEqual, Token::AmpersandEqual, Token::PipeEqual,
            Token::CaretEqual, Token::ShiftLeft, Token::ShiftRight, Token::ShiftLeftEqual, Token::ShiftRightEqual,
        ];
        for expected in expected_tokens.iter() {
            assert_eq!(&lex.next().unwrap(), expected);
        }
    }
}
