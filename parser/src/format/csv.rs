//! Comma-Separated Values (CSV) serialization and deserialization for transaction logs.
//!
//! This module provides capabilities to read and write transaction records using a strict,
//! newline-delimited, comma-separated format.
//!
//! ### Expected CSV Schema
//!
//! Every valid CSV file must begin with a specific header row. Columns are position-based
//! and must appear in the following order:
//!
//! ```text
//! TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION
//! ```

use crate::{
    error::ParserError,
    model::{Transaction, TransactionStatus, TxType},
};
use std::io::{BufRead, BufReader, Read, Write};

/// An intermediate representation of a transaction matching the CSV layout.
///
/// This structure matches the exact order and names of fields found within the CSV schema.
/// It implements [`From`] for lossless, infallible conversion into the core domain
/// model [`Transaction`].
#[derive(Debug, PartialEq, Eq)]
pub struct CsvRecord {
    /// Unique identifier for the transaction.
    pub tx_id: i64,
    /// The type of the transaction (parsed case-insensitively).
    pub tx_type: TxType,
    /// Identifier of the user initiating the transaction.
    pub from_user_id: i64,
    /// Identifier of the user receiving the transaction.
    pub to_user_id: i64,
    /// The monetary value transferred.
    pub amount: i64,
    /// Unix timestamp indicating when the transaction occurred.
    pub timestamp: i64,
    /// The status of the transaction (parsed case-insensitively).
    pub status: TransactionStatus,
    /// Textual details accompanying the transaction record. Can be empty.
    pub description: String,
}

impl From<CsvRecord> for Transaction {
    fn from(rec: CsvRecord) -> Self {
        Self {
            tx_id: rec.tx_id,
            tx_type: rec.tx_type,
            from_user_id: rec.from_user_id,
            to_user_id: rec.to_user_id,
            amount: rec.amount,
            timestamp: rec.timestamp,
            status: rec.status,
            description: rec.description,
        }
    }
}

impl From<Transaction> for CsvRecord {
    fn from(tx: Transaction) -> Self {
        Self {
            tx_id: tx.tx_id,
            tx_type: tx.tx_type,
            from_user_id: tx.from_user_id,
            to_user_id: tx.to_user_id,
            amount: tx.amount,
            timestamp: tx.timestamp,
            status: tx.status,
            description: tx.description,
        }
    }
}

impl CsvRecord {
    /// The precise header line that must match at the beginning of the file.
    const EXPECTED_HEADER: &str =
        "TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION";

    /// Total expected count of fields separated by commas per line.
    const FIELDS_COUNT: usize = 8;

    /// Reads all CSV records sequentially from the given input stream.
    ///
    /// # Errors
    ///
    /// Returns a [`ParserError`] if:
    /// - The source stream is completely empty ([`ParserError::EmptyFile`]).
    /// - The first line does not match `Self::EXPECTED_HEADER` exactly ([`ParserError::InvalidHeader`]).
    /// - Any record line has a mismatched column count or contains values that fail to parse into their respective types.
    pub fn read_from(r: impl Read) -> Result<Vec<Self>, ParserError> {
        let reader = BufReader::new(r);
        let mut lines = reader.lines();

        let header = lines.next().ok_or(ParserError::EmptyFile)??;
        if header != Self::EXPECTED_HEADER {
            return Err(ParserError::InvalidHeader {
                expected: Self::EXPECTED_HEADER,
                got: header,
            });
        }

        let mut records = Vec::new();
        for line in lines {
            records.push(Self::parse_record(line?)?);
        }

        Ok(records)
    }

    /// Serializes and writes a slice of records into the provided output writer stream.
    ///
    /// This method automatically prepends the expected CSV header row followed by a newline character.
    ///
    /// # Errors
    ///
    /// Returns an underlying [`ParserError::Io`] error if a write operation fails.
    pub fn write_to(records: &[Self], writer: &mut impl Write) -> Result<(), ParserError> {
        writer.write_all(Self::EXPECTED_HEADER.as_bytes())?;
        writer.write_all(b"\n")?;

        for record in records {
            let line = format!(
                "{},{},{},{},{},{},{},{}\n",
                record.tx_id,
                record.tx_type,
                record.from_user_id,
                record.to_user_id,
                record.amount,
                record.timestamp,
                record.status,
                record.description
            );
            writer.write_all(line.as_bytes())?
        }
        Ok(())
    }

