use log;
use std::process::exit;

use cli::{get_args, shared_function, DebugLevel};

fn main() {
    println!("Hello, world!");
    shared_function();

    // parse command line arguments
    let args = get_args();

    // set up logging level
    std::env::set_var(
        "RUST_LOG",
        match args.debug_level {
            DebugLevel::Trace => "trace",
            DebugLevel::Debug => "debug",
            DebugLevel::Info => "info",
            DebugLevel::Warn => "warn",
            DebugLevel::Error => "error",
        },
    );
    env_logger::init();

    // determine running mode
    match args.command {
        Some(t) => match t {
            cli::Commands::Gui => {
                log::info!("GUI mode selected.");
                // TODO: Implement GUI mode
                log::error!("GUI is not supported yet!");
                exit(1)
            }
            cli::Commands::NoGui {
                server_params,
                target_params,
                scan_params,
            } => {
                log::info!("No GUI mode selected.");
            }
        },
        None => {
            log::info!("No subcommand specified. GUI mode is chosen by default.");
            log::error!("GUI is not supported yet!");
            exit(1)
        }
    };
}
