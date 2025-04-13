use anyhow::{Error, Result, anyhow};
use chrono::Local;
use log;
use regex::Regex;
use sha2::{Digest, Sha256};
use std::{
    cell::RefCell,
    collections::HashSet,
    hash::Hash,
    io::Read,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

fn get_default_folder_name() -> PathBuf {
    let formatted_date = Local::now().format("%Y-%m-%d").to_string();
    let computer_identifier = match gethostname::gethostname().to_str() {
        Some(name) => {
            let home_dir = dirs::home_dir().unwrap();
            format!(
                "{} -- {}",
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

/// Generate a `TargetFile` from a `LocalFile`.
pub fn convert_local_file_to_target<T, U>(source: LocalFile, tm: T) -> U
where
    // F: FolderManager,
    T: TargetManager,
    U: TargetFile<T = T>,
{
    // TODO: the `root_path` is problematic!
    let root_path = PathBuf::from(&source.ltm.get_base_path())
        .canonicalize()
        .unwrap();
    let abspath = &source.path.canonicalize().unwrap();
    let relpath = abspath.strip_prefix(&root_path).unwrap();
    TargetFile::from_relpath(&root_path, relpath, tm)
}

pub trait SingleFile {
    /// Return error if connection failed to establish
    fn is_exists(&self) -> Result<bool, Error>;
    fn get_relpath(&self) -> Result<PathBuf, Error>;
    fn get_sha256(&self) -> Result<String, Error>;
    fn get_new_sha256(&self) -> Result<String, Error>;
}

pub trait TargetFile: SingleFile {
    type T: TargetManager;
    fn from_relpath(root_path: &Path, relpath: &Path, fm: Self::T) -> Self;
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
// #[derive(Eq)]
pub struct LocalFile {
    pub path: PathBuf,
    sha256: RefCell<Option<String>>,
    ltm: Box<dyn LocalFolderManager>,
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
                    .strip_prefix(&self.ltm.get_base_path())
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
    type T = LocalTargetManager;
    fn from_relpath(root_path: &Path, relpath: &Path, fm: Self::T) -> Self {
        Self {
            path: root_path.join(relpath),
            sha256: RefCell::new(None),
            ltm: Box::new(fm),
        }
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
        self.get_sha256().ok().as_deref() == other.get_sha256().ok().as_deref()
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
pub struct SshTargetFile<'a> {
    pub path: PathBuf,
    sha256_cell: RefCell<Option<String>>,
    ssh_manager: RefCell<SshTargetManager<'a>>,
}
impl SingleFile for SshTargetFile<'_> {
    fn is_exists(&self) -> Result<bool, Error> {
        let mut ssh_manager = self.ssh_manager.borrow_mut();

        let sftp_conn = ssh_manager.get_sftp()?;
        // let mut chan = match ssh_manager.get_channel() {
        //     Ok(chan) => chan,
        //     Err(err) => {
        //         log::error!("Failed to get channel: {}", err);
        //         return false;
        //     }
        // };
        let sha256 = match sftp_conn.stat(&self.path) {
            Ok(_) => true,
            Err(_) => false,
        };
        Ok(sha256)
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
impl<'a> TargetFile for SshTargetFile<'a> {
    type T = SshTargetManager<'a>;
    fn from_relpath(root_path: &Path, relpath: &Path, fm: SshTargetManager<'a>) -> Self {
        Self {
            path: root_path.join(relpath),
            sha256_cell: RefCell::new(None),
            ssh_manager: RefCell::new(fm),
        }
    }
}

/// trait of `LocalSourceManager` and all `TargetManager`
pub trait FolderManager {
    fn get_base_path(&self) -> &str;
}

pub trait LocalFolderManager: FolderManager {
    fn is_local(&self) -> bool {
        return true;
    }
}

pub trait TargetManager: FolderManager {
    fn upload_file(&self, local_file: LocalFile) -> Result<(), anyhow::Error>;
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

/// manager for a local folder from which files are scanned and selected
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
        // This function needs further implementation.
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
            };
            if let Some(max_depth) = max_depth {
                temp_walkdir = temp_walkdir.max_depth(max_depth);
            };
            temp_walkdir.into_iter()
        };

        for entry in walker
            .filter_entry(|entry| {
                if !is_hidden(entry) || entry.file_type().is_file() {
                    log::trace!("{} is a hidden file", entry.clone().into_path().display());
                    false
                } else {
                    true
                }
            })
            .filter_map(|entry| entry.ok())
        {
            let file_path_buf = entry.into_path();
            log::trace!("Got file {}", file_path_buf.display());

            //  First check if the extension hits the target
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
                        ltm: Box::new(self.clone()),
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
            // Then check if the file name meets the regex
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
                    files.insert(LocalFile {
                        path: file_path_buf.clone(),
                        sha256: RefCell::new(None),
                        ltm: Box::new(self.clone()),
                    });
                    continue;
                } else {
                    log::trace!(
                        "{} failed to satisfy the regex pattern",
                        file_path_buf.display()
                    )
                }
            }
        }

        Ok(files)
    }
}
impl FolderManager for LocalSourceManager {
    fn get_base_path(&self) -> &str {
        self.base_path.to_str().unwrap()
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
    fn get_base_path(&self) -> &str {
        self.base_path.to_str().unwrap()
    }
}
impl LocalFolderManager for LocalTargetManager {}
impl TargetManager for LocalTargetManager {
    fn upload_file(&self, local_file: LocalFile) -> Result<(), anyhow::Error> {
        let _source_path = local_file.path;
        // get relavent path
        Ok(())
    }
}

pub trait SshRemoteAuthentication {
    fn authenticate(&self) -> Result<ssh2::Session, Error>;
}

pub struct SshPasswordAuthentication<'a> {
    pub ip: &'a str,
    pub port: i64,
    pub username: &'a str,
    pub password: &'a str,
}
impl<'a> SshRemoteAuthentication for SshPasswordAuthentication<'a> {
    fn authenticate(&self) -> Result<ssh2::Session, Error> {
        log::info!("Connecting to SSH server with password...");
        let tcp = std::net::TcpStream::connect(format!("{}:{}", self.ip, self.port))?;
        let mut session = ssh2::Session::new()?;
        session.set_tcp_stream(tcp);
        session.handshake()?;
        session.userauth_password(self.username, self.password)?;
        return Ok(session);
    }
}

pub struct SshKeyAuthentication<'a> {
    pub ip: &'a str,
    pub port: i64,
    pub username: &'a str,
    pub pem_key: Option<&'a str>,
}
impl<'a> SshRemoteAuthentication for SshKeyAuthentication<'a> {
    fn authenticate(&self) -> Result<ssh2::Session, Error> {
        // "SSH Key authentication is not supported yet!"
        Err(anyhow!("SSH Key authentication is not supported yet!"))
    }
}

/// manager for a folder which is accessible via SSH and to which
/// local files will be uploaded to
pub struct SshTargetManager<'a> {
    pub base_path: PathBuf,
    pub login_params: Box<dyn SshRemoteAuthentication + 'a>,
    pub session: ssh2::Session,
}
impl<'a> SshTargetManager<'a> {
    pub fn new<T: SshRemoteAuthentication + 'a>(
        base_path: Option<&'a str>,
        login_params: T,
    ) -> Self {
        let connection = login_params.authenticate().unwrap();
        Self {
            base_path: match base_path {
                Some(path) => PathBuf::from(path),
                None => get_default_folder_name(),
            },
            login_params: Box::new(login_params) as Box<dyn SshRemoteAuthentication + 'a>,
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
impl FolderManager for SshTargetManager<'_> {
    fn get_base_path(&self) -> &str {
        self.base_path.to_str().unwrap()
    }
}
impl TargetManager for SshTargetManager<'_> {
    fn upload_file(&self, local_file: LocalFile) -> Result<(), anyhow::Error> {
        // TODO
        Ok(())
    }
}
