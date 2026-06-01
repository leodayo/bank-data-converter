mod args;

use args::Cli;
use clap::Parser;
use parser::error::ParserError;
use parser::format::{bin::BinRecord, csv::CsvRecord, text::TextRecord};
use parser::model::Transaction;
use std::fs::File;
use std::io::{self, Write};

fn main() -> Result<(), ParserError> {
    let cli = Cli::parse();

    let transactions = read_transactions(&cli.input, &cli.input_format)?;
    write_transactions(transactions, &cli.output_format, &mut io::stdout().lock())?;

    Ok(())
}

fn read_transactions(path: &str, format: &str) -> Result<Vec<Transaction>, ParserError> {
    let file = File::open(path)?;
    match format {
        "csv" => {
            let records = CsvRecord::read_from(file)?;
            Ok(records.into_iter().map(Transaction::from).collect())
        }
        "text" => {
            let records = TextRecord::read_from(file)?;
            records.into_iter().map(Transaction::try_from).collect()
        }
        "bin" => {
            let records = BinRecord::read_from(file)?;
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
