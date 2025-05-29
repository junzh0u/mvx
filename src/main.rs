use clap::Parser;
use mvx::{Cli, run};

fn main() {
    if let Err(e) = run(&Cli::parse()) {
        eprintln!("Error: {e:?}");
        std::process::exit(1);
    }
}
