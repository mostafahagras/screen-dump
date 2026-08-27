#![cfg_attr(not(target_os = "macos"), allow(dead_code))]

#[cfg(not(target_os = "macos"))]
compile_error!("screen-dump currently supports macOS only");

#[cfg(target_os = "macos")]
mod ax;
#[cfg(target_os = "macos")]
mod cli;
#[cfg(target_os = "macos")]
mod collector;
#[cfg(target_os = "macos")]
mod model;
#[cfg(target_os = "macos")]
mod output;
#[cfg(target_os = "macos")]
mod screenshot;

#[cfg(target_os = "macos")]
fn main() {
    let cli = cli::Cli::parse();
    if let Err(error) = collector::run(&cli) {
        eprintln!("screen-dump: {error}");
        std::process::exit(error.exit_code());
    }
}
