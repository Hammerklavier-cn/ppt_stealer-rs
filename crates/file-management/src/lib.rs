use anyhow::{Context, Error, Result, anyhow};
use chrono::Local;
use log;
use regex::Regex;
use sha2::{Digest, Sha256};
use std::{
    cell::RefCell,
    collections::HashSet,
    ffi::OsStr,
    fs,
    hash::Hash,
    io::{self, Read},
    path::{Path, PathBuf},
    rc::Rc,
};
use walkdir::WalkDir;

fn get_default_folder_name() -> PathBuf {
    let formatted_date = Local::now().format("%Y-%m-%d").to_string();
    let computer_identifier = match gethostname::gethostname().to_str() {
        Some(name) => {
            let home_dir = dirs::home_dir().unwrap();
            format!(
                "{}--{}",
                home_dir.file_name().unwrap().to_str().unwrap(),
                name
            )
        }
        None => {
            let home_dir = dirs::home_dir().unwrap();
            home_dir.file_name().unwrap().to_str().unwrap().to_string()
        }
    };
    let path = Path::new(&formatted_date).join(Path::new(&computer_identifier));
    log::debug!("Default folder name identified as `{}`", path.display());
    path
}

// The following function is deprecated, but it still contains some useful implementation.
// pub fn get_target_base_path(identifier_path: Option<&str>, folder_name: &str) -> PathBuf {
//     let identifier_path = match identifier_path {
//         Some(path) => Path::new(path).to_path_buf(),
//         None => get_default_folder_name(),
//     };
//     identifier_path.join(
//         folder_name
//             .chars()
//             .map(|c| match c {
//                 c if c.is_ascii_alphanumeric() => c,
//                 '-' | '_' | '.' | ' ' => c,
//                 _ => '_',
//             })
//             .collect::<String>(),
//     )
// }

/// Generate a `TargetFile` from a `LocalFile`.
///
/// Example:
/// Source file is /home/user/documents/report.pptx, and LocalSourceManager has a
/// `base_path` of /home/user
/// It will generate a target file with a path of
/// `{TargetManager.base_path}/user/documents/report.pptx
pub fn convert_local_file_to_target<T>(source: &LocalFile, tm: Rc<RefCell<T>>) -> T::File
where
    // F: FolderManager,
    T: TargetManager,
{
    let root_path = PathBuf::from(&source.ltm.borrow().get_base_path())
        .canonicalize()
        .unwrap();
    let root_folder = PathBuf::from(
        root_path
            .file_name()
            .unwrap_or_else(|| OsStr::new(root_path.to_str().unwrap()))
            .to_str()
            .unwrap()
            .chars()
            .map(|c| match c {
                c if c.is_ascii_alphanumeric() => c,
                '-' | '_' | '.' | ' ' => c,
                _ => '_',
            })
            .collect::<String>(),
    );
    let abspath = source.path.canonicalize().unwrap();
    let relpath = root_folder.join(abspath.strip_prefix(&root_path).unwrap());
    TargetFile::from_relpath(&relpath, tm)
}

pub trait SingleFile {
    /// Return error if connection failed to establish
    fn is_exists(&self) -> Result<bool, Error>;
    fn get_relpath(&self) -> Result<PathBuf, Error>;
    fn get_sha256(&self) -> Result<String, Error>;
    fn get_new_sha256(&self) -> Result<String, Error>;
}

// pub trait SourceFile: SingleFile {
//     fn upload_to_folder<T: TargetManager>(
//         &self,
//         target_manager: Rc<RefCell<T>>,
//     ) -> Result<(), Error>;
//     // fn upload_to_file<T: TargetFile>(&self, target_file: &T) -> Result<(), Error>;
// }

pub trait TargetFile: SingleFile {
    type Manager: TargetManager<File = Self>;
    fn from_relpath(relpath: &Path, fm: Rc<RefCell<Self::Manager>>) -> Self;
    /// Make sure that self.path is a valid remote path. If it is not, create
    /// the folders.
    fn initialise_path(&self) -> Result<(), Error>;
    fn receive_from_file(&self, source_file: &LocalFile) -> Result<(), Error>;
}

