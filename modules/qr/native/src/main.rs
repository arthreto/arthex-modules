//! The system side of QR Code: traces a zone of the screen, saves a code, joins a Wi-Fi network and
//! opens what a code holds with the programs of the computer.
//!
//! A traced zone stays in memory and only goes to the interface. Nothing leaves the computer.

mod address;
#[cfg_attr(not(windows), allow(dead_code))]
mod bitmap;
#[cfg(windows)]
mod desktop;
#[cfg_attr(not(windows), allow(dead_code))]
mod paint;
#[cfg_attr(not(windows), allow(dead_code))]
mod wifi_profile;

#[cfg(windows)]
fn main() -> std::io::Result<()> {
    desktop::run()
}

#[cfg(not(windows))]
fn main() -> std::io::Result<()> {
    arthex_module::Module::new().run()
}
