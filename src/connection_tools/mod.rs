use ssh2::Session;

struct SshSessionGuard<'a> {
    session: &'a mut Session,
}
impl Drop for SshSessionGuard<'_> {
    fn drop(&mut self) {
        self.session
            .disconnect(None, "Out of scope, close session.", None)
            .expect("Failed to disconnect");
    }
}