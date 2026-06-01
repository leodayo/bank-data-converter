mod args;

use args::Cli;
use clap::Parser;
use parser::error::ParserError;
use parser::format::{bin::BinRecord, csv::CsvRecord, text::TextRecord};
use parser::model::Transaction;
use std::fs::File;

fn main() -> Result<(), ParserError> {
    let cli = Cli::parse();

    let transactions1 = read_transactions(&cli.file1, &cli.format1)?;
    let transactions2 = read_transactions(&cli.file2, &cli.format2)?;

    if transactions1.len() != transactions2.len() {
        println!(
            "'{}' and '{}' contain different amount of transactions.",
            cli.file1, cli.file2
        );
        return Ok(());
    }

    for (i, (t1, t2)) in transactions1.iter().zip(&transactions2).enumerate() {
        if t1 != t2 {
            println!(
                "Transactions in '{}' and '{}' aren't identical.",
                cli.file1, cli.file2
            );
            println!("First difference at index {}:\n  {:?}\n  {:?}", i, t1, t2);
            return Ok(());
        }
    }

    println!(
        "The transaction records in '{}' and '{}' are identical.",
        cli.file1, cli.file2
    );
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
            reason: format!("Unsupported format: {}", format),
        }),
    }
}
