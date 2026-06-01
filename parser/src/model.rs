//! Domain models representing transactions and their attributes.
//!
//! This module contains core data structures used by the parser to map,
//! validate, and store transaction records.

use crate::error::ParserError;

/// Represents a single transaction record.
#[derive(Debug, PartialEq, Eq)]
pub struct Transaction {
    /// Unique identifier for the transaction.
    pub tx_id: i64,
    /// The type of the transaction (e.g., Deposit, Transfer).
    pub tx_type: TxType,
    /// Unique identifier of the user initiating the transaction.
    pub from_user_id: i64,
    /// Unique identifier of the user receiving the transaction.
    pub to_user_id: i64,
    /// The monetary value transferred, represented in the smallest currency unit (e.g., cents).
    pub amount: i64,
    /// Unix timestamp indicating when the transaction was created.
    pub timestamp: i64,
    /// The current processing state of the transaction.
    pub status: TransactionStatus,
    /// Textual details accompanying the transaction. Can be empty.
    pub description: String,
}

/// The type of a financial transaction.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum TxType {
    /// Funds being deposited to an account.
    Deposit,
    /// Funds being moved between two different users.
    Transfer,
    /// Funds being withdrawed from an account.
    Withdrawal,
}

/// The status of a transaction.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum TransactionStatus {
    /// The transaction completed successfully.
    Success,
    /// The transaction failed during processing.
    Failure,
    /// The transaction is awaiting processing or confirmation.
    Pending,
}

impl std::str::FromStr for TxType {
    type Err = ParserError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("DEPOSIT") {
            return Ok(TxType::Deposit);
        };
        if s.eq_ignore_ascii_case("TRANSFER") {
            return Ok(TxType::Transfer);
        };
        if s.eq_ignore_ascii_case("WITHDRAWAL") {
            return Ok(TxType::Withdrawal);
        };
        Err(ParserError::InvalidField {
            field: "TX_TYPE",
            reason: "unknown transaction type".to_string(),
        })
    }
}

impl std::str::FromStr for TransactionStatus {
    type Err = ParserError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("SUCCESS") {
            return Ok(TransactionStatus::Success);
        };
        if s.eq_ignore_ascii_case("FAILURE") {
            return Ok(TransactionStatus::Failure);
        };
        if s.eq_ignore_ascii_case("PENDING") {
            return Ok(TransactionStatus::Pending);
        };

        Err(ParserError::InvalidField {
            field: "STATUS",
            reason: "unknown transaction status".to_string(),
        })
    }
}

impl std::fmt::Display for TxType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            TxType::Deposit => "DEPOSIT",
            TxType::Transfer => "TRANSFER",
            TxType::Withdrawal => "WITHDRAWAL",
        };
        write!(f, "{}", s)
    }
}

impl std::fmt::Display for TransactionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            TransactionStatus::Success => "SUCCESS",
            TransactionStatus::Failure => "FAILURE",
            TransactionStatus::Pending => "PENDING",
        };
        write!(f, "{}", s)
    }
}

impl TryFrom<u8> for TxType {
    type Error = ParserError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(TxType::Deposit),
            1 => Ok(TxType::Transfer),
            2 => Ok(TxType::Withdrawal),
            _ => Err(ParserError::InvalidField {
                field: "TX_TYPE",
                reason: format!("invalid byte: {:#04X}", value),
            }),
        }
    }
}

impl From<TxType> for u8 {
    fn from(value: TxType) -> Self {
        match value {
            TxType::Deposit => 0,
            TxType::Transfer => 1,
            TxType::Withdrawal => 2,
        }
    }
}

impl TryFrom<u8> for TransactionStatus {
    type Error = ParserError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(TransactionStatus::Success),
            1 => Ok(TransactionStatus::Failure),
            2 => Ok(TransactionStatus::Pending),
            _ => Err(ParserError::InvalidField {
                field: "STATUS",
                reason: format!("invalid byte: {:#04X}", value),
            }),
        }
    }
}

impl From<TransactionStatus> for u8 {
    fn from(value: TransactionStatus) -> Self {
        match value {
            TransactionStatus::Success => 0,
            TransactionStatus::Failure => 1,
            TransactionStatus::Pending => 2,
        }
    }
}
