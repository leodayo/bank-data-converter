use clap::Parser;

/// Converter for Transactions between following formats: CSV, TEXT, BIN
#[derive(Parser)]
pub struct Cli {
    /// optional input file path, reads from stdin if omitted
    #[arg(long)]
    pub input: Option<String>,

    /// imput file format (csv, text, bin)
    #[arg(long)]
    pub input_format: String,

    /// output file format (csv, text, bin)
    #[arg(long)]
    pub output_format: String,
}