/// This struct depicts a local file.
///
/// Attributes:
///
/// - pub path (PathBuf): path to this local file
/// - pub base_path (PathBuf): path to the foler
///   from which the local file is scanned
/// - sha256 (RefCell<Option<String>>): sha256sum of the local
///   file whose value updates lazily
#[derive(Clone)]
pub struct LocalFile {
    pub path: PathBuf,
    sha256: RefCell<Option<String>>,
    ltm: Rc<RefCell<dyn LocalFolderManager>>,
}
impl LocalFile {
    fn upload_to_folder<T: TargetManager>(
        &self,
        target_manager: Rc<RefCell<T>>,
    ) -> Result<(), Error> {
        // TODO

        // get corresponding `TargetFile` according to `self` and `target_manager`
        let target_file = convert_local_file_to_target(self, Rc::clone(&target_manager));

        TargetFile::receive_from_file(&target_file, self)?;

        Ok(())
    }
}
impl SingleFile for LocalFile {
    fn is_exists(&self) -> Result<bool, Error> {
        Ok(self.path.exists())
    }

    fn get_relpath(&self) -> Result<PathBuf, Error> {
        self.path
            .canonicalize()
            .map_err(|e| {
                anyhow!(
                    "Failed to get canonicalized path of {}: {}",
                    self.path.display(),
                    e
                )
            })
            .and_then(|canonical_path| {
                canonical_path
                    .strip_prefix(&self.ltm.borrow().get_base_path())
                    .map(|p| p.to_path_buf())
                    .map_err(|e| anyhow!("Failed to get relative path: {}", e))
            })
    }

    fn get_new_sha256(&self) -> Result<String, anyhow::Error> {
        let sha256_result = {
            if let Ok(true) = self.is_exists() {
                let file = std::fs::File::open(&self.path)?;
                let mut reader = std::io::BufReader::new(file);
                let mut hasher = Sha256::new();
                std::io::copy(&mut reader, &mut hasher)?;

                let result = hasher.finalize();
                Ok(format!("{:x}", result))
            } else if let Ok(false) = self.is_exists() {
                return Err(anyhow!("Local file not found at {}", self.path.display()));
            } else {
                return Err(anyhow!("Unexpected error!"));
            }
        };

        self.sha256
            .replace(Some(sha256_result.as_deref().unwrap().to_string()));
        sha256_result
    }

    fn get_sha256(&self) -> Result<String, Error> {
        let sha256_result = match self.sha256.borrow().as_ref() {
            Some(sha256sum) => Ok(sha256sum.clone()),
            None => self.get_new_sha256(),
        };
        if let Ok(sha256_value) = &sha256_result {
            log::debug!(
                "SHA256 sum of file `{}` is `{}`",
                self.path.display(),
                sha256_value
            );
        }
        sha256_result
    }
}
impl TargetFile for LocalFile {
    type Manager = LocalTargetManager;
    fn from_relpath(relpath: &Path, fm: Rc<RefCell<Self::Manager>>) -> Self {
        Self {
            path: fm.borrow().base_path.join(relpath).canonicalize().unwrap(),
            sha256: RefCell::new(None),
            ltm: fm.clone(),
        }
    }
    fn initialise_path(&self) -> Result<(), Error> {
        std::fs::create_dir_all(self.path.parent().unwrap()).with_context(|| {
            anyhow!(
                "Failed to create folder for {}, most possibly because of permission issues.",
                self.path.display()
            )
        })?;

        Ok(())
    }
    fn receive_from_file(&self, source_file: &LocalFile) -> Result<(), Error> {
        let mut remote_file_io = fs::File::open(&self.path)?;
        let mut local_file_io = fs::File::open(&*source_file.path)?;

        io::copy(&mut local_file_io, &mut remote_file_io)?;
        Ok(())
    }
}
impl PartialEq for LocalFile {
    fn eq(&self, other: &Self) -> bool {
        self.get_sha256().ok().as_deref() == other.get_sha256().ok().as_deref()
    }
}
impl Eq for LocalFile {}
impl PartialEq for Box<dyn SingleFile> {
    fn eq(&self, other: &Self) -> bool {
        self.get_new_sha256().ok().as_deref() == other.get_new_sha256().ok().as_deref()
    }
}
impl Eq for Box<dyn SingleFile> {}
impl Hash for Box<dyn SingleFile> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.get_new_sha256().ok().as_deref().hash(state);
    }
}
// impl Eq for LocalFile {}
impl Hash for LocalFile {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.path.hash(state);
        if let Some(sha256) = self.sha256.borrow().as_ref() {
            sha256.hash(state);
        }
    }
}

