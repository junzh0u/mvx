use clap::Parser;
use mvx::{Cli, run};

fn main() {
    match run(&Cli::parse()) {
        Ok(()) => (),
        Err(e) => {
            eprintln!("Error: {e:?}");
            std::process::exit(1);
        }
    }
}
