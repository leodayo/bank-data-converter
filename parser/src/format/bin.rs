use std::io::{BufReader, Read, Write};

use crate::{
    error::ParserError,
    model::{Transaction, TransactionStatus, TxType},
};

#[derive(Debug, PartialEq, Eq)]
pub struct BinRecordHeader {
    pub magic: [u8; 4],
    pub record_size: u32,
}

#[derive(Debug, PartialEq, Eq)]
pub struct BinRecord {
    pub tx_id: u64,
    pub tx_type: TxType,
    pub from_user_id: u64,
    pub to_user_id: u64,
    pub amount: i64,
    pub timestamp: u64,
    pub status: TransactionStatus,
    pub description: String,
}

impl TryFrom<BinRecord> for Transaction {
    type Error = ParserError;

    fn try_from(rec: BinRecord) -> Result<Self, Self::Error> {
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
            amount: rec.amount,
            timestamp: to_i64(rec.timestamp, "timestamp")?,
            status: rec.status,
            description: rec.description,
        })
    }
}

impl TryFrom<Transaction> for BinRecord {
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
            amount: tx.amount, // i64
            timestamp: to_u64(tx.timestamp, "timestamp")?,
            status: tx.status,
            description: tx.description,
        })
    }
}

impl BinRecord {
    const MAGIC: &[u8; 4] = b"YPBN";
    const EXPECTED_SIZE_WITHOUT_DESCRIPTION: u32 = 46;

