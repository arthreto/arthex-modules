//! The system side of Remote Connections: saved SFTP, FTP and FTPS servers, each mounted as a
//! drive of Explorer while connected.

#[cfg(windows)]
mod desktop;
#[cfg_attr(not(windows), allow(dead_code))]
mod model;
#[cfg_attr(not(windows), allow(dead_code))]
mod paths;

#[cfg(windows)]
fn main() -> std::io::Result<()> {
    desktop::run()
}

#[cfg(not(windows))]
fn main() -> std::io::Result<()> {
    arthex_module::Module::new().run()
}