    fn parse_record(line: String) -> Result<Self, ParserError> {
        let parts: Vec<&str> = line.splitn(Self::FIELDS_COUNT, ',').collect();
        if parts.len() != Self::FIELDS_COUNT {
            return Err(ParserError::InvalidFormat {
                reason: format!("Expected {} elements per line", Self::FIELDS_COUNT),
            });
        }

        Ok(Self {
            tx_id: parts[0]
                .trim()
                .parse::<i64>()
                .map_err(|e| ParserError::InvalidField {
                    field: "TX_ID",
                    reason: e.to_string(),
                })?,
            tx_type: parts[1].trim().parse::<TxType>()?,
            from_user_id: parts[2].trim().parse::<i64>().map_err(|e| {
                ParserError::InvalidField {
                    field: "FROM_USER_ID",
                    reason: e.to_string(),
                }
            })?,
            to_user_id: parts[3]
                .trim()
                .parse::<i64>()
                .map_err(|e| ParserError::InvalidField {
                    field: "TO_USER_ID",
                    reason: e.to_string(),
                })?,
            amount: parts[4]
                .trim()
                .parse::<i64>()
                .map_err(|e| ParserError::InvalidField {
                    field: "AMOUNT",
                    reason: e.to_string(),
                })?,
            timestamp: parts[5]
                .trim()
                .parse::<i64>()
                .map_err(|e| ParserError::InvalidField {
                    field: "TIMESTAMP",
                    reason: e.to_string(),
                })?,
            status: parts[6].trim().parse::<TransactionStatus>()?,
            description: parts[7].trim().to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    const VALID_CSV: &str = "TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION\n\
    1001,DEPOSIT,0,501,50000,1672531200000,SUCCESS,\"Initial account funding\"\n\
    1002,TRANSFER,501,502,15000,1672534800000,FAILURE,\"Payment for services, invoice #123\"\n\
    1003,WITHDRAWAL,502,0,1000,1672538400000,PENDING,\"ATM withdrawal\"\n";

    #[test]
    fn test_fields_count_matches_struct() {
        assert_eq!(
            CsvRecord::EXPECTED_HEADER.split(',').count(),
            CsvRecord::FIELDS_COUNT,
            "FIELDS_COUNT doesn't match count of fields in EXPECTED_HEADER",
        );
    }

    #[test]
    fn read_from_valid_csv() {
        let data = Cursor::new(VALID_CSV);
        let records = CsvRecord::read_from(data).unwrap();

        assert_eq!(records.len(), 3);
        assert_eq!(
            records[0],
            CsvRecord {
                tx_id: 1001,
                tx_type: TxType::Deposit,
                from_user_id: 0,
                to_user_id: 501,
                amount: 50000,
                timestamp: 1672531200000,
                status: TransactionStatus::Success,
                description: "\"Initial account funding\"".to_string()
            }
        );
        assert_eq!(
            records[1],
            CsvRecord {
                tx_id: 1002,
                tx_type: TxType::Transfer,
                from_user_id: 501,
                to_user_id: 502,
                amount: 15000,
                timestamp: 1672534800000,
                status: TransactionStatus::Failure,
                description: "\"Payment for services, invoice #123\"".to_string()
            }
        );
        assert_eq!(
            records[2],
            CsvRecord {
                tx_id: 1003,
                tx_type: TxType::Withdrawal,
                from_user_id: 502,
                to_user_id: 0,
                amount: 1000,
                timestamp: 1672538400000,
                status: TransactionStatus::Pending,
                description: "\"ATM withdrawal\"".to_string()
            }
        );
    }

    #[test]
    fn read_from_empty() {
        let data = Cursor::new("");
        let err = CsvRecord::read_from(data).unwrap_err();

        assert!(matches!(err, ParserError::EmptyFile));
    }

    #[test]
    fn read_from_invalid_header() {
        let csv = "SOME,RANDOM,HEADER\n\
            1001,DEPOSIT,0,501,50000,1672531200000,SUCCESS,\"Initial account funding\"";
        let data = Cursor::new(csv);
        let err = CsvRecord::read_from(data).unwrap_err();

        assert!(matches!(err, ParserError::InvalidHeader { .. }));
    }

    #[test]
    fn read_from_invalid_field_tx_id() {
        let csv = "\
            TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION\n\
            not_an_int,DEPOSIT,0,501,50000,1672531200000,SUCCESS,\"Initial account funding\"\n";
        let data = Cursor::new(csv);
        let err = CsvRecord::read_from(data).unwrap_err();

        assert!(matches!(
            err,
            ParserError::InvalidField { field: "TX_ID", .. }
        ));
    }

    #[test]
    fn read_from_invalid_field_tx_type() {
        let csv = "\
            TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION\n\
            1001,BUY,0,501,50000,1672531200000,SUCCESS,\"Initial account funding\"\n";
        let data = Cursor::new(csv);
        let err = CsvRecord::read_from(data).unwrap_err();

        assert!(matches!(
            err,
            ParserError::InvalidField {
                field: "TX_TYPE",
                ..
            }
        ));
    }

    #[test]
    fn write_and_read_back() {
        let original = vec![CsvRecord {
            tx_id: 1003,
            tx_type: TxType::Withdrawal,
            from_user_id: 502,
            to_user_id: 0,
            amount: 1000,
            timestamp: 1672538400000,
            status: TransactionStatus::Pending,
            description: "ATM withdrawal".to_string(),
        }];

        let mut buffer = Vec::new();
        CsvRecord::write_to(&original, &mut buffer).unwrap();

        let read_back = CsvRecord::read_from(Cursor::new(buffer)).unwrap();
        assert_eq!(original, read_back);
    }
}
