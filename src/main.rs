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

use lexer::lexer::Lexer;
use lexer::token_type::TokenType;
use parser::parser::Parser;
use std::env;
use std::fs;
use std::path::Path;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Erreur : aucun fichier fourni.");
        eprintln!("Usage: {} <fichier.proto>", args[0]);
        process::exit(1);
    }

    let file_path = &args[1];
    let path = Path::new(file_path);

    if !path.exists() {
        eprintln!("Erreur : le fichier '{}' n'existe pas.", file_path);
        process::exit(1);
    }

    if path.extension().and_then(|ext| ext.to_str()) != Some("proto") {
        eprintln!("Erreur : le fichier '{}' n'a pas l'extension .proto", file_path);
        process::exit(1);
    }

    let input = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Erreur : impossible de lire le fichier '{}' ({})", file_path, e);
            process::exit(1);
        }
    };

    println!("Fichier '{}' chargé avec succès ({} caracteres).", file_path, input.len());

    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize();

    for token in &tokens {
        if token.token_type == TokenType::Invalid {
            eprintln!(
                "Erreur de syntaxe : caractere invalide '{}' a la ligne {}",
                token.value, token.line
            );
            process::exit(1);
        }
    }

    println!("Syntaxe valide. {} tokens generes.", tokens.len());

    let mut parser = Parser::new(tokens);
    match parser.parse_proto_file() {
        Ok(proto_file) => {
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
                eprintln!("");
                eprintln!("Generation annulee : l'AST contient {} erreur(s) de coherence.", validation_errors.len());
                process::exit(1);
            }

            let json_output = proto_file.to_json();
            let json_path = format!("{}.json", file_path);
            match fs::write(&json_path, &json_output) {
                Ok(_) => println!("\nAST exporte avec succes vers : {}", json_path),
                Err(e) => eprintln!("Erreur lors de l'export JSON : {}", e),
            }

            println!("\nCode Rust genere pour chaque message :");
            let mut generated_code = String::new();
            for message in &proto_file.messages {
                let rust_code = codegen::rust_gen::generate_struct(message);
                println!("{}", rust_code);
                generated_code.push_str(&rust_code);
                generated_code.push('\n');
            }

            // ISS-02 : les traits Client/Server (generate_service_interface) sont a titre
            // de documentation uniquement. Ils ne sont plus ecrits dans generated.rs car
            // le pipeline reellement utilise par la crate compilee est celui de Tonic
            // (build.rs + tonic-prost-build), pas ces traits synchrones.
            println!("\nInterfaces de communication (documentation uniquement, non compilees) :");
            for service in &proto_file.services {
                let interface_code = codegen::rust_gen::generate_service_interface(service);
                println!("{}", interface_code);
            }

            let tests_code = codegen::rust_gen::generate_tests_module(&proto_file.messages);
            generated_code.push_str(&tests_code);

            let lib_dir = format!("{}.generated_lib", file_path);
            let src_dir = format!("{}/src", lib_dir);
            match fs::create_dir_all(&src_dir) {
                Ok(_) => {
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
                    let write_generated = fs::write(&generated_path, &generated_code);
                    let write_lib = fs::write(&lib_path, &lib_rs_content);
                    let write_cargo = fs::write(&cargo_path, &cargo_toml_content);
                    let write_proto = fs::write(&proto_path, &proto_content);
                    let write_build_rs = fs::write(&build_rs_path, &build_rs_content);
                    match (write_generated, write_lib, write_cargo, write_proto, write_build_rs) {
                        (Ok(_), Ok(_), Ok(_), Ok(_), Ok(_)) => {
                            println!("\nBibliotheque generee dans le dossier : {}", lib_dir);
                            println!("  - {}", lib_path);
                            println!("  - {}", generated_path);
                            println!("  - {}", cargo_path);

                            println!("\nCompilation automatique de la bibliotheque generee...");
                            let compile_result = process::Command::new("cargo")
                                .arg("build")
                                .current_dir(&lib_dir)
                                .output();

                            match compile_result {
                                Ok(output) => {
                                    if output.status.success() {
                                        println!("Compilation reussie : la bibliotheque generee est valide.");

                                        println!("\nExecution des tests unitaires generes...");
                                        let test_result = process::Command::new("cargo")
                                            .arg("test")
                                            .current_dir(&lib_dir)
                                            .output();

                                        match test_result {
                                            Ok(test_output) => {
                                                if test_output.status.success() {
                                                    println!("Tous les tests generes sont passes avec succes.");

                                                    println!("\nPackaging de la bibliotheque en crate distribuable...");
                                                    let package_result = process::Command::new("cargo")
                                                        .arg("package")
                                                        .arg("--allow-dirty")
                                                        .current_dir(&lib_dir)
                                                        .output();

                                                    match package_result {
                                                        Ok(package_output) => {
                                                            if package_output.status.success() {
                                                                println!("Packaging reussi : crate prete pour distribution.");
                                                            } else {
                                                                eprintln!("Echec du packaging :");
                                                                eprintln!("{}", String::from_utf8_lossy(&package_output.stderr));
                                                                process::exit(1);
                                                            }
                                                        }
                                                        Err(e) => {
                                                            eprintln!("Impossible de lancer cargo package : {}", e);
                                                            process::exit(1);
                                                        }
                                                    }
                                                } else {
                                                    eprintln!("Echec des tests generes :");
                                                    eprintln!("{}", String::from_utf8_lossy(&test_output.stdout));
                                                    process::exit(1);
                                                }
                                            }
                                            Err(e) => {
                                                eprintln!("Impossible de lancer cargo test : {}", e);
                                                process::exit(1);
                                            }
                                        }
                                    } else {
                                        eprintln!("Echec de la compilation de la bibliotheque generee :");
                                        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
                                        process::exit(1);
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Impossible de lancer cargo build : {}", e);
                                    process::exit(1);
                                }
                            }
                        }
                        _ => eprintln!("Erreur lors de l'ecriture des fichiers de la bibliotheque."),
                    }
                }
                Err(e) => eprintln!("Erreur creation dossier bibliotheque : {}", e),
            }
        }
        Err(e) => {
            eprintln!("Erreur de parsing : {}", e);
            process::exit(1);
        }
    }
}