/// The _location_ of a file to which it will be uploaded.
///
/// It is binded to a `SshTargetManager`, which is responsible
/// for ssh/sftp connection establishment.
///
/// There can be no file at `SshTargetFile.path`, in which case
/// `SshTargetFile.sha256_cell` is RefCell<None> and
/// `SshTargetFile.is_exists()` returns Ok(false).
#[derive(Clone)]
pub struct SshTargetFile {
    pub path: PathBuf,
    sha256_cell: RefCell<Option<String>>,
    ssh_manager: Rc<RefCell<SshTargetManager>>,
}
impl SingleFile for SshTargetFile {
    fn is_exists(&self) -> Result<bool, Error> {
        let mut ssh_manager = self.ssh_manager.borrow_mut();

        let sftp_conn = ssh_manager.get_sftp()?;

        let exist_or_not = match sftp_conn.stat(&self.path) {
            Ok(_) => true,
            Err(_) => false,
        };
        Ok(exist_or_not)
    }

    fn get_relpath(&self) -> Result<PathBuf, Error> {
        self.path
            .canonicalize()
            .map_err(|e| {
                anyhow!(
                    "Failed to get canonicalized path of {}: {}",
                    self.path.display(),
                    e
                )
            })
            .and_then(|canonical_path| {
                canonical_path
                    .strip_prefix(&self.ssh_manager.borrow().base_path)
                    .map(|p| p.to_path_buf())
                    .map_err(|e| anyhow!("Failed to get relative path: {}", e))
            })
    }

    /// Get value from self.sha256_cell if exists, or get the value
    /// by self.get_new_sha256()
    fn get_sha256(&self) -> Result<String, Error> {
        if let Some(sha256) = self.sha256_cell.borrow().as_ref() {
            Ok(sha256.clone())
        } else {
            let sha256 = self.get_new_sha256()?;
            self.sha256_cell.borrow_mut().replace(sha256.clone());
            Ok(sha256)
        }
    }

    /// Refresh the sha256 value of the file and update the value to
    /// self.sha256_cell
    fn get_new_sha256(&self) -> Result<String, Error> {
        log::debug!("Refreshing sha256 value of {}", self.path.display());

        let mut ssh_manager = self.ssh_manager.borrow_mut();

        let sftp_conn = ssh_manager.get_sftp()?;
        let mut chan = ssh_manager.get_channel()?;
        let sha256 = match sftp_conn.stat(&self.path) {
            Ok(_) => {
                log::trace!("Found {} in remote ssh server", self.path.display());
                let cmd = format!(
                    "sha256sum '{}' | awk '{{print $1}}'",
                    self.path.to_str().unwrap().replace("\\", "/")
                );
                log::trace!("Executing command: '{}'", cmd);
                chan.exec(&cmd).unwrap(); // Work here!

                let mut remote_hash = String::new();
                chan.read_to_string(&mut remote_hash)
                    .expect("Failed to read remote operation result.");
                remote_hash = remote_hash.trim().to_string();

                chan.wait_close().unwrap();

                match chan.exit_status() {
                    Ok(0) => {
                        log::trace!(
                            "Hash of remote file {} is {}",
                            self.path.display(),
                            remote_hash
                        );
                        Ok(remote_hash)
                    }
                    Ok(code) => {
                        log::warn!(
                            "Failed to get hash of remote file: Maybe false syntax? Error code: {}",
                            code
                        );
                        Err(anyhow!(
                            "Executing remote sha256sum returned non-zero exit-code: {code}"
                        ))
                    }
                    Err(err) => {
                        log::warn!(
                            "Failed to get hash of remote file: Maybe connection lost? Error info: {}",
                            err
                        );
                        Err(anyhow!(
                            "Failed to get hash of remote file: Maybe connection lost? Error info: {err}"
                        ))
                    }
                }
            }
            Err(err) => {
                log::warn!(
                    "Failed to get hash of remote file {} because it does not exist: {}",
                    self.path.display(),
                    err
                );
                Err(err.into())
            }
        };

        if let Ok(sha256sum) = sha256.as_deref() {
            log::trace!(
                "Update new sha256 value {} to SshTargetFile.sha256",
                sha256sum
            );
            // Update sha256sum. Note that self.sha256 is type `RefCell<Option<String>>`
            self.sha256_cell.borrow_mut().replace(sha256sum.to_string());
        }

        return sha256;
    }
}
impl TargetFile for SshTargetFile {
    type Manager = SshTargetManager;
    fn from_relpath(relpath: &Path, fm: Rc<RefCell<SshTargetManager>>) -> Self {
        Self {
            path: fm.borrow().base_path.join(relpath).canonicalize().unwrap(),
            sha256_cell: RefCell::new(None),
            ssh_manager: fm.clone(),
        }
    }
    fn initialise_path(&self) -> Result<(), Error> {
        // This method implements an inefficient algorithm to detect and create directories,
        // but its stability has been proven by time.
        let mut ssh_manager = self.ssh_manager.borrow_mut();
        let sftp = ssh_manager.get_sftp()?;
        log::debug!("sftp connection established");

        let remote_dirpath = self.path.parent().unwrap();

        loop {
            if sftp.stat(remote_dirpath).is_ok() {
                log::debug!("Remote folder '{}' exists!", remote_dirpath.display());
                break;
            }
            let mut temp_path = remote_dirpath;
            loop {
                let parent_folder = temp_path.parent().unwrap();
                if sftp.stat(parent_folder).is_ok() {
                    log::trace!(
                        "Remote folder '{}' exists. Now create child folder '{}'.",
                        parent_folder.display(),
                        temp_path.display()
                    );
                    sftp.mkdir(temp_path, 0o755).or_else(|e| {
                        Err(anyhow!(
                            "Failed to create remote folder at '{}': {}",
                            temp_path.display(),
                            e
                        ))
                    })?;
                    log::trace!("Remote folder '{}' created.", temp_path.display());
                } else {
                    log::trace!("Remote folder '{}' does not exist!", temp_path.display());
                    temp_path = parent_folder;
                }
            }
        }

        Ok(())
    }
    fn receive_from_file(&self, source_file: &LocalFile) -> Result<(), Error> {
        // TODO
        // First compare the two file
        let self_box = Box::new(self.clone()) as Box<dyn SingleFile>;
        let source_box = Box::new(source_file.clone()) as Box<dyn SingleFile>;

        if self_box == source_box {
            return Ok(());
        }
        //
        let mut sftp = self.ssh_manager.borrow_mut().get_sftp()?;
        let mut remote_file_io = sftp.create(&*self.path)?;
        Ok(())
    }
}

