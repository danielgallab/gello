/// All possible token kinds in the Gello language
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    Number(f64),
    String(String),
    Identifier(String),

    // Keywords
    Let,
    Fn,
    Return,
    If,
    Else,
    While,
    Print,
    True,
    False,
    Null,

    // Operators
    Plus,        // +
    Minus,       // -
    Star,        // *
    Slash,       // /
    Percent,     // %
    Bang,        // !
    BangEqual,   // !=
    Equal,       // =
    EqualEqual,  // ==
    Less,        // <
    LessEqual,   // <=
    Greater,     // >
    GreaterEqual,// >=
    AmpAmp,      // &&
    PipePipe,    // ||

    // Delimiters
    LeftParen,    // (
    RightParen,   // )
    LeftBrace,    // {
    RightBrace,   // }
    LeftBracket,  // [
    RightBracket, // ]
    Comma,        // ,

    // Special
    Eof,
}

/// A token with its kind and source location
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub col: usize,
}

impl Token {
    pub fn new(kind: TokenKind, line: usize, col: usize) -> Self {
        Self { kind, line, col }
    }
}