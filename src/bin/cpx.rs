use clap::Parser;
use colored::Colorize;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(flatten)]
    verbosity: clap_verbosity_flag::Verbosity<clap_verbosity_flag::InfoLevel>,

    /// Overwrite existing files
    #[arg(short = 'f', long)]
    force: bool,

    /// Show what would be done without actually doing it
    #[arg(short = 'n', long, env = "MODE_DRY_RUN", value_parser = clap::builder::FalseyValueParser::new())]
    dry_run: bool,

    /// Paths to copy from
    #[arg(required = true)]
    srcs: Vec<PathBuf>,

    /// Path to copy or merge to
    dest: PathBuf,
}

fn main() {
    let cli = Cli::parse();
    let mp = mvx::init_logging(cli.verbosity.log_level_filter());
    let ctrlc = mvx::ctrlc_flag().unwrap();
    log::trace!("{cli:?}");

    let ctx = mvx::Ctx {
        moc: mvx::MoveOrCopy::Copy,
        force: cli.force,
        dry_run: cli.dry_run,
        mp: &mp,
        ctrlc: &ctrlc,
    };
    if let Err(e) = mvx::run_batch(&cli.srcs, &cli.dest, &ctx) {
        eprintln!("{} {:?}", "✗".red().bold(), e);
        std::process::exit(1);
    }
}
