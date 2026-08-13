use thiserror::Error;

/// Erreur unifiee du generateur (ISS-04).
/// Remplace les appels eprintln!/process::exit disperses dans main.rs
/// par un seul type d'erreur propageable via Result<T, GeneratorError>.
#[derive(Error, Debug)]
pub enum GeneratorError {
    #[error("Erreur de fichier ({context}) : {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Erreur de syntaxe : caractere invalide '{value}' a la ligne {line}")]
    InvalidToken { value: String, line: usize },

    #[error("Erreur de parsing : {0}")]
    Parse(String),

    #[error("Generation annulee : l'AST contient {count} erreur(s) de coherence :\n{details}")]
    Validation { count: usize, details: String },

    #[error("Echec de generation ({step}) : {details}")]
    Generation { step: String, details: String },

    #[error("Usage invalide : {0}")]
    Usage(String),
}
