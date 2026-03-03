use clap::{Parser, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "sc4n",
    about = "Stealthy adaptive network scanner",
    version = "0.1.0",
    author = "RowanDark"
)]
pub struct Cli {
    /// Target host or IP address
    #[arg(short = 'H', long)]
    pub host: String,

    /// Port range to scan (e.g. 1-1024, 80, 22,80,443)
    #[arg(short = 'p', long, default_value = "1-1024")]
    pub ports: String,

    /// Scan profile
    #[arg(short = 'P', long, value_enum, default_value = "balanced")]
    pub profile: ProfileArg,

    /// Number of concurrent probes (overrides profile)
    #[arg(short = 'c', long)]
    pub concurrency: Option<usize>,

    /// Requests per second limit (overrides profile, 0 = unlimited)
    #[arg(short = 'r', long)]
    pub rate: Option<u64>,

    /// Output file path
    #[arg(short = 'o', long, default_value = "sc4n_results.jsonl")]
    pub output: String,

    /// Timeout per probe in milliseconds (overrides profile)
    #[arg(short = 't', long)]
    pub timeout_ms: Option<u64>,

    /// Show all ports including closed (debug mode)
    #[arg(short = 'd', long, default_value = "false")]
    pub debug: bool,

    /// Disable port scan order randomization
    #[arg(long, default_value = "false")]
    pub no_randomize: bool,

    /// Suppress banner output, print results only
    #[arg(short = 'q', long, default_value = "false")]
    pub quiet: bool,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum ProfileArg {
    Aggressive,
    Balanced,
    Stealth,
    Paranoid,
}
