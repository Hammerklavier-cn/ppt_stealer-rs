use anyhow::{Error, Result};
use log;
use std::{cell::RefCell, collections::BTreeSet, rc::Rc};
use watch_dog::get_source_directories;

pub(crate) mod watch_dog;

use cli::{ScanParams, ServerParams, TargetParams, UploadTarget};
use file_management::{
    LocalSourceManager, LocalTargetManager, SshPasswordAuthentication, SshTargetManager,
};

// 新增：定义一个新的 trait 用于擦除关联类型
trait ErasedTargetManager {
    fn receive_from_folder(
        &self,
        local_source_folder: &LocalSourceManager,
        exts: &[&str],
        regex: Option<&str>,
        min_depth: Option<usize>,
        max_depth: Option<usize>,
    ) -> Result<(), anyhow::Error>;
}

// 新增：为 LocalTargetManager 实现 ErasedTargetManager
impl ErasedTargetManager for LocalTargetManager {
    fn receive_from_folder(
        &self,
        local_source_folder: &LocalSourceManager,
        exts: &[&str],
        regex: Option<&str>,
        min_depth: Option<usize>,
        max_depth: Option<usize>,
    ) -> Result<(), anyhow::Error> {
        let local_target_manager = Rc::new(RefCell::new(self.clone()));
        local_source_folder.upload_to_folder(
            local_target_manager,
            exts,
            regex,
            min_depth,
            max_depth,
        )
    }
}

// 新增：为 SshTargetManager 实现 ErasedTargetManager
impl ErasedTargetManager for SshTargetManager {
    fn receive_from_folder(
        &self,
        local_source_folder: &LocalSourceManager,
        exts: &[&str],
        regex: Option<&str>,
        min_depth: Option<usize>,
        max_depth: Option<usize>,
    ) -> Result<(), anyhow::Error> {
        let local_target_manager = Rc::new(RefCell::new(self.clone()));
        local_source_folder.upload_to_folder(
            local_target_manager,
            exts,
            regex,
            min_depth,
            max_depth,
        )
    }
}

pub fn headless(
    scan_params: ScanParams,
    server_params: ServerParams,
    target_params: TargetParams,
) -> Result<(), Error> {
    log::info!("Executing headless function.");

    // connect to target file manager
    log::info!("Targets: {:?}", target_params.upload_targets);
    // 修改：使用新的 trait 对象类型
    let mut target_managers: Vec<Box<dyn ErasedTargetManager>> = vec![];
    {
        let mut remote_server_selected = false;
        for upload_target in target_params.upload_targets {
            if remote_server_selected {
                panic!("Only one remote server can be selected!");
            }
            match upload_target {
                UploadTarget::Local => {
                    let manager =
                        LocalTargetManager::new(target_params.target_folder_name.as_deref());
                    target_managers.push(Box::new(manager) as Box<dyn ErasedTargetManager>);
                }
                UploadTarget::SshServer => {
                    if !remote_server_selected {
                        remote_server_selected = true;
                        let login_params = match server_params.password.as_deref() {
                            Some(passwd) => SshPasswordAuthentication {
                                ip: server_params.ip.as_ref().unwrap().clone(),
                                port: server_params.port.unwrap(),
                                username: server_params.username.as_ref().unwrap().clone(),
                                password: passwd.to_string(),
                            },
                            None => panic!("KeyAuth is currently unsupported!"),
                        };
                        let manager = SshTargetManager::new(
                            target_params.target_folder_name.as_deref(),
                            login_params,
                        );
                        target_managers.push(Box::new(manager) as Box<dyn ErasedTargetManager>);
                    }
                }
                UploadTarget::SmbServer | UploadTarget::FtpServer => {
                    panic!("SMB and FTP are supported yet!");
                    // if !remote_server_selected {
                    //     remote_server_selected = true;
                    // }
                }
            };
        }
    }

    // continuously scan and upload
    loop {
        log::debug!("Loop.");
        // get source directories
        // As source might include mutable divices (like usb), it must be refreshed periodically.
        let source_directories = get_source_directories(&scan_params)?;
        let mut source_managers: BTreeSet<LocalSourceManager> = BTreeSet::new();
        for dir in source_directories {
            source_managers.insert(LocalSourceManager { base_path: dir });
        }
        // let mut local_target_managers = BTreeSet::new();
        // for source_pathbuf in source_pathbuf_set {
        //     local_target_managers.insert(LocalSourceManager {
        //         base_path: source_pathbuf,
        //     });
        //     println!("{:?}", &local_target_managers)
        // }
        for target_manager in target_managers.iter() {
            for local_manager in source_managers.iter() {
                target_manager.receive_from_folder(
                    local_manager,
                    &scan_params
                        .formats
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>(),
                    scan_params.regex.as_deref(),
                    scan_params.min_depth,
                    scan_params.max_depth,
                )?;
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(scan_params.refresh_interval));
    }
}
