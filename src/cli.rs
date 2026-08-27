use std::path::PathBuf;

use usage::{Args as UsageArgs, Cli as UsageCli};

/// Dump visible macOS windows, their geometry, and display information.
#[derive(UsageCli)]
#[usage(bin = "screen-dump", version = "0.2.0", completion)]
pub(crate) struct Cli {
    #[usage(flatten)]
    pub(crate) args: CliArgs,
}

#[derive(UsageArgs)]
pub(crate) struct CliArgs {
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
    #[usage(
        long,
        value_name = "NAME_OR_BUNDLE_ID",
        complete = crate::completion::app_candidates
    )]
    pub(crate) app: Option<String>,

    /// Filter by owning process ID.
    #[usage(
        long,
        value_name = "PID",
        complete = crate::completion::pid_candidates
    )]
    pub(crate) pid: Option<i32>,

    /// Filter by Core Graphics window ID.
    #[usage(
        long,
        value_name = "ID",
        complete = crate::completion::window_candidates
    )]
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
