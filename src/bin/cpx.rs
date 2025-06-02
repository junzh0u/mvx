use std::path::PathBuf;

use clap::Parser;
use clap_verbosity_flag::{InfoLevel, Verbosity};
use colored::Colorize;
use mvx::{MoveOrCopy, init_logging, run_batch};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(flatten)]
    verbosity: Verbosity<InfoLevel>,

    /// Paths to copy from
    #[arg(required = true)]
    srcs: Vec<PathBuf>,

    /// Path to copy or merge to
    dest: PathBuf,
}

fn main() {
    let cli = Cli::parse();
    let mp = init_logging(cli.verbosity.log_level_filter());
    log::trace!("{cli:?}");

    if let Err(e) = run_batch(cli.srcs, cli.dest, mp.as_ref(), &MoveOrCopy::Copy) {
        eprintln!("{} {:?}", "✗".red().bold(), e);
        std::process::exit(1);
    }
}
