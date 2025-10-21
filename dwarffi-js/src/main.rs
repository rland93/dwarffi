use anyhow::Result;
use clap::Parser;
use log::{debug, info, warn};
use std::path::PathBuf;

/// dwarffi-js - extract C FFI signatures and generate JavaScript bindings
#[derive(Parser)]
#[command(name = "dwarffi-js")]
#[command(version)]
#[command(about = "extract function signatures from C libraries using DWARF debug info", long_about = None)]
struct Cli {
    /// path to the library file (.dylib, .so, .o, or dSYM)
    library: PathBuf,

    /// show all functions (including internal/hidden ones)
    #[arg(long)]
    all: bool,

    /// suppress informational messages (only show signatures)
    #[arg(short = 'q', long)]
    quiet: bool,

    /// verbose logging to console (-v for info, -vv for debug, -vvv for trace)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Output JavaScript bindings using ref-struct-di
    #[arg(long)]
    js: bool,

    /// Output JSON representation of types and functions
    #[arg(short = 'j', long)]
    json: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

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
    let analyzer = dwarffi::DwarfAnalyzer::from_file(&cli.library)?;

    let result = analyzer.extract_analysis(exported_only)?;

    if result.signatures.is_empty() {
        warn!(
            "no functions found in the library. maybe you compiled without debug info, or stripped the binary?"
        );
        return Ok(());
    }

    // sort signatures by name for consistent output
    let mut sorted_sigs = result.signatures;
    sorted_sigs.sort_by(|a, b| a.name.cmp(&b.name));

    if cli.json {
        unimplemented!("JSON output not yet implemented");
    } else if cli.js {
        unimplemented!("JavaScript output not yet implemented");
    } else {
        // standard C signature output
        for sig in &sorted_sigs {
            println!("{};", sig.to_string(&result.type_registry));
        }
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
