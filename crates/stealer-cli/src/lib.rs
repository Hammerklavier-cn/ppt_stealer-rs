use log;
use std::{collections::BTreeSet, process::exit};

mod watch_dog;

use cli::{Commands, DebugLevel, ScanParams, ServerParams, TargetParams, UploadTarget, get_args};
use file_management::{
    LocalSourceManager, LocalTargetManager, SshKeyAuthentication, SshPasswordAuthentication,
    SshRemoteAuthentication, SshTargetManager, TargetManager,
};

pub fn headless(scan_params: ScanParams, server_params: ServerParams, target_params: TargetParams) {
    log::info!("Executing headless function.");

    // determine base directory

    // connect to target file manager
    log::info!("Targets: {:?}", target_params.upload_targets);
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
        log::debug!("Loop.");
        // get source directories
        // As source might include mutable divices (like usb), it must be refreshed periodically.
        let source_pathbuf_set = match watch_dog::get_source_directories(&scan_params) {
            Ok(directory_set) => directory_set,
            Err(_) => continue,
        };

        let mut local_target_managers = BTreeSet::new();
        for source_pathbuf in source_pathbuf_set {
            local_target_managers.insert(LocalSourceManager {
                base_path: source_pathbuf,
            });
            println!("{:?}", &local_target_managers)
        }
    }
}
