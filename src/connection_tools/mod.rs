use std::sync::{Arc, Mutex};

use ssh2::Session;

pub struct SshSessionGuard<'a> {
    pub session: &'a Arc<Mutex<Session>>,
}
impl Drop for SshSessionGuard<'_> {
    fn drop(&mut self) {
        let sess = self.session.lock().unwrap();
        match sess
            .disconnect(None, "Out of scope, close session.", None) {
                Ok(_) => log::info!("Session closed successfully."),
                Err(e) => log::error!("Error closing session: {}", e)
        }
    }
}