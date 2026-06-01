use crate::{
    error::ParserError,
    model::{TransactionStatus, TxType},
};
use std::io::{BufRead, BufReader, Read};

#[derive(Debug, PartialEq, Eq)]
pub struct CsvRecord {
    pub tx_id: i64,
    pub tx_type: TxType,
    pub from_user_id: i64,
    pub to_user_id: i64,
    pub amount: i64,
    pub timestamp: i64,
    pub status: TransactionStatus,
    pub description: String,
}

impl CsvRecord {
    const EXPECTED_HEADER: &str =
        "TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION";
    const FIELDS_COUNT: usize = 8;

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
            description: parts[7].trim().trim_matches('"').to_string(),
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
                description: "Initial account funding".to_string()
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
                description: "Payment for services, invoice #123".to_string()
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
                description: "ATM withdrawal".to_string()
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
}
