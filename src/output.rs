use std::io::{self, Write};

use thiserror::Error;

use crate::model::{Snapshot, WindowInfo};

#[derive(Debug, Error)]
pub enum Error {
    #[error("could not serialize JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("could not write stdout: {0}")]
    Io(#[from] io::Error),
}

pub fn print(snapshot: &Snapshot, json: bool, verbosity: u8) -> Result<(), Error> {
    if json {
        let text = serde_json::to_string_pretty(snapshot)?;
        println!("{text}");
    } else {
        let mut stdout = io::BufWriter::new(io::stdout().lock());
        write_human(&mut stdout, snapshot, verbosity)?;
        stdout.flush()?;
    }
    Ok(())
}

fn write_human(output: &mut impl Write, snapshot: &Snapshot, verbosity: u8) -> Result<(), Error> {
    if let Some(app) = &snapshot.frontmost_app {
        writeln!(
            output,
            "frontmost: {}{} (pid {})",
            app.name.as_deref().unwrap_or("<unknown>"),
            app.bundle_id
                .as_deref()
                .map(|id| format!(" [{id}]"))
                .unwrap_or_default(),
            app.pid
        )?;
    } else {
        writeln!(output, "frontmost: <unknown>")?;
    }
    writeln!(
        output,
        "focused window: {}",
        snapshot
            .focused_window_id
            .map_or_else(|| "<none>".to_owned(), |id| id.to_string())
    )?;

    if let Some(display) = &snapshot.display {
        writeln!(
            output,
            "display: {}{} {}x{} px bounds={}",
            display.id,
            if display.main { " (main)" } else { "" },
            display.pixel_width,
            display.pixel_height,
            format_rect(display.bounds)
        )?;
    }
    if let Some(displays) = &snapshot.displays {
        writeln!(output, "displays: {}", displays.len())?;
        for display in displays {
            writeln!(
                output,
                "  - {}{} {}x{} px bounds={}",
                display.id,
                if display.main { " (main)" } else { "" },
                display.pixel_width,
                display.pixel_height,
                format_rect(display.bounds)
            )?;
        }
    }

    writeln!(output, "windows: {}", snapshot.windows.len())?;
    for window in &snapshot.windows {
        write_window(output, window, verbosity)?;
    }
    if let Some(path) = &snapshot.screenshot_path {
        writeln!(output, "screenshot: {path}")?;
    }
    Ok(())
}

fn write_window(output: &mut impl Write, window: &WindowInfo, verbosity: u8) -> Result<(), Error> {
    let marker = if window.is_focused {
        "*"
    } else if window.is_frontmost_app {
        ">"
    } else {
        " "
    };
    let title = window.title.as_deref().unwrap_or("<untitled>");
    let display = window
        .display_id
        .map_or_else(String::new, |id| format!(" display={id}"));
    writeln!(
        output,
        "  {marker} z={} id={} pid={} app={} title={title:?} bounds={}{}",
        window.z_order,
        window.window_id,
        window.owner.pid,
        window.owner.name.as_deref().unwrap_or("<unknown>"),
        format_rect(window.bounds),
        display
    )?;

    if verbosity >= 1 {
        writeln!(
            output,
            "    layer={} alpha={:.2} onscreen={} minimized={} main={} role={} subrole={}",
            window.layer,
            window.alpha,
            window.is_onscreen,
            optional_bool(window.is_minimized),
            optional_bool(window.is_main),
            window.role.as_deref().unwrap_or("<none>"),
            window.subrole.as_deref().unwrap_or("<none>")
        )?;
    }
    if verbosity >= 2 {
        if let Some(errors) = &window.ax_errors {
            writeln!(output, "    AX errors: {errors:?}")?;
        }
        if let Some(raw) = &window.raw {
            writeln!(output, "    raw: {raw:?}")?;
        }
    }
    Ok(())
}

fn format_rect(rect: crate::model::Rect) -> String {
    format!(
        "({:.0},{:.0} {:.0}x{:.0})",
        rect.x, rect.y, rect.width, rect.height
    )
}

fn optional_bool(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "true",
        Some(false) => "false",
        None => "unknown",
    }
}
