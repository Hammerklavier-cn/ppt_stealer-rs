use chrono::Local;
use log;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    error::Error,
    path::{Path, PathBuf},
};

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}

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
    return Path::new(&formatted_date).join(Path::new(&computer_identifier));
}

pub trait TargetFile {
    fn is_exists(&self) -> bool;
    fn get_sha256(&self) -> Result<Vec<u8>, Box<dyn Error>>;
}

pub struct LocalTargetFile {
    pub path: PathBuf,
    sha256: Option<String>,
}
impl TargetFile for LocalTargetFile {
    fn is_exists(&self) -> bool {
        self.path.exists()
    }

    fn get_sha256(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        match self.sha256.as_ref() {
            Some(md5) => Ok(md5.clone().into_bytes()),
            None => {
                if self.is_exists() {
                    let file = std::fs::File::open(&self.path)?;
                    let mut reader = std::io::BufReader::new(file);
                    let mut hasher = Sha256::new();
                    std::io::copy(&mut reader, &mut hasher)?;

                    let result = hasher.finalize();
                    Ok(result.to_vec())
                } else {
                    return Err(format!("Local file not found at {}", self.path.display()).into());
                }
            }
        }
    }
}
impl<T: TargetFile> PartialEq<T> for LocalTargetFile {
    fn eq(&self, other: &T) -> bool {
        // 安全处理 Result，避免 unwrap() 导致 panic
        self.get_sha256().ok().as_deref() == other.get_sha256().ok().as_deref()
    }
}

struct SshTargetFile {
    pub path: PathBuf,
    sha256: Option<String>,
}
impl TargetFile for SshTargetFile {
    fn is_exists(&self) -> bool {
        false
    }

    fn get_sha256(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        Err("Not supported yet".into())
    }
}

pub trait TargetManager {
    fn get_base_path(&self) -> &str;
}

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
    pub fn get_files(
        &self,
        exts: &str,
        regex: &str,
    ) -> Result<BTreeSet<LocalTargetFile>, Box<dyn Error>> {
        Ok(BTreeSet::new())
    }
}
impl TargetManager for LocalTargetManager {
    fn get_base_path(&self) -> &str {
        self.base_path.to_str().unwrap()
    }
}

pub trait SshRemoteAuthentication {
    fn authenticate(&self) -> Result<ssh2::Session, Box<dyn Error>>;
}

pub struct SshPasswordAuthentication<'a> {
    pub ip: &'a str,
    pub port: i64,
    pub username: &'a str,
    pub password: &'a str,
}
impl<'a> SshRemoteAuthentication for SshPasswordAuthentication<'a> {
    fn authenticate(&self) -> Result<ssh2::Session, Box<dyn Error>> {
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
    fn authenticate(&self) -> Result<ssh2::Session, Box<dyn Error>> {
        // "SSH Key authentication is not supported yet!"
        Err("SSH Key authentication is not supported yet!".into())
    }
}

pub struct SshTargetManager<'a> {
    pub base_path: PathBuf,
    pub login_params: Box<dyn SshRemoteAuthentication + 'a>,
    pub connection: ssh2::Session,
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
            connection,
        }
    }
}
impl TargetManager for SshTargetManager<'_> {
    fn get_base_path(&self) -> &str {
        self.base_path.to_str().unwrap()
    }
}
