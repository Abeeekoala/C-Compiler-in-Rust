use logos::Logos;
use crate::error::CompileError;

/// Keyword tokens are suffixed with Kw to differentiate them from the literal tokens
#[derive(Logos, Debug, PartialEq, Clone)]
pub enum Token {
    // Keywords
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

    // Identifiers
    #[regex("[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Identifier(String),

    // Numeric Literals
    #[regex("0[xX][0-9a-fA-F]+[uUlL]*", |lex| parse_int(lex.slice()))]
    IntLiteralHex((i64, Option<String>)),
    #[regex("0[0-7]+[uUlL]*", |lex| parse_int(lex.slice()))]
    IntLiteralOct((i64, Option<String>)),
    #[regex("[0-9]+[uUlL]*", |lex| parse_int(lex.slice()))]
    IntLiteralDec((i64, Option<String>)),

    #[regex(r"'([^'\\]|\\.)'", parse_char_as_int)]
    IntLiteralChar(i64),

    // Floating point literal
    #[regex(r"[0-9]+\.[0-9]*([Ee][+-]?[0-9]+)?[fFlL]?", |lex| parse_float(lex.slice()))]
    #[regex(r"\.[0-9]+([Ee][+-]?[0-9]+)?[fFlL]?", |lex| parse_float(lex.slice()))]
    #[regex(r"[0-9]+([Ee][+-]?[0-9]+)[fFlL]?", |lex| parse_float(lex.slice()))]
    FloatLiteral((f64, Option<String>)),

    // String literal
    #[regex(r#""([^"\\]|\\.)*""#, |lex| {
        let s = lex.slice();
        let inner = &s[1..s.len() - 1]; // Remove quotes
        let mut result = String::new();
        let mut chars = inner.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\\' {
                if let Some(next) = chars.next() {
                    match next {
                        'n' => result.push('\n'),
                        't' => result.push('\t'),
                        'r' => result.push('\r'),
                        '\\' => result.push('\\'),
                        '\'' => result.push('\''),
                        '\"' => result.push('\"'),
                        _ => {
                            result.push('\\');
                            result.push(next);
                        }
                    }
                }
            } else {
                result.push(c);
            }
        }
        result
    })]
    StringLiteral(String),

    // --- Punctuation and Operators ---
    #[token("...")]
    Ellipsis,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("<%")]
    #[token("{")]
    LBrace,
    #[token("%>")]
    #[token("}")]
    RBrace,
    #[token("<:")]
    #[token("[")]
    LBracket,
    #[token(":>")]
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
    Add,
    #[token("-")]
    Sub,
    #[token("*")]
    Mul,
    #[token("/")]
    Div,
    #[token("%")]
    Mod,
    #[token("&")]
    BitAnd,
    #[token("|")]
    BitOr,
    #[token("^")]
    BitXor,
    #[token("!")]
    Not,
    #[token("~")]
    Tilde,
    #[token("=")]
    Assign,
    #[token("==")]
    Equal,
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
    LogicAnd,
    #[token("||")]
    LogicOr,
    #[token("?")]
    Question,
    #[token(":")]
    Colon,
    // Compound assignment operators
    #[token("+=")]
    AddAssign,
    #[token("-=")]
    SubAssign,
    #[token("*=")]
    MulAssign,
    #[token("/=")]
    DivAssign,
    #[token("%=")]
    ModAssign,
    #[token("&=")]
    AndAssign,
    #[token("|=")]
    OrAssign,
    #[token("^=")]
    XorAssign,
    #[token("<<")]
    ShiftLeft,
    #[token(">>")]
    ShiftRight,
    #[token("<<=")]
    LeftAssign,
    #[token(">>=")]
    RightAssign,

    // Whitespace and Comments not implemented yet
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
fn split_numeric_and_suffix(slice: &str) -> (&str, &str) {
    let suffix_chars = |c: char| matches!(c, 'u' | 'U' | 'l' | 'L');
    let numeric_len = slice.trim_end_matches(suffix_chars).len();
    let numeric = &slice[..numeric_len];
    let suffix = &slice[numeric_len..];
    (numeric, suffix)
}

