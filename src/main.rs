use clap::Parser;
use colored::Colorize;
use mvx::{Cli, run};

fn main() {
    if let Err(e) = run(&Cli::parse()) {
        eprintln!("{} {:?}", "✗".red().bold(), e);
        std::process::exit(1);
    }
}
