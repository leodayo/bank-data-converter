use std::str::FromStr;

use crate::error::ParserError;

#[derive(Debug, PartialEq, Eq)]
pub enum TxType {
    Deposit,
    Transfer,
    Withdrawal,
}

#[derive(Debug, PartialEq, Eq)]
pub enum TransactionStatus {
    Success,
    Failure,
    Pending,
}

impl FromStr for TxType {
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

impl FromStr for TransactionStatus {
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