fn parse_int(slice: &str) -> (i64, Option<String>) {
    let (numeric, suffix) = split_numeric_and_suffix(slice);
    let value = if numeric.starts_with("0x") || numeric.starts_with("0X") {
        i64::from_str_radix(&numeric[2..], 16).unwrap()
    } else if numeric.starts_with("0") && numeric.len() > 1 {
        i64::from_str_radix(&numeric[1..], 8).unwrap()
    } else {
        numeric.parse::<i64>().unwrap()
    };
    let suffix = if suffix.is_empty() {
        None
    } else {
        Some(suffix.to_string())
    };
    (value, suffix)
}

fn parse_char_as_int(lex: &logos::Lexer<Token>) -> i64 {
    let s = lex.slice();
    let inner = &s[1..s.len() - 1];
    if inner.starts_with('\\') {
        match inner.chars().nth(1).unwrap() {
            'n' => '\n' as i64,
            't' => '\t' as i64,
            'r' => '\r' as i64,
            '\\' => '\\' as i64,
            '\'' => '\'' as i64,
            '\"' => '\"' as i64,
            other => other as i64,
        }
    } else {
        inner.chars().next().unwrap() as i64
    }
}

fn parse_float(slice: &str) -> (f64, Option<String>) {
    let mut numeric = slice;
    let mut suffix = None;
    if let Some(last) = slice.chars().last() {
        if matches!(last, 'f' | 'F' | 'l' | 'L') {
            numeric = &slice[..slice.len() - 1];
            suffix = Some(last.to_string());
        }
    }
    let value = numeric.parse::<f64>().unwrap();
    (value, suffix)
}

/// String of tokens into a vector of tokens.
pub fn tokenize(source: &str) -> Result<Vec<Token>, CompileError> {
    let mut lexer = Token::lexer(source);
    let mut tokens = Vec::new();

    while let Some(token) = lexer.next() {
        match token {
            Token::Error => return Err(CompileError::LexerError(format!("Lexical error at position {}", lexer.span().start))),
            _ => tokens.push(token),
        }
    }

    Ok(tokens)
}

