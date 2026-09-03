//! Error types for the arkouda crate.

use crate::concept::manifest::ManifestError;
use thiserror::Error;

/// Errors that can occur during arkouda operations.
#[derive(Error, Debug)]
pub enum ArkoudaError {
    /// No concept documents were found at the configured paths.
    #[error("No concepts found in {path}")]
    NoConceptsFound {
        /// Path that was searched.
        path: String,
    },

    /// A concept could not be found by id or filename.
    #[error("Concept not found: {0}")]
    ConceptNotFound(String),

    /// More than one concept matched a lookup.
    #[error("Concept lookup '{query}' is ambiguous; matched {count} files")]
    AmbiguousConcept {
        /// Lookup query.
        query: String,
        /// Number of matching files.
        count: usize,
    },

    /// A requested Markdown section was not present in the concept.
    #[error("Concept '{id}' has no `## {section}` section")]
    SectionNotFound {
        /// Concept id.
        id: String,
        /// Requested section name.
        section: String,
    },

    /// `arkouda section` was asked for a concept's primary section, but the
    /// concept declares a type arkouda does not know, so there is none.
    #[error(
        "Concept '{id}' declares type '{concept_type}', which arkouda does not know, so it has \
         no default section; name the section explicitly"
    )]
    UnknownConceptType {
        /// Concept id.
        id: String,
        /// The `type` the concept declares.
        concept_type: String,
    },

    /// `arkouda new --status` named a value outside the chosen type's
    /// vocabulary.
    #[error("Invalid status '{status}' for type '{concept_type}'; use one of: {valid}")]
    InvalidStatus {
        /// The status that was asked for.
        status: String,
        /// The OKF type it was asked for.
        concept_type: String,
        /// The vocabulary that type does accept.
        valid: String,
    },

    /// A concept id is invalid.
    #[error(
        "Invalid concept id '{0}': must be lowercase alphanumeric words separated by single hyphens"
    )]
    InvalidId(String),

    /// A new concept would overwrite an existing file.
    #[error("Concept '{id}' already exists at {path}")]
    ConceptExists {
        /// Concept id.
        id: String,
        /// Existing file path.
        path: String,
    },

    /// `arkouda new` was asked for a type the configuration gives no directory
    /// to write into.
    #[error(
        "No directory is configured for `--type {slug}`; add `{slug} = [\"<path>\"]` under \
         `[dirs]` in .arkoudarc.toml, or pass --dir"
    )]
    NoDirForType {
        /// The type's CLI slug.
        slug: String,
    },

    /// An operation that rewrites the bundle was asked to run against a single
    /// concept file rather than the bundle root.
    #[error(
        "Cannot regenerate the index for {path} from a single file; point --dir at the bundle directory"
    )]
    PartialBundle {
        /// Bundle root path.
        path: String,
    },

    /// Parsing a concept document failed.
    #[error("{path}: {source}")]
    Manifest {
        /// File path.
        path: String,
        /// Underlying manifest error.
        source: ManifestError,
    },

    /// Parsing the configuration file failed.
    #[error("{path}: {message}")]
    Config {
        /// Config file path.
        path: String,
        /// Underlying error message.
        message: String,
    },

    /// An I/O error occurred.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// A specialized Result type for arkouda operations.
pub type Result<T> = std::result::Result<T, ArkoudaError>;
