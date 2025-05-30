use std::path::PathBuf;

use clap::Parser;
use clap_verbosity_flag::{InfoLevel, Verbosity};
use colored::Colorize;
use indicatif::MultiProgress;
use mvx::run;
use std::io::Write;

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

    let mp = (cli.verbosity.log_level() >= Some(log::Level::Info)).then(MultiProgress::new);
    let mp_clone = mp.clone();

    env_logger::Builder::new()
        .filter_level(cli.verbosity.log_level_filter())
        .format(move |buf, record| {
            let ts = chrono::Local::now().to_rfc3339().bold();
            let file_and_line = format!(
                "[{:>10}:{:<3}]",
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
            )
            .italic();
            let level = match record.level() {
                log::Level::Error => "ERROR".red(),
                log::Level::Warn => "WARN ".yellow(),
                log::Level::Info => "INFO ".green(),
                log::Level::Debug => "DEBUG".blue(),
                log::Level::Trace => "TRACE".magenta(),
            }
            .bold();

            let msg = format!("{ts} {file_and_line} {level} {}", record.args());

            match &mp_clone {
                Some(mp) => mp.println(msg),
                None => writeln!(buf, "{msg}"),
            }
        })
        .init();

    if let Err(e) = run(&cli.src, &cli.dest, mp.as_ref()) {
        eprintln!("{} {:?}", "✗".red().bold(), e);
        std::process::exit(1);
    }
}
