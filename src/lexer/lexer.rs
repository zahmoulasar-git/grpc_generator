use crate::lexer::token::Token;
use crate::lexer::token_type::TokenType;

#[derive(Debug)]
pub struct Lexer {
    input: String,
    position: usize,
    line: usize,
}

impl Lexer {
    pub fn new(input: String) -> Self {
        Self {
            input,
            position: 0,
            line: 1,
        }
    }

    pub fn peek_char(&self) -> Option<char> {
        self.input.chars().nth(self.position)
    }

    pub fn peek_next_char(&self) -> Option<char> {
        self.input.chars().nth(self.position + 1)
    }

    pub fn advance(&mut self) {
        if self.peek_char() == Some('\n') {
            self.line += 1;
        }
        self.position += 1;
    }

    pub fn current_line(&self) -> usize {
        self.line
    }

    pub fn skip_whitespace(&mut self) {
        loop {
            match self.peek_char() {
                Some(c) if c.is_whitespace() => {
                    self.advance();
                }
                Some('/') if self.peek_next_char() == Some('/') => {
                    // On saute les deux caracteres "//"
                    self.advance();
                    self.advance();
                    // On avance jusqu'a la fin de la ligne ou du fichier
                    while let Some(c) = self.peek_char() {
                        if c == '\n' {
                            break;
                        }
                        self.advance();
                    }
                }
                _ => break,
            }
        }
    }

    pub fn lookup_identifier(word: &str) -> TokenType {
        match word {
            "message" => TokenType::Keyword,
            "service" => TokenType::Keyword,
            "rpc" => TokenType::Keyword,
            "syntax" => TokenType::Keyword,
            "returns" => TokenType::Keyword,
            "enum" => TokenType::Keyword,
            "package" => TokenType::Keyword,
            "import" => TokenType::Keyword,
            "option" => TokenType::Keyword,
            "repeated" => TokenType::Keyword,
            "optional" => TokenType::Keyword,
            "map" => TokenType::Keyword,
            "int32" => TokenType::Keyword,
            "int64" => TokenType::Keyword,
            "uint32" => TokenType::Keyword,
            "uint64" => TokenType::Keyword,
            "bool" => TokenType::Keyword,
            "string" => TokenType::Keyword,
            "bytes" => TokenType::Keyword,
            "float" => TokenType::Keyword,
            "double" => TokenType::Keyword,
            _ => TokenType::Identifier,
        }
    }

    pub fn read_identifier(&mut self) -> Token {
        let mut result = String::new();

        while let Some(c) = self.peek_char() {
            if c.is_alphanumeric() || c == '_' {
                result.push(c);
                self.advance();
            } else {
                break;
            }
        }

        let token_type = Self::lookup_identifier(&result);

        Token {
            token_type,
            value: result,
            line: self.line,
        }
    }

    pub fn read_number(&mut self) -> Token {
        let mut result = String::new();

        while let Some(c) = self.peek_char() {
            if c.is_numeric() {
                result.push(c);
                self.advance();
            } else if c == '.' {
                // On ne consomme le point que s'il est suivi d'un chiffre
                // (sinon c'est un point de nom qualifie, ex: google.protobuf)
                match self.peek_next_char() {
                    Some(next_c) if next_c.is_numeric() => {
                        result.push(c);
                        self.advance();
                    }
                    _ => break,
                }
            } else {
                break;
            }
        }

        Token {
            token_type: TokenType::Number,
            value: result,
                    line: self.line,
        }
    }

