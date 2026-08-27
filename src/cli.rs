use std::path::PathBuf;

use usage::Cli as UsageCli;

/// Dump visible macOS windows, their geometry, and display information.
#[derive(UsageCli)]
#[usage(bin = "screen-dump", version = "0.1.0")]
pub(crate) struct Cli {
    /// Emit JSON instead of the human-readable report.
    #[usage(long)]
    pub(crate) json: bool,

    /// Capture the main display as a PNG at PATH.
    #[usage(long, value_name = "PATH")]
    pub(crate) screenshot: Option<PathBuf>,

    /// Include every Core Graphics window returned by macOS.
    #[usage(long)]
    pub(crate) all: bool,

    /// Filter by application name or bundle identifier.
    #[usage(long, value_name = "NAME_OR_BUNDLE_ID")]
    pub(crate) app: Option<String>,

    /// Filter by owning process ID.
    #[usage(long, value_name = "PID")]
    pub(crate) pid: Option<i32>,

    /// Filter by Core Graphics window ID.
    #[usage(long, value_name = "ID")]
    pub(crate) window_id: Option<u32>,

    /// Include hidden or minimized windows.
    #[usage(long)]
    pub(crate) include_hidden: bool,

    /// Include desktop shell and system utility windows.
    #[usage(long)]
    pub(crate) include_system: bool,

    /// Increase diagnostic detail; repeat for raw values and AX errors.
    #[usage(short = 'v', long, count)]
    pub(crate) verbosity: u8,
}
