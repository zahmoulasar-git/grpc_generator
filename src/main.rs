// ============================================================================
// NOTE ARCHITECTURALE (ISS-03) :
//
// Ce projet contient DEUX chaines de lecture d'un fichier .proto :
//
// 1. lexer/ + parser/ (ci-dessous) : Lexer et Parser ecrits a la main,
//    developpes durant les Sprints 1-2 du stage. Ils tokenisent le fichier,
//    construisent l'AST (ProtoFile), l'affichent, le valident, et generent
//    les structs Rust (serde) ainsi que le .proto standard consomme ensuite
//    par le pipeline Tonic. C'est un EXERCICE PEDAGOGIQUE d'apprentissage
//    de Rust et de conception de compilateur (Lexer -> Parser -> AST).
//
// 2. Le pipeline reellement utilise pour la communication gRPC (build.rs
//    genere + tonic-prost-build + protoc, voir codegen::rust_gen::
//    generate_build_rs) est INDEPENDANT du Lexer/Parser ci-dessus : il
//    reparse le fichier .proto genere par nos soins via l'outil officiel
//    protoc. C'est ce chemin qui alimente les tests reseau reels
//    (tests/grpc_integration.rs, tests/grpc_tls_integration.rs).
//
// Le Lexer/Parser fait main N'EST PAS utilise pour produire le code gRPC
// final ; il sert a l'analyse, la validation, et la generation des structs
// de donnees (messages) uniquement. Cette separation est documentee ici
// suite a une revue de code (ISS-03) plutot que supprimee, afin de
// conserver la demarche d'apprentissage des premiers sprints du stage.
// ============================================================================
mod lexer;
mod parser;
mod codegen;
mod error;

use lexer::lexer::Lexer;
use lexer::token_type::TokenType;
use parser::parser::Parser;
use error::GeneratorError;
use std::env;
use std::fs;
use std::path::Path;
use std::process;

// ISS-04 : point d'entree unique. Toute la logique vit dans run(), qui retourne
// un Result<(), GeneratorError>. main() ne fait plus qu'une seule chose :
// afficher l'erreur si besoin, et gerer le code de sortie a UN SEUL endroit.
fn main() {
    if let Err(e) = run() {
        eprintln!("{}", e);
        process::exit(1);
    }
}

// Ecrit un fichier et convertit toute erreur d'IO en GeneratorError::Io,
// avec un contexte lisible (evite de repeter ce pattern partout).
fn write_file(path: &str, content: &str, context: &str) -> Result<(), GeneratorError> {
    fs::write(path, content).map_err(|source| GeneratorError::Io {
        context: format!("{} ({})", context, path),
        source,
    })
}

// Lance une sous-commande cargo (build/test/package) dans un dossier donne.
// Convertit un echec de lancement en GeneratorError::Io, et un code de sortie
// non nul en GeneratorError::Generation avec le detail (stderr ou stdout).
fn run_cargo_step(step_name: &str, args: &[&str], dir: &str, use_stdout: bool) -> Result<(), GeneratorError> {
    let output = process::Command::new("cargo")
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|source| GeneratorError::Io {
            context: format!("impossible de lancer cargo {}", step_name),
            source,
        })?;

    if output.status.success() {
        Ok(())
    } else {
        let details = if use_stdout {
            String::from_utf8_lossy(&output.stdout).to_string()
        } else {
            String::from_utf8_lossy(&output.stderr).to_string()
        };
        Err(GeneratorError::Generation {
            step: step_name.to_string(),
            details,
        })
    }
}

