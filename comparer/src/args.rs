use clap::Parser;

/// Compare two transaction files
#[derive(Parser)]
pub struct Cli {
    /// file1 path
    #[arg(long)]
    pub file1: String,

    /// file1 format (csv, text, bin)
    #[arg(long)]
    pub format1: String,

    /// file2 path
    #[arg(long)]
    pub file2: String,

    /// file2 format (csv, text, bin)
    #[arg(long)]
    pub format2: String,
}
