//! Serializers and deserializers for various data stream representations.
//!
//! This module groups implementations for different ledger exchange formats.
//! Each format submodule translates raw input streams into its intermediate record representation,
//! which can then be safely bound to the uniform [`Transaction`](crate::model::Transaction) domain structure.
//!
//! ## Module Features
//!
//! - [`bin`](crate::format::bin): Custom binary protocol utilizing `YPBN` magic bytes and Big-Endian integer flags.
//! - [`csv`](crate::format::bin): Comma-separated values format strictly enforced by an upper-case schema header constraint.
//! - [`text`](crate::format::bin): Loose key-value ledger format block-separated by clean empty lines, accommodating unordered parameters.

/// Custom binary format serialization logic.
pub mod bin;

/// Standard Comma-Separated Values format logic.
pub mod csv;

/// Block-based key-value plain-text layout logic.
pub mod text;
