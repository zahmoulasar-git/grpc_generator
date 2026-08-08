use crate::parser::ast::{Message, Field, Service};

// Convertit un nom en PascalCase/camelCase (ex: "GetUser") vers snake_case (ex: "get_user")
pub fn to_snake_case(name: &str) -> String {
    let mut result = String::new();

    for (i, c) in name.chars().enumerate() {
        if c.is_uppercase() {
            if i != 0 {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }

    result
}

// Convertit un type Protobuf (string, int32...) vers son equivalent Rust (String, i32...)
pub fn proto_type_to_rust(proto_type: &str) -> String {
    match proto_type {
        "string" => "String".to_string(),
        "int32" => "i32".to_string(),
        "int64" => "i64".to_string(),
        "uint32" => "u32".to_string(),
        "uint64" => "u64".to_string(),
        "bool" => "bool".to_string(),
        "float" => "f32".to_string(),
        "double" => "f64".to_string(),
        "bytes" => "Vec<u8>".to_string(),
        // Si ce n'est pas un type connu, on suppose que c'est un autre message (ex: Address)
        other => other.to_string(),
    }
}

// Genere la ligne Rust pour un seul champ, ex: "pub name: String,"
pub fn generate_field(field: &Field) -> String {
    let rust_type = proto_type_to_rust(&field.field_type);

    let final_type = if field.repeated {
        format!("Vec<{}>", rust_type)
    } else {
        rust_type
    };

    format!("    pub {}: {},", field.name, final_type)
}

// Genere le struct Rust complet pour un Message
pub fn generate_struct(message: &Message) -> String {
    let mut code = String::new();

    code.push_str("#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]\n");
    code.push_str(&format!("pub struct {} {{\n", message.name));

    for field in &message.fields {
        code.push_str(&generate_field(field));
        code.push('\n');
    }

    code.push_str("}\n");

    code
}

// Genere un trait Rust representant le contrat client d'un Service
pub fn generate_client_trait(service: &Service) -> String {
    let mut code = String::new();

    code.push_str(&format!("pub trait {}Client {{\n", service.name));

    for method in &service.methods {
        let method_name = to_snake_case(&method.name);
        code.push_str(&format!(
            "    fn {}(&self, request: {}) -> {};\n",
            method_name, method.request_type, method.response_type
        ));
    }

    code.push_str("}\n");

    code
}

// Genere un trait Rust representant le contrat serveur d'un Service
pub fn generate_server_trait(service: &Service) -> String {
    let mut code = String::new();

    code.push_str(&format!("pub trait {}Server {{\n", service.name));

    for method in &service.methods {
        let method_name = to_snake_case(&method.name);
        code.push_str(&format!(
            "    fn {}(&self, request: {}) -> {};\n",
            method_name, method.request_type, method.response_type
        ));
    }

    code.push_str("}\n");

    code
}

// Genere l'interface de communication complete d'un Service :
// documentation + contrat client + contrat serveur, dans un seul bloc coherent.
pub fn generate_service_interface(service: &Service) -> String {
    let mut code = String::new();

    code.push_str(&format!(
        "// ==================== Interface de communication : {} ====================\n",
        service.name
    ));
    code.push_str(&format!(
        "// Ce service definit {} methode(s) RPC. Chaque methode a :\n",
        service.methods.len()
    ));
    code.push_str("// - un type de requete (Request), envoye par le client\n");
    code.push_str("// - un type de reponse (Response), renvoye par le serveur\n");
    code.push_str("//\n");
    code.push_str("// Contrat des methodes :\n");
    for method in &service.methods {
        code.push_str(&format!(
            "//   - {} : requete = {}, reponse = {}\n",
            method.name, method.request_type, method.response_type
        ));
    }
    code.push_str("\n");

    code.push_str("/// Contrat que le CLIENT utilise pour appeler le service a distance.\n");
    code.push_str(&generate_client_trait(service));
    code.push_str("\n");

    code.push_str("/// Contrat que le SERVEUR doit implementer pour traiter les appels entrants.\n");
    code.push_str(&generate_server_trait(service));

    code
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::{Field, Message, Service, RpcMethod};

    #[test]
    fn test_proto_type_to_rust_mapping() {
        assert_eq!(proto_type_to_rust("string"), "String");
        assert_eq!(proto_type_to_rust("int32"), "i32");
        assert_eq!(proto_type_to_rust("bool"), "bool");
    }

    #[test]
    fn test_to_snake_case_conversion() {
        assert_eq!(to_snake_case("GetUser"), "get_user");
        assert_eq!(to_snake_case("A"), "a");
    }

    #[test]
    fn test_generate_struct_contains_fields() {
        let message = Message {
            name: "Person".to_string(),
            fields: vec![
                Field { field_type: "string".to_string(), name: "name".to_string(), number: 1, repeated: false },
                Field { field_type: "string".to_string(), name: "tags".to_string(), number: 2, repeated: true },
            ],
        };
        let code = generate_struct(&message);
        assert!(code.contains("pub struct Person"));
        assert!(code.contains("pub name: String,"));
        assert!(code.contains("pub tags: Vec<String>,"));
    }

    #[test]
    fn test_generate_client_trait_signature() {
        let service = Service {
            name: "UserService".to_string(),
            methods: vec![RpcMethod {
                name: "GetUser".to_string(),
                request_type: "GetUserRequest".to_string(),
                response_type: "GetUserResponse".to_string(),
            }],
        };
        let code = generate_client_trait(&service);
        assert!(code.contains("pub trait UserServiceClient"));
        assert!(code.contains("fn get_user(&self, request: GetUserRequest) -> GetUserResponse;"));
    }
}

// Genere le contenu de lib.rs : point d'entree qui expose tout le code genere.
pub fn generate_lib_rs(package_name: &str) -> String {
    let mut code = String::new();
    code.push_str("// Point d'entree de la bibliotheque generee automatiquement.\n");
    code.push_str("// Regroupe tous les types (messages) et interfaces (services) generes.\n\n");
    code.push_str("pub mod generated;\n\n");
    code.push_str("pub use generated::*;\n\n");
    code.push_str("// Module gRPC genere par tonic-prost-build a partir du .proto genere.\n");
    code.push_str("pub mod grpc {\n");
    code.push_str(&format!("    tonic::include_proto!(\"{}\");\n", package_name));
    code.push_str("}\n");
    code
}

// Genere le contenu du Cargo.toml pour la bibliotheque generee.
// crate_name : nom du package (derive du nom du fichier .proto source)
pub fn generate_cargo_toml(crate_name: &str) -> String {
    format!(
        "[package]\nname = \"{}\"\nversion = \"0.1.0\"\nedition = \"2021\"\ndescription = \"Bibliotheque Rust generee automatiquement a partir d'\''un fichier .proto par grpc_generator\"\nlicense = \"MIT\"\n\n\
[dependencies]\nserde = {{ version = \"1\", features = [\"derive\"] }}\nserde_json = \"1\"\ntonic = {{ version = \"0.14\", features = [\"tls-ring\"] }}\ntonic-prost = \"0.14\"\nprost = \"0.14\"\ntokio = {{ version = \"1\", features = [\"full\"] }}\n\n\
[build-dependencies]\ntonic-prost-build = \"0.14\"\n",
        crate_name
    )
}

// Genere un test unitaire pour un Message : verifie que la valeur par defaut
// survit a un aller-retour serialisation -> deserialisation.
pub fn generate_struct_test(message: &Message) -> String {
    format!(
        "    #[test]\n    fn test_{}_roundtrip() {{\n        let original = {}::default();\n        let json = serde_json::to_string(&original).expect(\"serialisation echouee\");\n        let restored: {} = serde_json::from_str(&json).expect(\"deserialisation echouee\");\n        assert_eq!(original, restored);\n    }}\n",
        to_snake_case(&message.name), message.name, message.name
    )
}

// Genere le module de tests complet pour tous les messages d'un ProtoFile.
pub fn generate_tests_module(messages: &[Message]) -> String {
    let mut code = String::new();
    code.push_str("#[cfg(test)]\nmod generated_tests {\n    use super::*;\n\n");
    for message in messages {
        code.push_str(&generate_struct_test(message));
        code.push('\n');
    }
    code.push_str("}\n");
    code
}

// Genere un vrai fichier .proto standard (syntaxe Protobuf classique), utilisable par tonic-prost-build.
// package_name : nom du package proto (derive du nom de la crate)
pub fn generate_proto_file(proto_file: &crate::parser::ast::ProtoFile, package_name: &str) -> String {
    let mut code = String::new();

    code.push_str(&format!("syntax = \"{}\";\n\n", proto_file.syntax));
    code.push_str(&format!("package {};\n\n", package_name));

    for message in &proto_file.messages {
        code.push_str(&format!("message {} {{\n", message.name));
        for field in &message.fields {
            let repeated_prefix = if field.repeated { "repeated " } else { "" };
            code.push_str(&format!(
                "    {}{} {} = {};\n",
                repeated_prefix, field.field_type, field.name, field.number
            ));
        }
        code.push_str("}\n\n");
    }

    for enum_def in &proto_file.enums {
        code.push_str(&format!("enum {} {{\n", enum_def.name));
        for value in &enum_def.values {
            code.push_str(&format!("    {} = {};\n", value.name, value.number));
        }
        code.push_str("}\n\n");
    }

    for service in &proto_file.services {
        code.push_str(&format!("service {} {{\n", service.name));
        for method in &service.methods {
            code.push_str(&format!(
                "    rpc {}({}) returns ({});\n",
                method.name, method.request_type, method.response_type
            ));
        }
        code.push_str("}\n\n");
    }

    code
}

// Genere le contenu de build.rs, qui compile le .proto genere via tonic-prost-build.
pub fn generate_build_rs(proto_filename: &str) -> String {
    format!(
        "fn main() -> Result<(), Box<dyn std::error::Error>> {{\n    tonic_prost_build::configure()\n        .compile_protos(&[\"{}\"], &[\".\"])?;\n    Ok(())\n}}\n",
        proto_filename
    )
}
