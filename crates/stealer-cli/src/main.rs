use std::process::exit;

use cli::{get_args, Commands, DebugLevel, ScanParams, ServerParams, TargetParams};

fn main() {
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

    // check running mode
    // As this is a CLI crate, we need to make sure that `no-gui` is assigned.
    if let Some(mode) = args.command {
        match mode {
            Commands::NoGui {
                server_params,
                target_params,
                scan_params,
            } => {
                headless(scan_params, server_params, target_params);
            }
            Commands::Gui => {
                log::error!(
                    "`gui` subcommand detected. This is a CLI crate, so gui is not supported."
                );
                println!("This is a CLI crate, so `gui` subcommand is not supported. You can only specify `no-gui` subcommand.");
            }
        }
    } else {
        log::error!("No subcommand provided.");
        println!("You need to explicitly specify `no-gui` subcommand!");
        exit(1)
    }
}

pub fn headless(scan_params: ScanParams, server_params: ServerParams, target_params: TargetParams) {
    log::info!("Executing headless mode.");
}