/// trait of `LocalSourceManager` and all `TargetManager`
pub trait FolderManager {
    fn get_base_path(&self) -> PathBuf;
}

pub trait LocalFolderManager: FolderManager {
    fn is_local(&self) -> bool {
        return true;
    }
}

/// There is no method called `receive_from_file` as it will be a (relatively)
/// very inefficient process. Use `upload_from_folder`, or `SourceManager::upload_to_folder`
/// instead.
///
/// All members of TargetManager should implement `new` method.
pub trait TargetManager: FolderManager {
    type File: TargetFile<Manager = Self>;
    /// Note that another `target_file` parameter should be added to this trait method!
    /// Maybe this method can be remove, and converted to a function.
    fn receive_from_folder(
        &self,
        local_source_folder: LocalSourceManager,
    ) -> Result<(), anyhow::Error>;
    // fn base_path_initialise(&self) -> Result<(), anyhow::Error>;
}

// pub enum AnyTargetManager<'a> {
//     Local(LocalTargetManager),
//     Ssh(SshTargetManager<'a>),
// }

// impl<'a> TargetManager for AnyTargetManager<'a> {
//     fn upload_file<T: FolderManager>(&self, local_file: LocalFile<T>) -> Result<(), anyhow::Error> {
//         match self {
//             AnyTargetManager::Local(m) => m.upload_file(local_file),
//             AnyTargetManager::Ssh(m) => m.upload_file(local_file),
//         }
//     }
// }

