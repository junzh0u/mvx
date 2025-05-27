use clap::Parser;
use mvx::{run, Cli};

fn main() {
    run(Cli::parse()).unwrap();
}
