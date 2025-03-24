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
            base_path: base_path.unwrap_or(""), // Modify default base path later.
        }
    }
}
impl<'a> TargetManager for LocalTargetManager<'a> {
    fn get_base_path(&self) -> &str {
        self.base_path
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

pub struct SshTargetManager<'a, 'b> {
    pub base_path: &'a Path,
    pub login_params: Box<dyn SshRemoteAuthentication + 'b>,
    pub connection: ssh2::Session,
}
impl<'a, 'b> SshTargetManager<'a, 'b> {
    pub fn new<T: SshRemoteAuthentication + 'b>(
        base_path: Option<&'a str>,
        login_params: T,
    ) -> Self {
        let connection = login_params.authenticate().unwrap();
        Self {
            base_path: Path::new(base_path.unwrap_or("default")), // Modify default path here later.
            login_params: Box::new(login_params) as Box<dyn SshRemoteAuthentication + 'b>,
            connection,
        }
    }
}
impl<'a, 'b> TargetManager for SshTargetManager<'a, 'b> {
    fn get_base_path(&self) -> &str {
        self.base_path.to_str().unwrap()
    }
}
