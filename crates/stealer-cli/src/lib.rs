use anyhow::Result;
use log;
use std::{cell::RefCell, collections::BTreeSet, rc::Rc};

mod watch_dog;

use cli::{ScanParams, ServerParams, TargetParams, UploadTarget};
use file_management::{
    LocalSourceManager, LocalTargetManager, SshPasswordAuthentication, SshTargetManager,
    TargetFile, TargetManager,
};

// 新增：定义一个新的 trait 用于擦除关联类型
trait ErasedTargetManager {
    fn receive_from_folder(
        &self,
        local_source_folder: LocalSourceManager,
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
        local_source_folder: LocalSourceManager,
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
        local_source_folder: LocalSourceManager,
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

pub fn headless(scan_params: ScanParams, server_params: ServerParams, target_params: TargetParams) {
    log::info!("Executing headless function.");

    // determine base directory

    // connect to target file manager
    log::info!("Targets: {:?}", target_params.upload_targets);
    // 修改：使用新的 trait 对象类型
    let mut target_managers: Vec<Box<dyn ErasedTargetManager>> = vec![];
    {
        let mut remote_server_selected = false;
        for upload_target in target_params.upload_targets {
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
                        let manager = SshTargetManager::new(Some("base_path"), login_params);
                        target_managers.push(Box::new(manager) as Box<dyn ErasedTargetManager>);
                    }
                }
                UploadTarget::SmbServer | UploadTarget::FtpServer => {
                    if !remote_server_selected {
                        remote_server_selected = true;
                        panic!("SMB and FTP are supported yet!");
                    }
                }
            };
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
