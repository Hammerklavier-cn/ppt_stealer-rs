use std::process::exit;

mod watch_dog;

use cli::{Commands, DebugLevel, ScanParams, ServerParams, TargetParams, UploadTarget, get_args};
use file_management::{
    LocalTargetManager, SshKeyAuthentication, SshPasswordAuthentication, SshRemoteAuthentication,
    SshTargetManager, TargetManager,
};
use walkdir::WalkDir;

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
    {
        let mut remote_server_selected = false;
        for upload_target in target_params.upload_targets {
            let target_manager: Box<dyn TargetManager> = match upload_target {
                UploadTarget::Local => Box::new(LocalTargetManager::new(
                    target_params.target_folder_name.as_deref(),
                )) as Box<dyn TargetManager>,

                UploadTarget::SshServer => {
                    if !remote_server_selected {
                        remote_server_selected = true;

                        let login_params = match server_params.password.as_deref() {
                            Some(passwd) => SshPasswordAuthentication {
                                ip: server_params.ip.as_deref().unwrap(),
                                port: server_params.port.unwrap(),
                                username: server_params.username.as_deref().unwrap(),
                                password: passwd,
                            },
                            None => panic!("KeyAuth is currently unsupported!"),
                        };
                        Box::new(SshTargetManager::new(Some("base_path"), login_params))
                            as Box<dyn TargetManager>
                    } else {
                        continue;
                    }
                }

                UploadTarget::SmbServer | UploadTarget::FtpServer => {
                    if !remote_server_selected {
                        remote_server_selected = true;
                        panic!("SMB and FTP are supported yet!");
                    } else {
                        continue;
                    }
                }
            };
            target_managers.push(target_manager);
        }
    }

    // continuously scan and upload
    loop {
        // get source directories
        let source_pathbuf_set = watch_dog::get_source_directories(&scan_params);
    }
}
