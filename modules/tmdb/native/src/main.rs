//! The system side of TMDB: asks TMDB for films, series and people with the user's own key, which
//! stays encrypted for their Windows account. Only TMDB's API and image servers are reached.

#[cfg(windows)]
mod desktop;
#[cfg_attr(not(windows), allow(dead_code))]
mod request;

#[cfg(windows)]
fn main() -> std::io::Result<()> {
    desktop::run()
}

#[cfg(not(windows))]
fn main() -> std::io::Result<()> {
    arthex_module::Module::new().run()
}
