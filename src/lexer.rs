use crate::token::{Token, TokenKind};

/// Lexer for the Gello language
pub struct Lexer {
    source: Vec<char>,
    tokens: Vec<Token>,
    start: usize,
    current: usize,
    line: usize,
    col: usize,
    start_col: usize,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Self {
            source: source.chars().collect(),
            tokens: Vec::new(),
            start: 0,
            current: 0,
            line: 1,
            col: 1,
            start_col: 1,
        }
    }

    /// Tokenize the source code into a vector of tokens
    pub fn tokenize(mut self) -> Result<Vec<Token>, String> {
        while !self.is_at_end() {
            self.start = self.current;
            self.start_col = self.col;
            self.scan_token()?;
        }

        self.tokens.push(Token::new(TokenKind::Eof, self.line, self.col));
        Ok(self.tokens)
    }

    fn scan_token(&mut self) -> Result<(), String> {
        let c = self.advance();

        match c {
            // Whitespace
            ' ' | '\r' | '\t' => {}
            '\n' => {
                self.line += 1;
                self.col = 1;
            }

            // Single-character tokens
            '(' => self.add_token(TokenKind::LeftParen),
            ')' => self.add_token(TokenKind::RightParen),
            '{' => self.add_token(TokenKind::LeftBrace),
            '}' => self.add_token(TokenKind::RightBrace),
            '[' => self.add_token(TokenKind::LeftBracket),
            ']' => self.add_token(TokenKind::RightBracket),
            ',' => self.add_token(TokenKind::Comma),
            '+' => self.add_token(TokenKind::Plus),
            '*' => self.add_token(TokenKind::Star),
            '/' => self.add_token(TokenKind::Slash),
            '%' => self.add_token(TokenKind::Percent),

            // Comment or minus
            '-' => {
                if self.match_char('-') {
                    // Line comment: skip until end of line
                    while self.peek() != '\n' && !self.is_at_end() {
                        self.advance();
                    }
                } else {
                    self.add_token(TokenKind::Minus);
                }
            }

            // Two-character tokens
            '!' => {
                if self.match_char('=') {
                    self.add_token(TokenKind::BangEqual);
                } else {
                    self.add_token(TokenKind::Bang);
                }
            }
            '=' => {
                if self.match_char('=') {
                    self.add_token(TokenKind::EqualEqual);
                } else {
                    self.add_token(TokenKind::Equal);
                }
            }
            '<' => {
                if self.match_char('=') {
                    self.add_token(TokenKind::LessEqual);
                } else {
                    self.add_token(TokenKind::Less);
                }
            }
            '>' => {
                if self.match_char('=') {
                    self.add_token(TokenKind::GreaterEqual);
                } else {
                    self.add_token(TokenKind::Greater);
                }
            }
            '&' => {
                if self.match_char('&') {
                    self.add_token(TokenKind::AmpAmp);
                } else {
                    return Err(format!(
                        "Unexpected character '&' at line {}, column {}",
                        self.line, self.start_col
                    ));
                }
            }
            '|' => {
                if self.match_char('|') {
                    self.add_token(TokenKind::PipePipe);
                } else {
                    return Err(format!(
                        "Unexpected character '|' at line {}, column {}",
                        self.line, self.start_col
                    ));
                }
            }

            // String literals
            '"' => self.string()?,

            // Numbers and identifiers
            _ => {
                if c.is_ascii_digit() {
                    self.number()?;
                } else if c.is_alphabetic() || c == '_' {
                    self.identifier();
                } else {
                    return Err(format!(
                        "Unexpected character '{}' at line {}, column {}",
                        c, self.line, self.start_col
                    ));
                }
            }
        }

        Ok(())
    }

    fn string(&mut self) -> Result<(), String> {
        let start_line = self.line;
        let start_col = self.start_col;
        let mut value = String::new();

        while self.peek() != '"' && !self.is_at_end() {
            if self.peek() == '\n' {
                self.line += 1;
                self.col = 1;
            }

            if self.peek() == '\\' {
                self.advance();
                match self.peek() {
                    'n' => {
                        self.advance();
                        value.push('\n');
                    }
                    't' => {
                        self.advance();
                        value.push('\t');
                    }
                    'r' => {
                        self.advance();
                        value.push('\r');
                    }
                    '\\' => {
                        self.advance();
                        value.push('\\');
                    }
                    '"' => {
                        self.advance();
                        value.push('"');
                    }
                    _ => {
                        return Err(format!(
                            "Invalid escape sequence at line {}, column {}",
                            self.line, self.col
                        ));
                    }
                }
            } else {
                value.push(self.advance());
            }
        }

        if self.is_at_end() {
            return Err(format!(
                "Unterminated string starting at line {}, column {}",
                start_line, start_col
            ));
        }

        // Consume closing "
        self.advance();

        self.add_token(TokenKind::String(value));
        Ok(())
    }

    fn number(&mut self) -> Result<(), String> {
        while self.peek().is_ascii_digit() {
            self.advance();
        }

        // Look for decimal part
        if self.peek() == '.' && self.peek_next().is_ascii_digit() {
            // Consume the '.'
            self.advance();

            while self.peek().is_ascii_digit() {
                self.advance();
            }
        }

        let text: String = self.source[self.start..self.current].iter().collect();
        let value: f64 = text.parse().map_err(|_| {
            format!(
                "Invalid number '{}' at line {}, column {}",
                text, self.line, self.start_col
            )
        })?;

        self.add_token(TokenKind::Number(value));
        Ok(())
    }

    fn identifier(&mut self) {
        while self.peek().is_alphanumeric() || self.peek() == '_' {
            self.advance();
        }

        let text: String = self.source[self.start..self.current].iter().collect();

        let kind = match text.as_str() {
            "let" => TokenKind::Let,
            "fn" => TokenKind::Fn,
            "return" => TokenKind::Return,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "print" => TokenKind::Print,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "null" => TokenKind::Null,
            _ => TokenKind::Identifier(text),
        };

        self.add_token(kind);
    }

    fn advance(&mut self) -> char {
        let c = self.source[self.current];
        self.current += 1;
        self.col += 1;
        c
    }

    fn peek(&self) -> char {
        if self.is_at_end() {
            '\0'
        } else {
            self.source[self.current]
        }
    }

    fn peek_next(&self) -> char {
        if self.current + 1 >= self.source.len() {
            '\0'
        } else {
            self.source[self.current + 1]
        }
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.is_at_end() || self.source[self.current] != expected {
            false
        } else {
            self.current += 1;
            self.col += 1;
            true
        }
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }

    fn add_token(&mut self, kind: TokenKind) {
        self.tokens.push(Token::new(kind, self.line, self.start_col));
    }
}
