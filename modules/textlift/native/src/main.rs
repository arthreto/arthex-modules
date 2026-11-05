//! The system side of `TextLift`: reads the text of an image with the recognition built into Windows.
//!
//! An image stays in memory while the tool shows it and is never saved. Nothing leaves the computer.

#[cfg_attr(not(windows), allow(dead_code))]
mod bitmap;
#[cfg(windows)]
mod desktop;
#[cfg_attr(not(windows), allow(dead_code))]
mod paint;

#[cfg(windows)]
fn main() -> std::io::Result<()> {
    desktop::run()
}

#[cfg(not(windows))]
fn main() -> std::io::Result<()> {
    arthex_module::Module::new().run()
}
