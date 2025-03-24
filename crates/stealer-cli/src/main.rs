use std::process::exit;

mod watch_dog;

use cli::{Commands, DebugLevel, ScanParams, ServerParams, TargetParams, UploadTarget, get_args};
use file_management::{
    LocalTargetManager, RemoteAuthentication, SshKeyAuthentication, SshPasswordAuthentication,
    SshTargetManager, TargetManager,
};

fn main() {
    let args = get_args();

    // set up logging level
    unsafe {
        std::env::set_var(
            "RUST_LOG",
            match args.debug_level {
                DebugLevel::Trace => "trace",
                DebugLevel::Debug => "debug",
                DebugLevel::Info => "info",
                DebugLevel::Warn => "warn",
                DebugLevel::Error => "error",
            },
        )
    };
    env_logger::init();

    println!(
        "{} {}, by {}\n{}",
        std::env!("CARGO_PKG_NAME"),
        std::env!("CARGO_PKG_VERSION"),
        std::env!("CARGO_PKG_AUTHORS"),
        std::env!("CARGO_PKG_DESCRIPTION")
    );

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
                println!(
                    "This is a CLI crate, so `gui` subcommand is not supported. You can only specify `no-gui` subcommand."
                );
            }
        }
    } else {
        log::error!("No subcommand provided.");
        println!("You need to explicitly specify `no-gui` subcommand!");
        exit(1)
    }
}

pub fn headless(scan_params: ScanParams, server_params: ServerParams, target_params: TargetParams) {
    log::info!("Executing headless function.");

    // determine base directory

    // connect to target file manager
    let mut target_managers: Vec<Box<dyn TargetManager>> = vec![];
    for upload_target in target_params.upload_targets {
        let target_manager = match upload_target {
            UploadTarget::Local => Box::new(LocalTargetManager::new(
                target_params.target_folder_name.as_deref(),
            )) as Box<dyn TargetManager>,
            UploadTarget::SshServer => {
                let login_params = match &server_params.password {
                    Some(passwd) => SshPasswordAuthentication {
                        ip: &server_params.ip.unwrap(),
                        port: server_params.port.unwrap(),
                        username: &server_params.username.unwrap(),
                        password: &passwd,
                    },
                    None => panic!("KeyAuth is currently unsupported!"),
                };
                Box::new(SshTargetManager::new(Some("base_path"), login_params))
            }
            UploadTarget::SmbServer | UploadTarget::FtpServer => panic!("Not supported yet!"),
        };
        target_managers.push(target_manager);
    }

    // continuously scan and upload
    loop {
        // get source directories
        let source_pathbuf_set = watch_dog::get_source_directories(&scan_params);
    }
}
