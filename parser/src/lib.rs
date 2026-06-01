//! # Transaction Parser Library
//!
//! A library for parsing and validating structured data files.
//! It handles schema verification, custom formats, and provides detailed error reporting.
//!
//! ## Modules Overview
//!
//! - [`mod@error`]: Error types for handling parsing failures.
//! - [`mod@format`]: Implementations of various file formats and specifications, covering [Binary](format::bin), [CSV](format::csv) and [Plain Text](format::text) protocols.
//! - [`mod@model`]: Core data structures and domain models representing the parsed data.

/// Error variants indicating stream or domain constraint failures.
pub mod error;

/// Stream format parser implementations.
pub mod format;

/// Domain representations of transactions.
pub mod model;