    pub fn read_string(&mut self) -> Token {
        self.advance();

        let mut result = String::new();

        while let Some(c) = self.peek_char() {
            if c == '"' {
                break;
            } else {
                result.push(c);
                self.advance();
            }
        }

        self.advance();

        Token {
            token_type: TokenType::StringLiteral,
            value: result,
                    line: self.line,
        }
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();

        match self.peek_char() {
            Some('{') => {
                self.advance();
                Token {
                    token_type: TokenType::LeftBrace,
                    value: "{".to_string(),
                    line: self.line,
                }
            }

            Some('}') => {
                self.advance();
                Token {
                    token_type: TokenType::RightBrace,
                    value: "}".to_string(),
                    line: self.line,
                }
            }

            Some('(') => {
                self.advance();
                Token {
                    token_type: TokenType::LeftParen,
                    value: "(".to_string(),
                    line: self.line,
                }
            }

            Some(')') => {
                self.advance();
                Token {
                    token_type: TokenType::RightParen,
                    value: ")".to_string(),
                    line: self.line,
                }
            }

            Some('=') => {
                self.advance();
                Token {
                    token_type: TokenType::Equal,
                    value: "=".to_string(),
                    line: self.line,
                }
            }

            Some(';') => {
                self.advance();
                Token {
                    token_type: TokenType::Semicolon,
                    value: ";".to_string(),
                    line: self.line,
                }
            }

            Some(',') => {
                self.advance();
                Token {
                    token_type: TokenType::Comma,
                    value: ",".to_string(),
                    line: self.line,
                }
            }

            Some('[') => {
                self.advance();
                Token {
                    token_type: TokenType::LeftBracket,
                    value: "[".to_string(),
                    line: self.line,
                }
            }

            Some(']') => {
                self.advance();
                Token {
                    token_type: TokenType::RightBracket,
                    value: "]".to_string(),
                    line: self.line,
                }
            }

            Some('.') => {
                self.advance();
                Token {
                    token_type: TokenType::Dot,
                    value: ".".to_string(),
                    line: self.line,
                }
            }

            Some('<') => {
                self.advance();
                Token {
                    token_type: TokenType::LessThan,
                    value: "<".to_string(),
                    line: self.line,
                }
            }

            Some('>') => {
                self.advance();
                Token {
                    token_type: TokenType::GreaterThan,
                    value: ">".to_string(),
                    line: self.line,
                }
            }

            Some('"') => {
                self.read_string()
            }

            Some(c) if c.is_alphabetic() || c == '_' => {
                self.read_identifier()
            }

            Some(c) if c.is_numeric() => {
                self.read_number()
            }

            None => Token {
                token_type: TokenType::EndOfFile,
                value: String::new(),
                    line: self.line,
            },

            Some(invalid_char) => {
                let value = invalid_char.to_string();
                self.advance();
                Token {
                    token_type: TokenType::Invalid,
                    value,
                    line: self.line,
                }
            }
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();

        loop {
            let token = self.next_token();
            let is_eof = token.token_type == TokenType::EndOfFile;
            tokens.push(token);
            if is_eof {
                break;
            }
        }

        tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_symbols() {
        let mut lexer = Lexer::new("{ } ( ) = ; , . < >".to_string());
        let tokens = lexer.tokenize();
        assert_eq!(tokens[0].token_type, TokenType::LeftBrace);
        assert_eq!(tokens[1].token_type, TokenType::RightBrace);
        assert_eq!(tokens[8].token_type, TokenType::LessThan);
        assert_eq!(tokens[9].token_type, TokenType::GreaterThan);
    }

    #[test]
    fn test_invalid_character_detected() {
        let mut lexer = Lexer::new("@".to_string());
        let tokens = lexer.tokenize();
        assert_eq!(tokens[0].token_type, TokenType::Invalid);
        assert_eq!(tokens[0].value, "@");
    }

    #[test]
    fn test_keyword_vs_identifier() {
        let mut lexer = Lexer::new("message Person".to_string());
        let tokens = lexer.tokenize();
        assert_eq!(tokens[0].token_type, TokenType::Keyword);
        assert_eq!(tokens[1].token_type, TokenType::Identifier);
    }

    #[test]
    fn test_decimal_number() {
        let mut lexer = Lexer::new("3.14".to_string());
        let tokens = lexer.tokenize();
        assert_eq!(tokens[0].token_type, TokenType::Number);
        assert_eq!(tokens[0].value, "3.14");
    }

    #[test]
    fn test_qualified_name_dot_not_consumed_by_number() {
        let mut lexer = Lexer::new("google.protobuf.Timestamp".to_string());
        let tokens = lexer.tokenize();
        assert_eq!(tokens[0].value, "google");
        assert_eq!(tokens[1].token_type, TokenType::Dot);
    }

    #[test]
    fn test_comment_skipped_and_line_tracking() {
        let mut lexer = Lexer::new("// comment\nmessage".to_string());
        let tokens = lexer.tokenize();
        assert_eq!(tokens[0].token_type, TokenType::Keyword);
        assert_eq!(tokens[0].line, 2);
    }

    #[test]
    fn test_string_literal() {
        let mut lexer = Lexer::new("\"proto3\"".to_string());
        let tokens = lexer.tokenize();
        assert_eq!(tokens[0].token_type, TokenType::StringLiteral);
        assert_eq!(tokens[0].value, "proto3");
    }
}