fn run() -> Result<(), GeneratorError> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        return Err(GeneratorError::Usage(format!(
            "aucun fichier fourni.\nUsage: {} <fichier.proto>",
            args.get(0).map(String::as_str).unwrap_or("grpc_generator")
        )));
    }

    let file_path = &args[1];
    let path = Path::new(file_path);

    if !path.exists() {
        return Err(GeneratorError::Usage(format!("le fichier '{}' n'existe pas.", file_path)));
    }

    if path.extension().and_then(|ext| ext.to_str()) != Some("proto") {
        return Err(GeneratorError::Usage(format!(
            "le fichier '{}' n'a pas l'extension .proto",
            file_path
        )));
    }

    let input = fs::read_to_string(path).map_err(|source| GeneratorError::Io {
        context: format!("impossible de lire le fichier '{}'", file_path),
        source,
    })?;

    println!("Fichier '{}' chargé avec succès ({} caracteres).", file_path, input.len());

    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize();

    for token in &tokens {
        if token.token_type == TokenType::Invalid {
            return Err(GeneratorError::InvalidToken {
                value: token.value.clone(),
                line: token.line,
            });
        }
    }

    println!("Syntaxe valide. {} tokens generes.", tokens.len());

    let mut parser = Parser::new(tokens);
    let proto_file = parser.parse_proto_file().map_err(|e| GeneratorError::Parse(e.to_string()))?;

    println!("AST genere avec succes :");
    println!("{:#?}", proto_file);

    println!("\nParcours hierarchique de l'AST :");
    for line in proto_file.traverse() {
        println!("{}", line);
    }

    println!("\nValidation de l'AST :");
    let validation_errors = proto_file.validate();
    if validation_errors.is_empty() {
        println!("Aucune erreur de coherence detectee.");
    } else {
        for err in &validation_errors {
            println!("- {}", err.description);
        }
        let details = validation_errors
            .iter()
            .map(|e| format!("- {}", e.description))
            .collect::<Vec<_>>()
            .join("\n");
        return Err(GeneratorError::Validation {
            count: validation_errors.len(),
            details,
        });
    }

    let json_output = proto_file.to_json();
    let json_path = format!("{}.json", file_path);
    write_file(&json_path, &json_output, "export JSON")?;
    println!("\nAST exporte avec succes vers : {}", json_path);

    println!("\nCode Rust genere pour chaque message :");
    let mut generated_code = String::new();
    for message in &proto_file.messages {
        let rust_code = codegen::rust_gen::generate_struct(message);
        println!("{}", rust_code);
        generated_code.push_str(&rust_code);
        generated_code.push('\n');
    }

    // ISS-02 : les traits Client/Server sont a titre de documentation uniquement,
    // ils ne sont plus ecrits dans generated.rs (voir note plus haut).
    println!("\nInterfaces de communication (documentation uniquement, non compilees) :");
    for service in &proto_file.services {
        let interface_code = codegen::rust_gen::generate_service_interface(service);
        println!("{}", interface_code);
    }

    let tests_code = codegen::rust_gen::generate_tests_module(&proto_file.messages);
    generated_code.push_str(&tests_code);

    let lib_dir = format!("{}.generated_lib", file_path);
    let src_dir = format!("{}/src", lib_dir);
    fs::create_dir_all(&src_dir).map_err(|source| GeneratorError::Io {
        context: format!("creation dossier bibliotheque '{}'", src_dir),
        source,
    })?;

    let generated_path = format!("{}/generated.rs", src_dir);
    let lib_path = format!("{}/lib.rs", src_dir);
    let cargo_path = format!("{}/Cargo.toml", lib_dir);
    let proto_path = format!("{}/generated.proto", lib_dir);
    let build_rs_path = format!("{}/build.rs", lib_dir);

    let crate_name: String = Path::new(file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("generated_crate")
        .to_lowercase()
        .replace(|c: char| !c.is_alphanumeric(), "_");
    let package_name = crate_name.clone();

    let lib_rs_content = codegen::rust_gen::generate_lib_rs(&package_name);
    let cargo_toml_content = codegen::rust_gen::generate_cargo_toml(&crate_name);
    let proto_content = codegen::rust_gen::generate_proto_file(&proto_file, &package_name);
    let build_rs_content = codegen::rust_gen::generate_build_rs("generated.proto");

    write_file(&generated_path, &generated_code, "ecriture generated.rs")?;
    write_file(&lib_path, &lib_rs_content, "ecriture lib.rs")?;
    write_file(&cargo_path, &cargo_toml_content, "ecriture Cargo.toml")?;
    write_file(&proto_path, &proto_content, "ecriture generated.proto")?;
    write_file(&build_rs_path, &build_rs_content, "ecriture build.rs")?;

    println!("\nBibliotheque generee dans le dossier : {}", lib_dir);
    println!("  - {}", lib_path);
    println!("  - {}", generated_path);
    println!("  - {}", cargo_path);

    println!("\nCompilation automatique de la bibliotheque generee...");
    run_cargo_step("build", &["build"], &lib_dir, false)?;
    println!("Compilation reussie : la bibliotheque generee est valide.");

    println!("\nExecution des tests unitaires generes...");
    run_cargo_step("test", &["test"], &lib_dir, true)?;
    println!("Tous les tests generes sont passes avec succes.");

    println!("\nPackaging de la bibliotheque en crate distribuable...");
    run_cargo_step("package", &["package", "--allow-dirty"], &lib_dir, false)?;
    println!("Packaging reussi : crate prete pour distribution.");

    Ok(())
}
