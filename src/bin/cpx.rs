use clap::Parser;
use colored::Colorize;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(flatten)]
    verbosity: clap_verbosity_flag::Verbosity<clap_verbosity_flag::InfoLevel>,

    /// Paths to copy from
    #[arg(required = true)]
    srcs: Vec<PathBuf>,

    /// Path to copy or merge to
    dest: PathBuf,
}

fn main() {
    let cli = Cli::parse();
    let mp = mvx::init_logging(cli.verbosity.log_level_filter());
    log::trace!("{cli:?}");

    if let Err(e) = mvx::run_batch(&cli.srcs, &cli.dest, mp.as_ref(), &mvx::MoveOrCopy::Copy) {
        eprintln!("{} {:?}", "✗".red().bold(), e);
        std::process::exit(1);
    }
}
