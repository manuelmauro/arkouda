//! Arkouda - a small CLI for navigating and validating ADR collections.

pub mod adr;
pub mod cli;
pub mod commands;
pub mod config;
pub mod error;

pub use error::{ArkoudaError, Result};
