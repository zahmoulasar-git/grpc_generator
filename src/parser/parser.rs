use crate::lexer::token::Token;
use crate::lexer::token_type::TokenType;
use crate::parser::ast::{Field, Message, Enum, EnumValue, RpcMethod, Service, ProtoFile};
use crate::parser::error::ParseError;

pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    pub fn peek(&self) -> &Token {
        &self.tokens[self.position]
    }

    // advance() ne panique plus jamais : si on depasse la fin,
    // on reste bloque sur le dernier token (normalement EndOfFile)
    pub fn advance(&mut self) -> Token {
        if self.position < self.tokens.len() - 1 {
            let token = self.tokens[self.position].clone();
            self.position += 1;
            token
        } else {
            self.tokens[self.tokens.len() - 1].clone()
        }
    }

    fn expect_value(&mut self, expected_value: &str, context: &str) -> Result<Token, ParseError> {
        let token = self.advance();
        if token.value == expected_value {
            Ok(token)
        } else {
            Err(ParseError::new(
                "SyntaxError",
                token.line,
                &format!("{} : attendu '{}', trouve '{}'", context, expected_value, token.value),
            ))
        }
    }

    fn expect_type(&mut self, expected: TokenType, context: &str) -> Result<Token, ParseError> {
        let token = self.advance();
        if token.token_type == expected {
            Ok(token)
        } else {
            Err(ParseError::new(
                "SyntaxError",
                token.line,
                &format!("{} : token inattendu '{}'", context, token.value),
            ))
        }
    }

    // Parse une ligne du type: string name = 1;  ou  repeated string tags = 3;
    pub fn parse_field(&mut self) -> Result<Field, ParseError> {
        let mut repeated = false;

        if self.peek().value == "repeated" {
            self.advance();
            repeated = true;
        }

        if self.peek().token_type == TokenType::EndOfFile {
            return Err(ParseError::new(
                "UnexpectedEOF",
                self.peek().line,
                "Fin de fichier inattendue : type de champ attendu",
            ));
        }
        let type_token = self.advance();
        let field_type = type_token.value;

        let name_token = self.expect_type(TokenType::Identifier, "Nom de champ")?;
        let name = name_token.value;

        self.expect_value("=", "Champ")?;

        let number_token = self.advance();
        let number: i64 = number_token.value.parse().map_err(|_| {
            ParseError::new(
                "InvalidNumber",
                number_token.line,
                &format!("Numero de champ invalide : '{}'", number_token.value),
            )
        })?;

        self.expect_value(";", "Champ")?;

        Ok(Field {
            field_type,
            name,
            number,
            repeated,
        })
    }

    // Parse un bloc complet: message Person { ... }
    pub fn parse_message(&mut self) -> Result<Message, ParseError> {
        self.expect_value("message", "Message")?;

        let name_token = self.expect_type(TokenType::Identifier, "Nom du message")?;
        let name = name_token.value;

        self.expect_value("{", "Message")?;

        let mut fields = Vec::new();

        while self.peek().token_type != TokenType::RightBrace {
            if self.peek().token_type == TokenType::EndOfFile {
                return Err(ParseError::new(
                    "UnexpectedEOF",
                    self.peek().line,
                    "Fin de fichier inattendue : '}' attendu pour fermer le message",
                ));
            }
            let field = self.parse_field()?;
            fields.push(field);
        }

        self.expect_value("}", "Message")?;

        Ok(Message { name, fields })
    }

    // Parse un bloc complet: enum Status { UNKNOWN = 0; ACTIVE = 1; }
    pub fn parse_enum(&mut self) -> Result<Enum, ParseError> {
        self.expect_value("enum", "Enum")?;

        let name_token = self.expect_type(TokenType::Identifier, "Nom de l'enum")?;
        let name = name_token.value;

        self.expect_value("{", "Enum")?;

        let mut values = Vec::new();

        while self.peek().token_type != TokenType::RightBrace {
            if self.peek().token_type == TokenType::EndOfFile {
                return Err(ParseError::new(
                    "UnexpectedEOF",
                    self.peek().line,
                    "Fin de fichier inattendue : '}' attendu pour fermer l'enum",
                ));
            }

            let value_name_token = self.expect_type(TokenType::Identifier, "Nom de la valeur enum")?;
            let value_name = value_name_token.value;

            self.expect_value("=", "Valeur enum")?;

            let number_token = self.advance();
            let number: i64 = number_token.value.parse().map_err(|_| {
                ParseError::new(
                    "InvalidNumber",
                    number_token.line,
                    &format!("Numero d'enum invalide : '{}'", number_token.value),
                )
            })?;

            self.expect_value(";", "Valeur enum")?;

            values.push(EnumValue {
                name: value_name,
                number,
            });
        }

        self.expect_value("}", "Enum")?;

        Ok(Enum { name, values })
    }

    // Parse une ligne du type: rpc GetUser(GetUserRequest) returns (GetUserResponse);
    pub fn parse_rpc_method(&mut self) -> Result<RpcMethod, ParseError> {
        self.expect_value("rpc", "Methode RPC")?;

        let name_token = self.expect_type(TokenType::Identifier, "Nom de la methode RPC")?;
        let name = name_token.value;

        self.expect_value("(", "Methode RPC")?;
        let request_token = self.expect_type(TokenType::Identifier, "Type de requete")?;
        let request_type = request_token.value;
        self.expect_value(")", "Methode RPC")?;

        self.expect_value("returns", "Methode RPC")?;

        self.expect_value("(", "Methode RPC")?;
        let response_token = self.expect_type(TokenType::Identifier, "Type de reponse")?;
        let response_type = response_token.value;
        self.expect_value(")", "Methode RPC")?;

        self.expect_value(";", "Methode RPC")?;

        Ok(RpcMethod {
            name,
            request_type,
            response_type,
        })
    }

    // Parse un bloc complet: service UserService { rpc ...; }
    pub fn parse_service(&mut self) -> Result<Service, ParseError> {
        self.expect_value("service", "Service")?;

        let name_token = self.expect_type(TokenType::Identifier, "Nom du service")?;
        let name = name_token.value;

        self.expect_value("{", "Service")?;

        let mut methods = Vec::new();

        while self.peek().token_type != TokenType::RightBrace {
            if self.peek().token_type == TokenType::EndOfFile {
                return Err(ParseError::new(
                    "UnexpectedEOF",
                    self.peek().line,
                    "Fin de fichier inattendue : '}' attendu pour fermer le service",
                ));
            }
            let method = self.parse_rpc_method()?;
            methods.push(method);
        }

        self.expect_value("}", "Service")?;

        Ok(Service { name, methods })
    }

    // Parse le fichier .proto complet : syntax + messages + enums + services, dans n'importe quel ordre
    pub fn parse_proto_file(&mut self) -> Result<ProtoFile, ParseError> {
        let mut proto_file = ProtoFile::new();

        while self.peek().token_type != TokenType::EndOfFile {
            match self.peek().value.as_str() {
                "syntax" => {
                    self.advance(); // "syntax"
                    self.expect_value("=", "Syntax")?;
                    let value_token = self.expect_type(TokenType::StringLiteral, "Valeur de syntax")?;
                    proto_file.syntax = value_token.value;
                    self.expect_value(";", "Syntax")?;
                }
                "message" => {
                    let message = self.parse_message()?;
                    proto_file.messages.push(message);
                }
                "enum" => {
                    let enum_def = self.parse_enum()?;
                    proto_file.enums.push(enum_def);
                }
                "service" => {
                    let service = self.parse_service()?;
                    proto_file.services.push(service);
                }
                other => {
                    return Err(ParseError::new(
                        "SyntaxError",
                        self.peek().line,
                        &format!("Element de niveau superieur inattendu : '{}'", other),
                    ));
                }
            }
        }

        Ok(proto_file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lexer::Lexer;

    fn tokens_from(input: &str) -> Vec<Token> {
        let mut lexer = Lexer::new(input.to_string());
        lexer.tokenize()
    }

    #[test]
    fn test_parse_field_simple() {
        let tokens = tokens_from("string name = 1;");
        let mut parser = Parser::new(tokens);
        let field = parser.parse_field().unwrap();
        assert_eq!(field.field_type, "string");
        assert_eq!(field.name, "name");
        assert_eq!(field.number, 1);
        assert_eq!(field.repeated, false);
    }

    #[test]
    fn test_parse_field_repeated() {
        let tokens = tokens_from("repeated string tags = 3;");
        let mut parser = Parser::new(tokens);
        let field = parser.parse_field().unwrap();
        assert_eq!(field.repeated, true);
    }

    #[test]
    fn test_parse_message() {
        let tokens = tokens_from("message Person { string name = 1; int32 age = 2; }");
        let mut parser = Parser::new(tokens);
        let message = parser.parse_message().unwrap();
        assert_eq!(message.name, "Person");
        assert_eq!(message.fields.len(), 2);
    }

    #[test]
    fn test_parse_enum() {
        let tokens = tokens_from("enum Status { UNKNOWN = 0; ACTIVE = 1; }");
        let mut parser = Parser::new(tokens);
        let e = parser.parse_enum().unwrap();
        assert_eq!(e.name, "Status");
        assert_eq!(e.values.len(), 2);
    }

    #[test]
    fn test_parse_service() {
        let tokens = tokens_from("service UserService { rpc GetUser(GetUserRequest) returns (GetUserResponse); }");
        let mut parser = Parser::new(tokens);
        let s = parser.parse_service().unwrap();
        assert_eq!(s.name, "UserService");
        assert_eq!(s.methods.len(), 1);
        assert_eq!(s.methods[0].name, "GetUser");
    }

    #[test]
    fn test_parse_proto_file_full() {
        let input = "syntax = \"proto3\"; message Person { string name = 1; } enum Status { A = 0; } service S { rpc M(Person) returns (Person); }";
        let tokens = tokens_from(input);
        let mut parser = Parser::new(tokens);
        let proto_file = parser.parse_proto_file().unwrap();
        assert_eq!(proto_file.syntax, "proto3");
        assert_eq!(proto_file.messages.len(), 1);
        assert_eq!(proto_file.enums.len(), 1);
        assert_eq!(proto_file.services.len(), 1);
    }

    #[test]
    fn test_parse_error_on_malformed_field() {
        let tokens = tokens_from("message Broken { string name 1; }");
        let mut parser = Parser::new(tokens);
        let result = parser.parse_message();
        assert!(result.is_err());
    }
}
