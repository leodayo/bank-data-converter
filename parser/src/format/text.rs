//! Key-Value plain-text serialization and deserialization for transaction logs.
//!
//! This module implements a flexible block-based plain-text format. Each transaction
//! record is represented as a block of `KEY: VALUE` lines, and blocks are separated by
//! one or more empty lines.
//!
//! ### Format Rules
//! - **Fields Order**: Fields within a transaction block can appear in any order.
//! - **Comments**: Lines starting with `#` are treated as comments and are ignored.
//! - **Separators**: A completely blank line signals the end of the current record.
//! - **Uniqueness**: Duplicate keys inside a single transaction block trigger an error.
//!
//! ### Example Layout
//! ```text
//! # This is an initial account deposit transaction
//! TX_ID: 1001
//! TX_TYPE: DEPOSIT
//! FROM_USER_ID: 0
//! TO_USER_ID: 501
//! AMOUNT: 50000
//! TIMESTAMP: 1672531200000
//! STATUS: SUCCESS
//! DESCRIPTION: Initial account deposit transaction
//!
//! TX_ID: 1002
//! ...
//! ```

use std::io::{BufRead, BufReader, Read, Write};

use crate::{
    error::ParserError,
    model::{Transaction, TransactionStatus, TxType},
};

macro_rules! parse_u64_field {
    ($processor:expr, $field:ident, $value:expr, $field_name:expr) => {{
        if $processor.current_record.$field.is_some() {
            return Err(ParserError::DuplicateField($field_name));
        }
        let parsed = $value
            .trim()
            .parse::<u64>()
            .map_err(|e| ParserError::InvalidField {
                field: $field_name,
                reason: e.to_string(),
            })?;
        $processor.current_record.$field(parsed);
    }};
}

macro_rules! parse_enum_field {
    ($processor:expr, $field:ident, $value:expr, $field_name:expr, $ty:ty) => {{
        if $processor.current_record.$field.is_some() {
            return Err(ParserError::DuplicateField($field_name));
        }
        let parsed = $value.trim().parse::<$ty>()?;
        $processor.current_record.$field(parsed);
    }};
}

/// An intermediate representation of a transaction matching the plain-text layout.
///
/// This structure stores raw attributes parsed from a `KEY: VALUE` text block. It can
/// be safely converted into the core domain model [`Transaction`] via [`TryFrom`].
#[derive(Debug, PartialEq, Eq)]
pub struct TextRecord {
    /// Unique identifier for the transaction.
    pub tx_id: u64,
    /// The type of the transaction.
    pub tx_type: TxType,
    /// Identifier of the user initiating the transaction.
    pub from_user_id: u64,
    /// Identifier of the user receiving the transaction.
    pub to_user_id: u64,
    /// The monetary value transferred.
    pub amount: u64,
    /// Unix timestamp indicating when the transaction occurred.
    pub timestamp: u64,
    /// The status of the transaction.
    pub status: TransactionStatus,
    /// Textual details accompanying the transaction record. Can be empty.
    pub description: String,
}

impl TryFrom<TextRecord> for Transaction {
    type Error = ParserError;

    fn try_from(rec: TextRecord) -> Result<Self, Self::Error> {
        fn to_i64(value: u64, field: &str) -> Result<i64, ParserError> {
            value.try_into().map_err(|_| ParserError::InvalidFormat {
                reason: format!("{} too large for i64: {}", field, value),
            })
        }

        Ok(Self {
            tx_id: to_i64(rec.tx_id, "tx_id")?,
            tx_type: rec.tx_type,
            from_user_id: to_i64(rec.from_user_id, "from_user_id")?,
            to_user_id: to_i64(rec.to_user_id, "to_user_id")?,
            amount: to_i64(rec.amount, "amount")?,
            timestamp: to_i64(rec.timestamp, "timestamp")?,
            status: rec.status,
            description: rec.description,
        })
    }
}

impl TryFrom<Transaction> for TextRecord {
    type Error = ParserError;

