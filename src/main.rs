use clap::Parser;
use mvx::{Cli, run};

fn main() {
    run(Cli::parse()).unwrap();
}
