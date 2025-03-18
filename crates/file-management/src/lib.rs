use log;
use std::{error::Error, path::Path};

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

pub trait TargetFile {
    fn is_exists(&self) -> bool;
    fn get_sha256(&self) -> Result<Vec<u8>, Box<dyn Error>>;
    fn upload(&self, source: &str) -> Result<(), Box<dyn Error>>;
}

pub struct LocalTargetFile<'a> {
    pub path: &'a Path,
}

impl TargetFile for LocalTargetFile<'_> {
    fn is_exists(&self) -> bool {
        self.path.exists()
    }

    fn get_sha256(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        Ok(vec![])
    }

    fn upload(&self, source: &str) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

pub trait TargetManager {
    fn get_base_path(&self) -> &str;
}

pub struct LocalTargetManager<'a> {
    pub base_path: &'a str,
}

impl<'a> LocalTargetManager<'a> {
    pub fn new(base_path: Option<&'a str>) -> Self {
        LocalTargetManager {
            base_path: base_path.unwrap_or(""),
        }
    }
}

impl<'a> TargetManager for LocalTargetManager<'a> {
    fn get_base_path(&self) -> &str {
        self.base_path
    }
}

pub trait RemoteAuthentication {
    fn authenticate(&self) -> Result<(), Box<dyn Error>>;
}

pub struct SshPasswordAuthentication<'a> {
    pub ip: &'a str,
    pub port: i64,
    pub username: &'a str,
    pub password: &'a str,
}
impl<'a> RemoteAuthentication for SshPasswordAuthentication<'a> {
    fn authenticate(&self) -> Result<(), Box<dyn Error>> {
        log::info!("Connecting to SSH server with password...");
        let tcp = std::net::TcpStream::connect(format!("{}:{}", self.ip, self.port))?;
        let mut session = ssh2::Session::new()?;
        session.set_tcp_stream(tcp);
        session.handshake()?;
        session.userauth_password(self.username, self.password)?;
        Ok(())
    }
}

pub struct SshKeyAuthentication<'a> {
    pub ip: &'a str,
    pub port: i64,
    pub username: &'a str,
    pub pem_key: Option<&'a str>,
}
impl<'a> RemoteAuthentication for SshKeyAuthentication<'a> {
    fn authenticate(&self) -> Result<(), Box<dyn Error>> {
        // "SSH Key authentication is not supported yet!"
        Err("SSH Key authentication is not supported yet!".into())
    }
}

pub struct SshTargetManager<'a, T: RemoteAuthentication> {
    pub base_path: &'a Path,
    pub login_params: T,
    pub connection: ssh2::Session,
}

impl<'a, T: RemoteAuthentication> SshTargetManager<'a, T> {
    pub fn new(base_path: Option<&'a str>, login_params: T, connection: ssh2::Session) -> Self {
        login_params.authenticate().unwrap();
        Self {
            base_path: Path::new(base_path.unwrap_or("default")),
            login_params,
            connection,
        }
    }
}
impl<'a, T: RemoteAuthentication> TargetManager for SshTargetManager<'a, T> {
    fn get_base_path(&self) -> &str {
        return self.base_path.to_str().unwrap();
    }
}
