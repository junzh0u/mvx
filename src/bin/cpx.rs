use std::path::PathBuf;

use clap::Parser;
use clap_verbosity_flag::{InfoLevel, Verbosity};
use colored::Colorize;
use mvx::{MoveOrCopy, init_logging, run};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(flatten)]
    verbosity: Verbosity<InfoLevel>,

    /// Path to move from
    src: PathBuf,

    /// Path to move or merge to
    dest: PathBuf,
}

fn main() {
    let cli = Cli::parse();
    let mp = init_logging(cli.verbosity.log_level_filter());

    if let Err(e) = run(&cli.src, &cli.dest, mp.as_ref(), &MoveOrCopy::Copy) {
        eprintln!("{} {:?}", "✗".red().bold(), e);
        std::process::exit(1);
    }
}
