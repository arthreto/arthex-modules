//! The system side of the Color module: takes a colour from the screen.
//!
//! The screen is copied once, before anything of ours is on it, so the colour read is the real one,
//! without the fade that shows the picker is active. The copy stays in memory for as long as the
//! picker is open, and is never saved. Arthex hides the overlay around the call.

#[cfg_attr(not(windows), allow(dead_code))]
mod paint;
#[cfg(windows)]
mod picker;

use arthex_module::Module;
use serde_json::Value;

/// The colour the user clicked, as `#RRGGBB`, or nothing when they cancelled.
#[cfg(windows)]
#[allow(clippy::unnecessary_wraps)] // Other systems fail here, and the handler has one signature.
fn pick() -> Result<Value, String> {
    Ok(serde_json::json!(picker::run()))
}

#[cfg(not(windows))]
fn pick() -> Result<Value, String> {
    Err("taking a colour from the screen needs Windows".to_string())
}

fn main() -> std::io::Result<()> {
    Module::new().on("pick", |_, _| pick()).run()
}