/// Manager for a local folder from which files are scanned and selected.
///
/// self.base_path is the root folder from which local files are scanned and
/// uploaded.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
pub struct LocalSourceManager {
    pub base_path: PathBuf,
}
impl LocalSourceManager {
    pub fn get_files(
        &self,
        exts: &[&str],
        regex: Option<&str>,
        min_depth: Option<usize>,
        max_depth: Option<usize>,
    ) -> Result<HashSet<LocalFile>, Error> {
        // 创建一个共享的 Rc<RefCell<LocalSourceManager>> 实例
        let shared_ltm = Rc::new(RefCell::new(self.clone()));

        let mut files = HashSet::new();

        let is_hidden = |entry: &walkdir::DirEntry| {
            entry
                .file_name()
                .to_str()
                .map(|s| s.starts_with(".") || s.starts_with("_") || s.starts_with("~$"))
                .unwrap_or(false)
        };

        let walker = {
            let mut temp_walkdir = WalkDir::new(&self.base_path);
            if let Some(min_depth) = min_depth {
                temp_walkdir = temp_walkdir.min_depth(min_depth);
            }
            if let Some(max_depth) = max_depth {
                temp_walkdir = temp_walkdir.max_depth(max_depth);
            }
            temp_walkdir.into_iter()
        };

        for entry in walker
            .filter_entry(|entry| !is_hidden(entry))
            .filter_map(|entry| entry.ok())
        {
            let file_path_buf = entry.into_path();
            log::trace!("Got file {}", file_path_buf.display());

            // 检查文件扩展名是否匹配
            if let Some(ext) = file_path_buf.extension() {
                let ext_string = ext.to_str().unwrap().to_lowercase();
                if exts.contains(&ext_string.as_str()) {
                    log::trace!(
                        "Extension of {} is {}, which hits the target.",
                        file_path_buf.display(),
                        &ext_string
                    );
                    let local_file = LocalFile {
                        path: file_path_buf.clone(),
                        sha256: RefCell::new(None),
                        ltm: shared_ltm.clone(), // 使用共享的实例
                    };
                    files.insert(local_file);
                    continue;
                } else {
                    log::trace!(
                        "Extension of {} is {}, which fails to hit the target.",
                        file_path_buf.display(),
                        &ext_string
                    );
                }
            }

            // 检查文件名是否匹配正则表达式
            if let Some(pattern) = regex {
                log::trace!("Checking {} against {}", file_path_buf.display(), pattern);
                let re = match Regex::new(pattern) {
                    Ok(re) => re,
                    Err(_) => {
                        log::error!("Invalid regex pattern: {}", pattern);
                        continue;
                    }
                };
                if re.is_match(file_path_buf.file_name().unwrap().to_str().unwrap()) {
                    log::trace!("{} satisfies the regex pattern", file_path_buf.display());
                    let local_file = LocalFile {
                        path: file_path_buf.clone(),
                        sha256: RefCell::new(None),
                        ltm: shared_ltm.clone(), // 使用共享的实例
                    };
                    files.insert(local_file);
                    continue;
                } else {
                    log::trace!(
                        "{} failed to satisfy the regex pattern",
                        file_path_buf.display()
                    );
                }
            }
        }
        Ok(files)
    }
    pub fn upload_to_folder<T: TargetManager>(&self, target_manager: Rc<RefCell<T>>) -> Result<()> {
        // TODO: implementation
        Ok(())
    }
}
impl FolderManager for LocalSourceManager {
    fn get_base_path(&self) -> PathBuf {
        self.base_path.canonicalize().unwrap()
    }
}
impl LocalFolderManager for LocalSourceManager {}

/// manager for target folder which exists locally
///
/// `self.base_path` is the root folder based on which files will
/// be uploaded according to the relative path.
#[derive(PartialEq, Eq)]
pub struct LocalTargetManager {
    pub base_path: PathBuf,
}
impl LocalTargetManager {
    pub fn new(base_path: Option<&str>) -> Self {
        LocalTargetManager {
            base_path: match base_path {
                Some(path) => Path::new(path).to_path_buf(),
                None => get_default_folder_name(),
            },
        }
    }
}
impl FolderManager for LocalTargetManager {
    fn get_base_path(&self) -> PathBuf {
        self.base_path.canonicalize().unwrap()
    }
}
impl LocalFolderManager for LocalTargetManager {}
impl TargetManager for LocalTargetManager {
    type File = LocalFile;
    fn receive_from_folder(
        &self,
        local_source_folder: LocalSourceManager,
    ) -> Result<(), anyhow::Error> {
        // let remote_path = {
        //     let source_path = local_source_file.base_path.clone();
        //     let source_root_path = local_source_file.ltm.deref().borrow().get_base_path();
        //     let relpath = source_path.strip_prefix(source_root_path)?;
        //     let remote_path = self.get_base_path().join(relpath);
        //     remote_path
        // };
        // // TODO: implement this method
        // // get relavent path
        // let fm = local_source_file.ltm.deref();
        // let remote_file = LocalFile {
        //     path: remote_path,
        //     sha256: RefCell::new(None),
        //     ltm: Box::new(LocalTargetManager {
        //         base_path: self.get_base_path(),
        //     }),
        // };
        // if local_source_file != remote_file {
        //     // TODO: Before copying files, we should
        //     // make sure that target path exists!
        //     log::debug!(
        //         "Copying local file {} to local {}",
        //         local_source_file.path.display(),
        //         remote_file.path.display()
        //     );
        // }
        Ok(())
    }
}

