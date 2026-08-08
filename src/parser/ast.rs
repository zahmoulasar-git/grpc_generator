use serde::Serialize;

#[derive(Debug, PartialEq, Serialize)]
pub struct Field {
    pub field_type: String,
    pub name: String,
    pub number: i64,
    pub repeated: bool,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Message {
    pub name: String,
    pub fields: Vec<Field>,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct EnumValue {
    pub name: String,
    pub number: i64,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Enum {
    pub name: String,
    pub values: Vec<EnumValue>,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct RpcMethod {
    pub name: String,
    pub request_type: String,
    pub response_type: String,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Service {
    pub name: String,
    pub methods: Vec<RpcMethod>,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct ProtoFile {
    pub syntax: String,
    pub messages: Vec<Message>,
    pub enums: Vec<Enum>,
    pub services: Vec<Service>,
}

impl ProtoFile {
    pub fn new() -> Self {
        Self {
            syntax: String::new(),
            messages: Vec::new(),
            enums: Vec::new(),
            services: Vec::new(),
        }
    }

    // Parcourt l'arbre dans l'ordre : syntax -> messages (avec leurs champs)
    // -> enums (avec leurs valeurs) -> services (avec leurs methodes).
    pub fn traverse(&self) -> Vec<String> {
        let mut lines = Vec::new();

        lines.push(format!("ProtoFile (syntax = {})", self.syntax));

        for message in &self.messages {
            lines.push(format!("  Message: {}", message.name));
            for field in &message.fields {
                lines.push(format!(
                    "    Field: {} {} = {}{}",
                    field.field_type,
                    field.name,
                    field.number,
                    if field.repeated { " (repeated)" } else { "" }
                ));
            }
        }

        for enum_def in &self.enums {
            lines.push(format!("  Enum: {}", enum_def.name));
            for value in &enum_def.values {
                lines.push(format!("    Value: {} = {}", value.name, value.number));
            }
        }

        for service in &self.services {
            lines.push(format!("  Service: {}", service.name));
            for method in &service.methods {
                lines.push(format!(
                    "    Rpc: {}({}) returns ({})",
                    method.name, method.request_type, method.response_type
                ));
            }
        }

        lines
    }

    // Exporte l'AST au format JSON, indente pour etre lisible.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("Erreur de serialisation JSON")
    }
}

#[derive(Debug, PartialEq)]
pub struct ValidationError {
    pub description: String,
}

impl ProtoFile {
    pub fn validate(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        for i in 0..self.messages.len() {
            for j in (i + 1)..self.messages.len() {
                if self.messages[i].name == self.messages[j].name {
                    errors.push(ValidationError {
                        description: format!("Message duplique : '{}'", self.messages[i].name),
                    });
                }
            }
        }

        for i in 0..self.enums.len() {
            for j in (i + 1)..self.enums.len() {
                if self.enums[i].name == self.enums[j].name {
                    errors.push(ValidationError {
                        description: format!("Enum duplique : '{}'", self.enums[i].name),
                    });
                }
            }
        }

        for i in 0..self.services.len() {
            for j in (i + 1)..self.services.len() {
                if self.services[i].name == self.services[j].name {
                    errors.push(ValidationError {
                        description: format!("Service duplique : '{}'", self.services[i].name),
                    });
                }
            }
        }

        for message in &self.messages {
            for i in 0..message.fields.len() {
                for j in (i + 1)..message.fields.len() {
                    if message.fields[i].number == message.fields[j].number {
                        errors.push(ValidationError {
                            description: format!(
                                "Numero de champ duplique ({}) dans le message '{}'",
                                message.fields[i].number, message.name
                            ),
                        });
                    }

                    if message.fields[i].name == message.fields[j].name {
                        errors.push(ValidationError {
                            description: format!(
                                "Nom de champ duplique '{}' dans le message '{}'",
                                message.fields[i].name, message.name
                            ),
                        });
                    }
                }
            }
        }

        let known_messages: Vec<&String> = self.messages.iter().map(|m| &m.name).collect();
        for service in &self.services {
            for method in &service.methods {
                if !known_messages.contains(&&method.request_type) {
                    errors.push(ValidationError {
                        description: format!(
                            "Type de requete inconnu '{}' dans la methode '{}' du service '{}'",
                            method.request_type, method.name, service.name
                        ),
                    });
                }
                if !known_messages.contains(&&method.response_type) {
                    errors.push(ValidationError {
                        description: format!(
                            "Type de reponse inconnu '{}' dans la methode '{}' du service '{}'",
                            method.response_type, method.name, service.name
                        ),
                    });
                }
            }
        }

        errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_valid() -> ProtoFile {
        ProtoFile {
            syntax: "proto3".to_string(),
            messages: vec![Message {
                name: "Person".to_string(),
                fields: vec![
                    Field { field_type: "string".to_string(), name: "name".to_string(), number: 1, repeated: false },
                    Field { field_type: "int32".to_string(), name: "age".to_string(), number: 2, repeated: false },
                ],
            }],
            enums: vec![],
            services: vec![],
        }
    }

    #[test]
    fn test_validate_no_errors_on_valid_file() {
        let pf = sample_valid();
        assert!(pf.validate().is_empty());
    }

    #[test]
    fn test_validate_duplicate_field_number() {
        let mut pf = sample_valid();
        pf.messages[0].fields[1].number = 1;
        let errors = pf.validate();
        assert!(errors.iter().any(|e| e.description.contains("Numero de champ duplique")));
    }

    #[test]
    fn test_validate_duplicate_field_name() {
        let mut pf = sample_valid();
        pf.messages[0].fields[1].name = "name".to_string();
        let errors = pf.validate();
        assert!(errors.iter().any(|e| e.description.contains("Nom de champ duplique")));
    }

    #[test]
    fn test_validate_unknown_rpc_type() {
        let mut pf = sample_valid();
        pf.services.push(Service {
            name: "S".to_string(),
            methods: vec![RpcMethod {
                name: "M".to_string(),
                request_type: "Inconnu".to_string(),
                response_type: "Person".to_string(),
            }],
        });
        let errors = pf.validate();
        assert!(errors.iter().any(|e| e.description.contains("Type de requete inconnu")));
    }

    #[test]
    fn test_to_json_contains_syntax() {
        let pf = sample_valid();
        let json = pf.to_json();
        assert!(json.contains("proto3"));
    }
}
