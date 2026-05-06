//! Error types for the arkouda crate.

use crate::adr::manifest::ManifestError;
use thiserror::Error;

/// Errors that can occur during ADR operations.
#[derive(Error, Debug)]
pub enum ArkoudaError {
    /// No ADR files were found at the configured path.
    #[error("No ADR files found in {path}")]
    NoAdrsFound {
        /// Path that was searched.
        path: String,
    },

    /// An ADR could not be found by id or filename.
    #[error("ADR not found: {0}")]
    AdrNotFound(String),

    /// More than one ADR matched a lookup.
    #[error("ADR lookup '{query}' is ambiguous; matched {count} files")]
    AmbiguousAdr {
        /// Lookup query.
        query: String,
        /// Number of matching files.
        count: usize,
    },

    /// An ADR id is invalid.
    #[error(
        "Invalid ADR id '{0}': must be lowercase alphanumeric words separated by single hyphens"
    )]
    InvalidId(String),

    /// A new ADR would overwrite an existing file.
    #[error("ADR '{id}' already exists at {path}")]
    AdrExists {
        /// ADR id.
        id: String,
        /// Existing file path.
        path: String,
    },

    /// Parsing an ADR manifest failed.
    #[error("{path}: {source}")]
    Manifest {
        /// File path.
        path: String,
        /// Underlying manifest error.
        source: ManifestError,
    },

    /// An I/O error occurred.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// A specialized Result type for arkouda operations.
pub type Result<T> = std::result::Result<T, ArkoudaError>;