    pub fn read_from(r: impl Read) -> Result<Vec<Self>, ParserError> {
        let mut reader = BufReader::new(r);
        let mut records = Vec::new();

        loop {
            let header = match Self::read_header(&mut reader) {
                Ok(header) => header,
                Err(ParserError::Io(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e),
            };

            let body = Self::read_body(&mut reader, header.record_size)?;
            records.push(body);
        }

        Ok(records)
    }

    pub fn write_to(records: &[Self], writer: &mut impl Write) -> Result<(), ParserError> {
        for record in records {
            writer.write_all(b"YPBN")?;
            let desc_len: u32 = u32::try_from(record.description.len()).map_err(|_| {
                ParserError::InvalidFormat {
                    reason: "DESCRIPTION is too long".to_string(),
                }
            })?;
            let record_size = Self::EXPECTED_SIZE_WITHOUT_DESCRIPTION
                .checked_add(desc_len)
                .ok_or_else(|| ParserError::InvalidFormat {
                    reason: "RECORD_SIZE overflow".to_string(),
                })?;
            writer.write_all(&record_size.to_be_bytes())?;

            writer.write_all(&record.tx_id.to_be_bytes())?;
            writer.write_all(&[record.tx_type.into()])?;
            writer.write_all(&record.from_user_id.to_be_bytes())?;
            writer.write_all(&record.to_user_id.to_be_bytes())?;
            writer.write_all(&record.amount.to_be_bytes())?;
            writer.write_all(&record.timestamp.to_be_bytes())?;
            writer.write_all(&[record.status.into()])?;
            writer.write_all(&desc_len.to_be_bytes())?;
            writer.write_all(record.description.as_bytes())?;
        }

        Ok(())
    }

    fn read_header(reader: &mut impl Read) -> Result<BinRecordHeader, ParserError> {
        let mut magic = [0; 4];
        reader.read_exact(&mut magic)?;

        if &magic != Self::MAGIC {
            return Err(ParserError::InvalidFormat {
                reason: "Invalid MAGIC".to_string(),
            });
        }

        Ok(BinRecordHeader {
            magic,
            record_size: Self::read_u32_be(reader)?,
        })
    }

    fn read_body(reader: &mut impl Read, record_size: u32) -> Result<BinRecord, ParserError> {
        let tx_id = Self::read_u64_be(reader)?;
        let tx_type = TxType::try_from(Self::read_u8(reader)?)?;
        let from_user_id = Self::read_u64_be(reader)?;
        let to_user_id = Self::read_u64_be(reader)?;
        let amount = Self::read_i64_be(reader)?;
        let timestamp = Self::read_u64_be(reader)?;
        let status = TransactionStatus::try_from(Self::read_u8(reader)?)?;
        let desc_len = Self::read_u32_be(reader)?;

        let record_size_without_description =
            record_size
                .checked_sub(desc_len)
                .ok_or_else(|| ParserError::InvalidFormat {
                    reason: format!("RECORD_SIZE {} < DESC_SIZE {}", record_size, desc_len),
                })?;

        if Self::EXPECTED_SIZE_WITHOUT_DESCRIPTION != record_size_without_description {
            return Err(ParserError::InvalidFormat {
                reason: format!(
                    "RECORD_SIZE mismatch. Expected {}, got {}+{}",
                    record_size,
                    Self::EXPECTED_SIZE_WITHOUT_DESCRIPTION,
                    desc_len
                ),
            });
        }

        let description = if desc_len != 0 {
            Self::read_string(reader, desc_len)?
        } else {
            String::new()
        };

        Ok(Self {
            tx_id,
            tx_type,
            from_user_id,
            to_user_id,
            amount,
            timestamp,
            status,
            description,
        })
    }

    fn read_u32_be(reader: &mut impl Read) -> Result<u32, ParserError> {
        let mut buf = [0; 4];
        reader.read_exact(&mut buf)?;
        Ok(u32::from_be_bytes(buf))
    }

    fn read_u64_be(reader: &mut impl Read) -> Result<u64, ParserError> {
        let mut buf = [0; 8];
        reader.read_exact(&mut buf)?;
        Ok(u64::from_be_bytes(buf))
    }

    fn read_i64_be(reader: &mut impl Read) -> Result<i64, ParserError> {
        let mut buf = [0; 8];
        reader.read_exact(&mut buf)?;
        Ok(i64::from_be_bytes(buf))
    }

    fn read_u8(reader: &mut impl Read) -> Result<u8, ParserError> {
        let mut buf = [0];
        reader.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    fn read_string(reader: &mut impl Read, size: u32) -> Result<String, ParserError> {
        let mut buf = vec![0u8; size as usize];
        reader.read_exact(&mut buf)?;
        String::from_utf8(buf).map_err(|e| ParserError::InvalidField {
            field: "DESCRIPTION",
            reason: e.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn read_from_valid_bin() {
        let mut data = Vec::new();

        // Record 1
        data.extend_from_slice(b"YPBN"); // MAGIC | YPBN
        let description = b"Terminal deposit";
        let record_size = 46u32 + description.len() as u32;
        data.extend_from_slice(&record_size.to_be_bytes()); // RECORD_SIZE | 46 + 16
        data.extend_from_slice(&123u64.to_be_bytes()); // TX_ID | 123
        data.push(0); // TX_TYPE | DEPOSIT
        data.extend_from_slice(&0u64.to_be_bytes()); // FROM_USER_ID | 0
        data.extend_from_slice(&501u64.to_be_bytes()); // TO_USER_ID | 501
        data.extend_from_slice(&10000i64.to_be_bytes()); // AMOUNT | 10000
        data.extend_from_slice(&1633036800000u64.to_be_bytes()); // TIMESTAMP | 1633036800000
        data.push(0); // STATUS | SUCCESS
        data.extend_from_slice(&(description.len() as u32).to_be_bytes()); // DESC_LEN | 16
        data.extend_from_slice(description); // DESCRIPTION | Terminal deposit

        // Record 2
        data.extend_from_slice(b"YPBN"); // MAGIC | YPBN
        let description = b"User transfer";
        let record_size = 46u32 + description.len() as u32;
        data.extend_from_slice(&record_size.to_be_bytes()); // RECORD_SIZE | 46 + 13
        data.extend_from_slice(&2312321321321321u64.to_be_bytes()); // TX_ID | 2312321321321321
        data.push(1); // TX_TYPE | Transfer
        data.extend_from_slice(&1231231231231231u64.to_be_bytes()); // FROM_USER_ID | 1231231231231231
        data.extend_from_slice(&9876543210987654u64.to_be_bytes()); // TO_USER_ID | 9876543210987654
        data.extend_from_slice(&1000i64.to_be_bytes()); // AMOUNT | 1000
        data.extend_from_slice(&1633056800000u64.to_be_bytes()); // TIMESTAMP | 1633056800000
        data.push(1); // STATUS | Failure
        data.extend_from_slice(&(description.len() as u32).to_be_bytes()); // DESC_LEN | 13
        data.extend_from_slice(description); // DESCRIPTION | User transfer

        // Record 3
        data.extend_from_slice(b"YPBN"); // MAGIC | YPBN
        let description = b"User withdrawal";
        let record_size = 46u32 + description.len() as u32;
        data.extend_from_slice(&record_size.to_be_bytes()); // RECORD_SIZE | 46 + 15
        data.extend_from_slice(&3213213213213213u64.to_be_bytes()); // TX_ID | 3213213213213213
        data.push(2); // TX_TYPE | Withdrawal
        data.extend_from_slice(&9876543210987654u64.to_be_bytes()); // FROM_USER_ID | 9876543210987654
        data.extend_from_slice(&0u64.to_be_bytes()); // TO_USER_ID | 0
        data.extend_from_slice(&100i64.to_be_bytes()); // AMOUNT | 100
        data.extend_from_slice(&1633066800000u64.to_be_bytes()); // TIMESTAMP | 1633066800000
        data.push(0); // STATUS | SUCCESS
        data.extend_from_slice(&(description.len() as u32).to_be_bytes()); // DESC_LEN | 16
        data.extend_from_slice(description); // DESCRIPTION | User withdrawal

        let data = Cursor::new(data);
        let records = BinRecord::read_from(data).unwrap();

        assert_eq!(records.len(), 3);
        assert_eq!(
            records[0],
            BinRecord {
                tx_id: 123,
                tx_type: TxType::Deposit,
                from_user_id: 0,
                to_user_id: 501,
                amount: 10000,
                timestamp: 1633036800000,
                status: TransactionStatus::Success,
                description: "Terminal deposit".to_string()
            }
        );
        assert_eq!(
            records[1],
            BinRecord {
                tx_id: 2312321321321321,
                tx_type: TxType::Transfer,
                from_user_id: 1231231231231231,
                to_user_id: 9876543210987654,
                amount: 1000,
                timestamp: 1633056800000,
                status: TransactionStatus::Failure,
                description: "User transfer".to_string()
            }
        );
        assert_eq!(
            records[2],
            BinRecord {
                tx_id: 3213213213213213,
                tx_type: TxType::Withdrawal,
                from_user_id: 9876543210987654,
                to_user_id: 0,
                amount: 100,
                timestamp: 1633066800000,
                status: TransactionStatus::Success,
                description: "User withdrawal".to_string()
            }
        );
    }

    #[test]
    fn read_from_empty() {
        let data = Cursor::new("");
        let records = BinRecord::read_from(data).unwrap();

        assert_eq!(records.len(), 0);
    }

    #[test]
    fn read_from_corrupted_record_wrong_magic() {
        let mut data = Vec::new();

        // Record 1
        data.extend_from_slice(b"ABCD"); // MAGIC | ABCD
        let description = b"Terminal deposit";
        let record_size = 46u32 + description.len() as u32;
        data.extend_from_slice(&record_size.to_be_bytes()); // RECORD_SIZE | 46 + 16
        data.extend_from_slice(&123u64.to_be_bytes()); // TX_ID | 123
        data.push(0); // TX_TYPE | DEPOSIT
        data.extend_from_slice(&0u64.to_be_bytes()); // FROM_USER_ID | 0
        data.extend_from_slice(&501u64.to_be_bytes()); // TO_USER_ID | 501
        data.extend_from_slice(&10000i64.to_be_bytes()); // AMOUNT | 10000
        data.extend_from_slice(&1633036800000u64.to_be_bytes()); // TIMESTAMP | 1633036800000
        data.push(0); // STATUS | SUCCESS
        data.extend_from_slice(&(description.len() as u32).to_be_bytes()); // DESC_LEN | 16
        data.extend_from_slice(description); // DESCRIPTION | Terminal deposit

        let data = Cursor::new(data);
        let err = BinRecord::read_from(data).unwrap_err();
        assert!(matches!(err, ParserError::InvalidFormat { .. }));
    }

    #[test]
    fn read_from_corrupted_record_record_size_mismatch() {
        let mut data = Vec::new();

        // Record 1
        data.extend_from_slice(b"YPBN"); // MAGIC | YPBN
        let description = b"Terminal deposit";
        let record_size = 46u32 + 4;
        data.extend_from_slice(&record_size.to_be_bytes()); // RECORD_SIZE | 46 + 4
        data.extend_from_slice(&123u64.to_be_bytes()); // TX_ID | 123
        data.push(0); // TX_TYPE | DEPOSIT
        data.extend_from_slice(&0u64.to_be_bytes()); // FROM_USER_ID | 0
        data.extend_from_slice(&501u64.to_be_bytes()); // TO_USER_ID | 501
        data.extend_from_slice(&10000i64.to_be_bytes()); // AMOUNT | 10000
        data.extend_from_slice(&1633036800000u64.to_be_bytes()); // TIMESTAMP | 1633036800000
        data.push(0); // STATUS | SUCCESS
        data.extend_from_slice(&(description.len() as u32).to_be_bytes()); // DESC_LEN | 16
        data.extend_from_slice(description); // DESCRIPTION | Terminal deposit

        let data = Cursor::new(data);
        let err = BinRecord::read_from(data).unwrap_err();
        assert!(matches!(err, ParserError::InvalidFormat { .. }));
    }

    #[test]
    fn read_from_invalid_field_tx_type() {
        let mut data = Vec::new();

        data.extend_from_slice(b"YPBN"); // MAGIC | YPBN
        let description = b"Terminal deposit";
        let record_size = 46u32 + description.len() as u32;
        data.extend_from_slice(&record_size.to_be_bytes()); // RECORD_SIZE | 46 + 16
        data.extend_from_slice(&123u64.to_be_bytes()); // TX_ID | 123
        data.push(9); // TX_TYPE | UNKNOWN
        data.extend_from_slice(&0u64.to_be_bytes()); // FROM_USER_ID | 0
        data.extend_from_slice(&501u64.to_be_bytes()); // TO_USER_ID | 501
        data.extend_from_slice(&10000i64.to_be_bytes()); // AMOUNT | 10000
        data.extend_from_slice(&1633036800000u64.to_be_bytes()); // TIMESTAMP | 1633036800000
        data.push(0); // STATUS | SUCCESS
        data.extend_from_slice(&(description.len() as u32).to_be_bytes()); // DESC_LEN | 16
        data.extend_from_slice(description); // DESCRIPTION | Terminal deposit

        let data = Cursor::new(data);
        let err = BinRecord::read_from(data).unwrap_err();
        assert!(matches!(
            err,
            ParserError::InvalidField {
                field: "TX_TYPE",
                ..
            }
        ));
    }

    #[test]
    fn read_from_valid_bin_dec_len_is_zero() {
        let mut data = Vec::new();

        // Record 1
        data.extend_from_slice(b"YPBN"); // MAGIC | YPBN
        let record_size = 46u32;
        data.extend_from_slice(&record_size.to_be_bytes()); // RECORD_SIZE | 46
        data.extend_from_slice(&123u64.to_be_bytes()); // TX_ID | 123
        data.push(0); // TX_TYPE | DEPOSIT
        data.extend_from_slice(&0u64.to_be_bytes()); // FROM_USER_ID | 0
        data.extend_from_slice(&501u64.to_be_bytes()); // TO_USER_ID | 501
        data.extend_from_slice(&10000i64.to_be_bytes()); // AMOUNT | 10000
        data.extend_from_slice(&1633036800000u64.to_be_bytes()); // TIMESTAMP | 1633036800000
        data.push(0); // STATUS | SUCCESS
        data.extend_from_slice(&0u32.to_be_bytes()); // DESC_LEN | 0

        let data = Cursor::new(data);
        let records = BinRecord::read_from(data).unwrap();

        assert_eq!(records.len(), 1);
        assert_eq!(
            records[0],
            BinRecord {
                tx_id: 123,
                tx_type: TxType::Deposit,
                from_user_id: 0,
                to_user_id: 501,
                amount: 10000,
                timestamp: 1633036800000,
                status: TransactionStatus::Success,
                description: String::new()
            }
        );
    }

    #[test]
    fn write_and_read_back() {
        let original = vec![BinRecord {
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
        BinRecord::write_to(&original, &mut buffer).unwrap();

        let read_back = BinRecord::read_from(Cursor::new(buffer)).unwrap();
        assert_eq!(original, read_back);
    }
}