    fn try_from(tx: Transaction) -> Result<Self, Self::Error> {
        fn to_u64(value: i64, field: &str) -> Result<u64, ParserError> {
            value.try_into().map_err(|_| ParserError::InvalidFormat {
                reason: format!("{} cannot be negative or exceed u64: {}", field, value),
            })
        }

        Ok(Self {
            tx_id: to_u64(tx.tx_id, "tx_id")?,
            tx_type: tx.tx_type,
            from_user_id: to_u64(tx.from_user_id, "from_user_id")?,
            to_user_id: to_u64(tx.to_user_id, "to_user_id")?,
            amount: to_u64(tx.amount, "amount")?,
            timestamp: to_u64(tx.timestamp, "timestamp")?,
            status: tx.status,
            description: tx.description,
        })
    }
}

impl TextRecord {
    /// Reads all key-value blocks sequentially from the given input plain-text stream.
    ///
    /// # Errors
    ///
    /// Returns a [`ParserError`] if:
    /// - Any line is corrupted or missing the required colon delimiter (`:`).
    /// - An unknown key is encountered ([`ParserError::UnknownField`]).
    /// - A required field is missing when a block ends ([`ParserError::MissingField`]).
    /// - A field is defined more than once in a single block ([`ParserError::DuplicateField`]).
    /// - Field validation or type parsing fails ([`ParserError::InvalidField`]).
    pub fn read_from(r: impl Read) -> Result<Vec<Self>, ParserError> {
        let reader = BufReader::new(r);
        let mut processor = RecordProcessor::new();

        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();
            processor.process_line(trimmed)?;
        }

        processor.finalize()
    }

    /// Serializes and writes a slice of records as plain-text key-value blocks into the stream.
    ///
    /// Each transaction is printed field by field, followed by an additional newline
    /// acting as a block separator.
    ///
    /// # Errors
    ///
    /// Returns an underlying [`ParserError::Io`] error if a write operation fails.
    pub fn write_to(records: &[Self], writer: &mut impl Write) -> Result<(), ParserError> {
        for record in records {
            let line = format!(
                "TX_ID: {}\nTX_TYPE: {}\nFROM_USER_ID: {}\nTO_USER_ID: {}\nAMOUNT: {}\nTIMESTAMP: {}\nSTATUS: {}\nDESCRIPTION: {}\n",
                record.tx_id,
                record.tx_type,
                record.from_user_id,
                record.to_user_id,
                record.amount,
                record.timestamp,
                record.status,
                record.description
            );
            writer.write_all(line.as_bytes())?;
            writer.write_all(b"\n")?;
        }
        Ok(())
    }
}

struct RecordProcessor {
    records: Vec<TextRecord>,
    current_record: TextRecordBuilder,
}

impl RecordProcessor {
    pub fn new() -> Self {
        Self {
            records: vec![],
            current_record: TextRecordBuilder::new(),
        }
    }

    pub fn process_line(&mut self, line: &str) -> Result<(), ParserError> {
        if line.is_empty() {
            if !self.current_record.is_empty() {
                self.records.push(self.current_record.build()?);
            }
            return Ok(());
        }

        if line.starts_with('#') {
            return Ok(());
        }

        self.process_entry(line)?;
        Ok(())
    }

    fn process_entry(&mut self, entry: &str) -> Result<(), ParserError> {
        let (key, value) = entry
            .split_once(':')
            .ok_or_else(|| ParserError::InvalidFormat {
                reason: format!(
                    "Corrupted line. Expected: \"KEY: VALUE\", got: \"{}\"",
                    entry
                ),
            })?;

        let key = key.trim();
        let value = value.trim();

        match key {
            "TX_ID" => parse_u64_field!(self, tx_id, value, "TX_ID"),
            "TX_TYPE" => parse_enum_field!(self, tx_type, value, "TX_TYPE", TxType),
            "FROM_USER_ID" => parse_u64_field!(self, from_user_id, value, "FROM_USER_ID"),
            "TO_USER_ID" => parse_u64_field!(self, to_user_id, value, "TO_USER_ID"),
            "AMOUNT" => parse_u64_field!(self, amount, value, "AMOUNT"),
            "TIMESTAMP" => parse_u64_field!(self, timestamp, value, "TIMESTAMP"),
            "STATUS" => parse_enum_field!(self, status, value, "STATUS", TransactionStatus),
            "DESCRIPTION" => {
                if self.current_record.description.is_some() {
                    return Err(ParserError::DuplicateField("DESCRIPTION"));
                }
                self.current_record.description(value.trim().to_string());
            }
            _ => return Err(ParserError::UnknownField(key.to_string())),
        }

        Ok(())
    }

