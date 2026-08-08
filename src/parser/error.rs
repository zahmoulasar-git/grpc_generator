#[derive(Debug)]
pub struct ParseError {
    pub error_type: String,
    pub line: usize,
    pub description: String,
}

impl ParseError {
    pub fn new(error_type: &str, line: usize, description: &str) -> Self {
        Self {
            error_type: error_type.to_string(),
            line,
            description: description.to_string(),
        }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "[{}] ligne {} : {}", self.error_type, self.line, self.description)
    }
}
