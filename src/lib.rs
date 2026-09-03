//! Arkouda - a small CLI for navigating and validating OKF concept bundles.

pub mod cli;
pub mod commands;
pub mod concept;
pub mod config;
pub mod error;
pub mod telemetry;

pub use error::{ArkoudaError, Result};
