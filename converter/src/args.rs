use clap::Parser;

/// Converter for Transactions between following formats: CSV, TEXT, BIN
#[derive(Parser)]
pub struct Cli {
    /// input file path
    #[arg(long)]
    pub input: String,

    /// imput file format (csv, text, bin)
    #[arg(long)]
    pub input_format: String,

    /// output file format (csv, text, bin)
    #[arg(long)]
    pub output_format: String,
}
