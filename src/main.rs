use anyhow::Result;
use clap::Parser;
use ffitool::DwarfAnalyzer;
use log::{debug, info, warn};
use std::path::PathBuf;

/// ffitool - extract function signatures from C libraries using DWARF debug information!
#[derive(Parser)]
#[command(name = "ffitool")]
#[command(version)]
#[command(about = "extract function signatures from C libraries using DWARF debug info", long_about = None)]
struct Cli {
    /// path to the library file (.dylib, .so, .o, or dSYM)
    library: PathBuf,

    /// show all functions (including internal/hidden ones)
    #[arg(short, long)]
    all: bool,

    /// suppress informational messages (only show signatures)
    #[arg(short, long)]
    quiet: bool,

    /// verbose logging to console (-v for info, -vv for debug, -vvv for trace)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // initialize logger based on verbosity level and quiet flag
    init_logger(cli.verbose, cli.quiet);

    let exported_only = !cli.all;

    info!("library: {}", cli.library.display());
    info!(
        "mode: {}",
        if exported_only {
            "exported only"
        } else {
            "all functions"
        }
    );

    // load the library
    debug!("load library file: {}", cli.library.display());
    let analyzer = DwarfAnalyzer::from_file(&cli.library)?;

    // Extract function signatures
    let signatures = analyzer.extract_signatures(exported_only)?;

    if signatures.is_empty() {
        warn!(
            "no functions found in the library. maybe you compiled without debug info, or stripped the binary?"
        );
        return Ok(());
    }

    // sort signatures by name for consistent output
    let mut sorted_sigs = signatures;
    sorted_sigs.sort_by(|a, b| a.name.cmp(&b.name));

    // print each signature
    for sig in sorted_sigs {
        println!("{};", sig.to_string());
    }

    Ok(())
}

fn init_logger(verbose: u8, quiet: bool) {
    // If quiet mode is enabled, only show warnings and errors
    let log_level = if quiet {
        log::LevelFilter::Warn
    } else {
        match verbose {
            0 => log::LevelFilter::Error,
            1 => log::LevelFilter::Info,
            2 => log::LevelFilter::Debug,
            _ => log::LevelFilter::Trace,
        }
    };

    env_logger::Builder::from_default_env()
        .filter_level(log_level)
        .format_timestamp(None)
        .format_module_path(false)
        .format_target(false)
        .init();
}