    pub fn finalize(mut self) -> Result<Vec<TextRecord>, ParserError> {
        if !self.current_record.is_empty() {
            self.records.push(self.current_record.build()?);
        }
        Ok(self.records)
    }
}

#[derive(Debug)]
struct TextRecordBuilder {
    tx_id: Option<u64>,
    tx_type: Option<TxType>,
    from_user_id: Option<u64>,
    to_user_id: Option<u64>,
    amount: Option<u64>,
    timestamp: Option<u64>,
    status: Option<TransactionStatus>,
    description: Option<String>,
}

impl TextRecordBuilder {
    pub fn new() -> Self {
        Self {
            tx_id: None,
            tx_type: None,
            from_user_id: None,
            to_user_id: None,
            amount: None,
            timestamp: None,
            status: None,
            description: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.tx_id.is_none()
            && self.tx_type.is_none()
            && self.from_user_id.is_none()
            && self.to_user_id.is_none()
            && self.amount.is_none()
            && self.timestamp.is_none()
            && self.status.is_none()
            && self.description.is_none()
    }

    pub fn build(&mut self) -> Result<TextRecord, ParserError> {
        Ok(TextRecord {
            tx_id: self
                .tx_id
                .take()
                .ok_or(ParserError::MissingField("TX_ID"))?,
            tx_type: self
                .tx_type
                .take()
                .ok_or(ParserError::MissingField("TX_TYPE"))?,
            from_user_id: self
                .from_user_id
                .take()
                .ok_or(ParserError::MissingField("FROM_USER_ID"))?,
            to_user_id: self
                .to_user_id
                .take()
                .ok_or(ParserError::MissingField("TO_USER_ID"))?,
            amount: self
                .amount
                .take()
                .ok_or(ParserError::MissingField("AMOUNT"))?,
            timestamp: self
                .timestamp
                .take()
                .ok_or(ParserError::MissingField("TIMESTAMP"))?,
            status: self
                .status
                .take()
                .ok_or(ParserError::MissingField("STATUS"))?,
            description: self
                .description
                .take()
                .ok_or(ParserError::MissingField("DESCRIPTION"))?,
        })
    }

    pub fn tx_id(&mut self, value: u64) -> &mut Self {
        self.tx_id = Some(value);
        self
    }

    pub fn tx_type(&mut self, value: TxType) -> &mut Self {
        self.tx_type = Some(value);
        self
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn from_user_id(&mut self, value: u64) -> &mut Self {
        self.from_user_id = Some(value);
        self
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn to_user_id(&mut self, value: u64) -> &mut Self {
        self.to_user_id = Some(value);
        self
    }

    pub fn amount(&mut self, value: u64) -> &mut Self {
        self.amount = Some(value);
        self
    }

    pub fn timestamp(&mut self, value: u64) -> &mut Self {
        self.timestamp = Some(value);
        self
    }

    pub fn status(&mut self, value: TransactionStatus) -> &mut Self {
        self.status = Some(value);
        self
    }

    pub fn description(&mut self, value: String) -> &mut Self {
        self.description = Some(value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    const VALID_TEXT: &str = r#"
    # Record 1 (Deposit)
    TX_ID: 1234567890123456
    TX_TYPE: DEPOSIT
    FROM_USER_ID: 0
    TO_USER_ID: 9876543210987654
    AMOUNT: 10000
    TIMESTAMP: 1633036800000
    STATUS: SUCCESS
    DESCRIPTION: "Terminal deposit"

    # Record 2 (Transfer)
    TX_ID: 2312321321321321
    TIMESTAMP: 1633056800000
    STATUS: FAILURE
    TX_TYPE: TRANSFER
    FROM_USER_ID: 1231231231231231
    TO_USER_ID: 9876543210987654
    AMOUNT: 1000
    DESCRIPTION: "User transfer"

    # Record 3 (Withdrawal)
    TX_ID: 3213213213213213
    AMOUNT: 100
    TX_TYPE: WITHDRAWAL
    FROM_USER_ID: 9876543210987654
    TO_USER_ID: 0
    TIMESTAMP: 1633066800000
    STATUS: SUCCESS
    DESCRIPTION: "User withdrawal""#;

    #[test]
    fn read_from_valid_text() {
        let data = Cursor::new(VALID_TEXT);
        let records = TextRecord::read_from(data).unwrap();

        assert_eq!(records.len(), 3);
        assert_eq!(
            records[0],
            TextRecord {
                tx_id: 1234567890123456,
                tx_type: TxType::Deposit,
                from_user_id: 0,
                to_user_id: 9876543210987654,
                amount: 10000,
                timestamp: 1633036800000,
                status: TransactionStatus::Success,
                description: "\"Terminal deposit\"".to_string()
            }
        );
        assert_eq!(
            records[1],
            TextRecord {
                tx_id: 2312321321321321,
                tx_type: TxType::Transfer,
                from_user_id: 1231231231231231,
                to_user_id: 9876543210987654,
                amount: 1000,
                timestamp: 1633056800000,
                status: TransactionStatus::Failure,
                description: "\"User transfer\"".to_string()
            }
        );
        assert_eq!(
            records[2],
            TextRecord {
                tx_id: 3213213213213213,
                tx_type: TxType::Withdrawal,
                from_user_id: 9876543210987654,
                to_user_id: 0,
                amount: 100,
                timestamp: 1633066800000,
                status: TransactionStatus::Success,
                description: "\"User withdrawal\"".to_string()
            }
        );
    }

    #[test]
    fn read_from_empty() {
        let data = Cursor::new("");
        let records = TextRecord::read_from(data).unwrap();

        assert_eq!(records.len(), 0);
    }

    #[test]
    fn read_from_corrupted_record_missing_field_status() {
        let text = r#"
            # Record 1 (Deposit)
            TX_ID: 1234567890123456
            TX_TYPE: DEPOSIT
            FROM_USER_ID: 0
            TO_USER_ID: 9876543210987654
            AMOUNT: 10000
            TIMESTAMP: 1633036800000
            DESCRIPTION: "Terminal deposit"
            "#;
        let data = Cursor::new(text);
        let err = TextRecord::read_from(data).unwrap_err();

        assert!(matches!(err, ParserError::MissingField("STATUS")));
    }

    #[test]
    fn read_from_invalid_field_tx_id() {
        let text = r#"
            # Record 1 (Deposit)
            TX_ID: NaN
            TX_TYPE: DEPOSIT
            FROM_USER_ID: 0
            TO_USER_ID: 9876543210987654
            AMOUNT: 10000
            TIMESTAMP: 1633036800000
            STATUS: SUCCESS
            DESCRIPTION: "Terminal deposit"
            "#;
        let data = Cursor::new(text);
        let err = TextRecord::read_from(data).unwrap_err();

        assert!(matches!(
            err,
            ParserError::InvalidField { field: "TX_ID", .. }
        ));
    }

    #[test]
    fn read_from_invalid_field_tx_type() {
        let text = r#"
            # Record 1 (Deposit)
            TX_ID: 1234567890123456
            TX_TYPE: 13
            FROM_USER_ID: 0
            TO_USER_ID: 9876543210987654
            AMOUNT: 10000
            TIMESTAMP: 1633036800000
            STATUS: SUCCESS
            DESCRIPTION: "Terminal deposit"
            "#;
        let data = Cursor::new(text);
        let err = TextRecord::read_from(data).unwrap_err();

        assert!(matches!(
            err,
            ParserError::InvalidField {
                field: "TX_TYPE",
                ..
            }
        ));
    }

    #[test]
    fn read_from_duplicate_field_status() {
        let text = r#"
            # Record 1 (Deposit)
            TX_ID: 1234567890123456
            TX_TYPE: DEPOSIT
            STATUS: FAILURE
            FROM_USER_ID: 0
            TO_USER_ID: 9876543210987654
            AMOUNT: 10000
            TIMESTAMP: 1633036800000
            STATUS: SUCCESS
            DESCRIPTION: "Terminal deposit"
            "#;
        let data = Cursor::new(text);
        let err = TextRecord::read_from(data).unwrap_err();

        assert!(matches!(err, ParserError::DuplicateField("STATUS")));
    }

    #[test]
    fn write_and_read_back() {
        let original = vec![TextRecord {
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
        TextRecord::write_to(&original, &mut buffer).unwrap();

        let read_back = TextRecord::read_from(Cursor::new(buffer)).unwrap();
        assert_eq!(original, read_back);
    }
}