pub trait SshRemoteAuthentication {
    fn authenticate(&self) -> Result<ssh2::Session, Error>;
}

pub struct SshPasswordAuthentication {
    pub ip: String,
    pub port: i64,
    pub username: String,
    pub password: String,
}
impl SshRemoteAuthentication for SshPasswordAuthentication {
    fn authenticate(&self) -> Result<ssh2::Session, Error> {
        log::info!("Connecting to SSH server with password...");
        let tcp = std::net::TcpStream::connect(format!("{}:{}", self.ip, self.port))?;
        let mut session = ssh2::Session::new()?;
        session.set_tcp_stream(tcp);
        session.handshake()?;
        session.userauth_password(&self.username, &self.password)?;
        return Ok(session);
    }
}

pub struct SshKeyAuthentication {
    pub ip: String,
    pub port: i64,
    pub username: String,
    pub pem_key: Option<String>,
}
impl SshRemoteAuthentication for SshKeyAuthentication {
    fn authenticate(&self) -> Result<ssh2::Session, Error> {
        // "SSH Key authentication is not supported yet!"
        Err(anyhow!("SSH Key authentication is not supported yet!"))
    }
}

/// manager for a folder which is accessible via SSH and to which
/// local files will be uploaded to
pub struct SshTargetManager {
    pub base_path: PathBuf,
    pub login_params: Box<dyn SshRemoteAuthentication>,
    pub session: ssh2::Session,
}
impl SshTargetManager {
    pub fn new<T: SshRemoteAuthentication + 'static>(
        base_path: Option<&str>,
        login_params: T,
    ) -> Self {
        let connection = login_params.authenticate().unwrap();
        Self {
            base_path: match base_path {
                Some(path) => PathBuf::from(path),
                None => get_default_folder_name(),
            },
            login_params: Box::new(login_params) as Box<dyn SshRemoteAuthentication + 'static>,
            session: connection,
        }
    }
    pub fn reconnect(&mut self) -> Result<(), Error> {
        self.session = self.login_params.authenticate()?;
        Ok(())
        // Further complete the implementation
    }

    /// Get SFTP connection.
    /// If you get an error, you needn't retry. The function automatically
    /// reconnect and retry 10 times.
    pub fn get_sftp(&mut self) -> Result<ssh2::Sftp, Error> {
        for i in 0..10 {
            if let Ok(sftp) = self.session.sftp() {
                return Ok(sftp);
            } else {
                log::warn!(
                    "Failed to establish sftp connection. Retry after {}s {}/10",
                    i + 1,
                    10 - i
                );
                std::thread::sleep(std::time::Duration::from_millis(1000 * (i + 1)));
                self.reconnect()?;
            }
        }
        log::error!("Failed to establish sftp connection");
        return Err(anyhow!("Failed to establish sftp connection"));
    }
    pub fn get_channel(&mut self) -> Result<ssh2::Channel, Error> {
        for i in 0..10 {
            if let Ok(channel) = self.session.channel_session() {
                return Ok(channel);
            } else {
                log::warn!(
                    "Failed to establish channel connection. Retry after 1s {}/10",
                    10 - i
                );
                std::thread::sleep(std::time::Duration::from_millis(1000));
                self.reconnect()?;
            }
        }
        log::error!("Failed to establish channel connection");
        return Err(anyhow!("Failed to establish channel connection"));
    }
}
impl FolderManager for SshTargetManager {
    fn get_base_path(&self) -> PathBuf {
        self.base_path.canonicalize().unwrap()
    }
}
impl TargetManager for SshTargetManager {
    type File = SshTargetFile;
    fn receive_from_folder(
        &self,
        local_source_folder: LocalSourceManager,
    ) -> Result<(), anyhow::Error> {
        // TODO
        Ok(())
    }
}