// Unit tests
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
    fn test_int_literals_with_suffixes() {
        let source = "123 123u 123l 123ul 0x1F 0x1Fu 0x1FL 0x1FuL 0755 0755u 0755L 0 0u 0L";
        let mut lex = Token::lexer(source);

        // Decimal literal 123, no suffix
        if let Token::IntLiteralDec((value, suffix)) = lex.next().unwrap() {
            assert_eq!(value, 123);
            assert_eq!(suffix, None);
        } else {
            panic!("Expected IntLiteralDec");
        }

        // Decimal literal 123u
        if let Token::IntLiteralDec((value, suffix)) = lex.next().unwrap() {
            assert_eq!(value, 123);
            assert_eq!(suffix, Some("u".to_string()));
        } else {
            panic!("Expected IntLiteralDec with suffix 'u'");
        }

        // Decimal literal 123l
        if let Token::IntLiteralDec((value, suffix)) = lex.next().unwrap() {
            assert_eq!(value, 123);
            assert_eq!(suffix, Some("l".to_string()));
        } else {
            panic!("Expected IntLiteralDec with suffix 'l'");
        }

        // Decimal literal 123ul
        if let Token::IntLiteralDec((value, suffix)) = lex.next().unwrap() {
            assert_eq!(value, 123);
            assert_eq!(suffix, Some("ul".to_string()));
        } else {
            panic!("Expected IntLiteralDec with suffix 'ul'");
        }

        // Hexadecimal literal 0x1F, no suffix
        if let Token::IntLiteralHex((value, suffix)) = lex.next().unwrap() {
            assert_eq!(value, 31);
            assert_eq!(suffix, None);
        } else {
            panic!("Expected IntLiteralHex");
        }

        // Hexadecimal literal 0x1Fu
        if let Token::IntLiteralHex((value, suffix)) = lex.next().unwrap() {
            assert_eq!(value, 31);
            assert_eq!(suffix, Some("u".to_string()));
        } else {
            panic!("Expected IntLiteralHex with suffix 'u'");
        }

        // Hexadecimal literal 0x1FL
        if let Token::IntLiteralHex((value, suffix)) = lex.next().unwrap() {
            assert_eq!(value, 31);
            assert_eq!(suffix, Some("L".to_string()));
        } else {
            panic!("Expected IntLiteralHex with suffix 'L'");
        }

        // Hexadecimal literal 0x1FuL
        if let Token::IntLiteralHex((value, suffix)) = lex.next().unwrap() {
            assert_eq!(value, 31);
            assert_eq!(suffix, Some("uL".to_string()));
        } else {
            panic!("Expected IntLiteralHex with suffix 'uL'");
        }

        // Octal literal 0755, no suffix
        if let Token::IntLiteralOct((value, suffix)) = lex.next().unwrap() {
            assert_eq!(value, 493);
            assert_eq!(suffix, None);
        } else {
            panic!("Expected IntLiteralOct");
        }

        // Octal literal 0755u
        if let Token::IntLiteralOct((value, suffix)) = lex.next().unwrap() {
            assert_eq!(value, 493);
            assert_eq!(suffix, Some("u".to_string()));
        } else {
            panic!("Expected IntLiteralOct with suffix 'u'");
        }

        // Octal literal 0755L
        if let Token::IntLiteralOct((value, suffix)) = lex.next().unwrap() {
            assert_eq!(value, 493);
            assert_eq!(suffix, Some("L".to_string()));
        } else {
            panic!("Expected IntLiteralOct with suffix 'L'");
        }

        // Decimal literal 0, no suffix
        if let Token::IntLiteralDec((value, suffix)) = lex.next().unwrap() {
            assert_eq!(value, 0);
            assert_eq!(suffix, None);
        } else {
            panic!("Expected IntLiteralDec");
        }

        // Decimal literal 0u
        if let Token::IntLiteralDec((value, suffix)) = lex.next().unwrap() {
            assert_eq!(value, 0);
            assert_eq!(suffix, Some("u".to_string()));
        } else {
            panic!("Expected IntLiteralDec with suffix 'u'");
        }

        // Decimal literal 0L
        if let Token::IntLiteralDec((value, suffix)) = lex.next().unwrap() {
            assert_eq!(value, 0);
            assert_eq!(suffix, Some("L".to_string()));
        } else {
            panic!("Expected IntLiteralDec with suffix 'L'");
        }
    }

    #[test]
    fn test_float_literals_with_suffixes() {
        let source = "3.14 3.14f 3.14L .5 .5f .5L 2. 2.f 2.L 1e10 1e10f 1e10L 3.14E-2 3.14E-2f 3.14E-2L";
        let mut lex = Token::lexer(source);

        // Float literal 3.14, no suffix
        if let Token::FloatLiteral((value, suffix)) = lex.next().unwrap() {
            assert!((value - 3.14).abs() < 1e-6);
            assert_eq!(suffix, None);
        } else {
            panic!("Expected FloatLiteral");
        }

        // Float literal 3.14f
        if let Token::FloatLiteral((value, suffix)) = lex.next().unwrap() {
            assert!((value - 3.14).abs() < 1e-6);
            assert_eq!(suffix, Some("f".to_string()));
        } else {
            panic!("Expected FloatLiteral with suffix 'f'");
        }

        // Float literal 3.14L
        if let Token::FloatLiteral((value, suffix)) = lex.next().unwrap() {
            assert!((value - 3.14).abs() < 1e-6);
            assert_eq!(suffix, Some("L".to_string()));
        } else {
            panic!("Expected FloatLiteral with suffix 'L'");
        }

        // Float literal .5, no suffix
        if let Token::FloatLiteral((value, suffix)) = lex.next().unwrap() {
            assert!((value - 0.5).abs() < 1e-6);
            assert_eq!(suffix, None);
        } else {
            panic!("Expected FloatLiteral");
        }

        // Float literal .5f
        if let Token::FloatLiteral((value, suffix)) = lex.next().unwrap() {
            assert!((value - 0.5).abs() < 1e-6);
            assert_eq!(suffix, Some("f".to_string()));
        } else {
            panic!("Expected FloatLiteral with suffix 'f'");
        }

        // Float literal .5L
        if let Token::FloatLiteral((value, suffix)) = lex.next().unwrap() {
            assert!((value - 0.5).abs() < 1e-6);
            assert_eq!(suffix, Some("L".to_string()));
        } else {
            panic!("Expected FloatLiteral with suffix 'L'");
        }

        // Float literal 2., no suffix
        if let Token::FloatLiteral((value, suffix)) = lex.next().unwrap() {
            assert!((value - 2.0).abs() < 1e-6);
            assert_eq!(suffix, None);
        } else {
            panic!("Expected FloatLiteral");
        }

        // Float literal 2.f
        if let Token::FloatLiteral((value, suffix)) = lex.next().unwrap() {
            assert!((value - 2.0).abs() < 1e-6);
            assert_eq!(suffix, Some("f".to_string()));
        } else {
            panic!("Expected FloatLiteral with suffix 'f'");
        }

        // Float literal 2.L
        if let Token::FloatLiteral((value, suffix)) = lex.next().unwrap() {
            assert!((value - 2.0).abs() < 1e-6);
            assert_eq!(suffix, Some("L".to_string()));
        } else {
            panic!("Expected FloatLiteral with suffix 'L'");
        }

        // Float literal 1e10, no suffix
        if let Token::FloatLiteral((value, suffix)) = lex.next().unwrap() {
            assert!((value - 1e10).abs() < 1e-2);
            assert_eq!(suffix, None);
        } else {
            panic!("Expected FloatLiteral");
        }

        // Float literal 1e10f
        if let Token::FloatLiteral((value, suffix)) = lex.next().unwrap() {
            assert!((value - 1e10).abs() < 1e-2);
            assert_eq!(suffix, Some("f".to_string()));
        } else {
            panic!("Expected FloatLiteral with suffix 'f'");
        }

        // Float literal 1e10L
        if let Token::FloatLiteral((value, suffix)) = lex.next().unwrap() {
            assert!((value - 1e10).abs() < 1e-2);
            assert_eq!(suffix, Some("L".to_string()));
        } else {
            panic!("Expected FloatLiteral with suffix 'L'");
        }

        // Float literal 3.14E-2, no suffix
        if let Token::FloatLiteral((value, suffix)) = lex.next().unwrap() {
            assert!((value - 3.14E-2).abs() < 1e-6);
            assert_eq!(suffix, None);
        } else {
            panic!("Expected FloatLiteral");
        }

        // Float literal 3.14E-2f
        if let Token::FloatLiteral((value, suffix)) = lex.next().unwrap() {
            assert!((value - 3.14E-2).abs() < 1e-6);
            assert_eq!(suffix, Some("f".to_string()));
        } else {
            panic!("Expected FloatLiteral with suffix 'f'");
        }

        // Float literal 3.14E-2L
        if let Token::FloatLiteral((value, suffix)) = lex.next().unwrap() {
            assert!((value - 3.14E-2).abs() < 1e-6);
            assert_eq!(suffix, Some("L".to_string()));
        } else {
            panic!("Expected FloatLiteral with suffix 'L'");
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
    fn test_punctuation_and_operators() {
        let source = "( ) { <% } %> [ <: ] :> ; , . -> ++ -- + - * / % & | ^ ! ~ = == != < <= > >= && || ? : += -= *= /= %= &= |= ^= << >> <<= >>=";
        let mut lex = Token::lexer(source);
        let expected_tokens = [
            Token::LParen, Token::RParen, Token::LBrace, Token::LBrace, Token::RBrace, Token::RBrace,
            Token::LBracket, Token::LBracket, Token::RBracket, Token::RBracket, Token::Semicolon, Token::Comma,
            Token::Dot, Token::Arrow, Token::PlusPlus, Token::MinusMinus,
            Token::Add, Token::Sub, Token::Mul, Token::Div, Token::Mod,
            Token::BitAnd, Token::BitOr, Token::BitXor, Token::Not, Token::Tilde,
            Token::Assign, Token::Equal, Token::NotEqual, Token::Less, Token::LessEqual,
            Token::Greater, Token::GreaterEqual, Token::LogicAnd, Token::LogicOr, Token::Question,
            Token::Colon, Token::AddAssign, Token::SubAssign, Token::MulAssign,
            Token::DivAssign, Token::ModAssign, Token::AndAssign, Token::OrAssign,
            Token::XorAssign, Token::ShiftLeft, Token::ShiftRight, Token::LeftAssign, Token::RightAssign,
        ];
        for expected in expected_tokens.iter() {
            assert_eq!(&lex.next().unwrap(), expected);
        }
    }
}
