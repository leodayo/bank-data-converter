mod args;

use args::Cli;
use clap::Parser;
use parser::error::ParserError;
use parser::format::{bin::BinRecord, csv::CsvRecord, text::TextRecord};
use parser::model::Transaction;
use std::fs::File;
use std::io::{self, Read, Write};

fn main() -> Result<(), ParserError> {
    let cli = Cli::parse();

    let reader: Box<dyn Read> = match &cli.input {
        Some(path) => Box::new(File::open(path)?),
        None => Box::new(io::stdin().lock()),
    };

    let transactions = read_transactions(reader, &cli.input_format)?;
    write_transactions(transactions, &cli.output_format, &mut io::stdout().lock())?;

    Ok(())
}

fn read_transactions(reader: impl Read, format: &str) -> Result<Vec<Transaction>, ParserError> {
    match format {
        "csv" => {
            let records = CsvRecord::read_from(reader)?;
            Ok(records.into_iter().map(Transaction::from).collect())
        }
        "text" => {
            let records = TextRecord::read_from(reader)?;
            records.into_iter().map(Transaction::try_from).collect()
        }
        "bin" => {
            let records = BinRecord::read_from(reader)?;
            records.into_iter().map(Transaction::try_from).collect()
        }
        _ => Err(ParserError::InvalidFormat {
            reason: format!(
                "Unsupported input format: {}. Expected one of (csv, text, bin)",
                format
            ),
        }),
    }
}

fn write_transactions(
    transactions: Vec<Transaction>,
    format: &str,
    writer: &mut impl Write,
) -> Result<(), ParserError> {
    match format {
        "csv" => {
            let records: Vec<CsvRecord> = transactions.into_iter().map(CsvRecord::from).collect();
            CsvRecord::write_to(&records, writer)
        }
        "text" => {
            let records: Result<Vec<TextRecord>, _> =
                transactions.into_iter().map(TextRecord::try_from).collect();
            TextRecord::write_to(&records?, writer)
        }
        "bin" => {
            let records: Result<Vec<BinRecord>, _> =
                transactions.into_iter().map(BinRecord::try_from).collect();
            BinRecord::write_to(&records?, writer)
        }
        _ => Err(ParserError::InvalidFormat {
            reason: format!("Unsupported output format: {}", format),
        }),
    }
}
