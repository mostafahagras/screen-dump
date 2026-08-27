use std::collections::BTreeMap;

use objc2_app_kit::{NSRunningApplication, NSWorkspace};
use objc2_core_foundation::{CFArray, CFDictionary, CFNumber, CFRetained, CFString, CFType};
use objc2_core_graphics::{
    CGDisplayBounds, CGDisplayPixelsHigh, CGDisplayPixelsWide, CGGetActiveDisplayList,
    CGMainDisplayID, CGWindowListCopyWindowInfo, CGWindowListOption, kCGWindowAlpha,
    kCGWindowBounds, kCGWindowIsOnscreen, kCGWindowLayer, kCGWindowName, kCGWindowNumber,
    kCGWindowOwnerName, kCGWindowOwnerPID,
};
use thiserror::Error;

use crate::ax::{self, AxElement, AxErrorKind, attributes};
use crate::cli::Cli;
use crate::model::{ApplicationInfo, DisplayInfo, Rect, Snapshot, WindowInfo};
use crate::{output, screenshot};

#[derive(Debug, Error)]
pub enum Error {
    #[error(
        "Accessibility permission is required. Enable screen-dump in System Settings > Privacy & Security > Accessibility and retry."
    )]
    AccessibilityPermission,
    #[error("Core Graphics failed while enumerating displays: {0:?}")]
    Display(objc2_core_graphics::CGError),
    #[error("Core Graphics did not return a window list")]
    WindowListUnavailable,
    #[error("failed to write screenshot: {0}")]
    Screenshot(#[from] screenshot::Error),
    #[error("failed to render output: {0}")]
    Output(#[from] output::Error),
    #[error("invalid runtime state: {0}")]
    Runtime(String),
}

impl Error {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::AccessibilityPermission => 3,
            Self::Screenshot(_) => 4,
            Self::Output(_) => 5,
            Self::Display(_) | Self::WindowListUnavailable | Self::Runtime(_) => 1,
        }
    }
}

pub fn run(cli: &Cli) -> Result<(), Error> {
    let mut snapshot = collect(cli)?;

    if let Some(path) = cli.screenshot.as_ref() {
        screenshot::capture_main_display(
            path,
            snapshot.display.as_ref(),
            snapshot.displays.as_deref(),
        )?;
        snapshot.screenshot_path = Some(path.display().to_string());
    }

    output::print(&snapshot, cli.json, cli.verbosity)?;
    Ok(())
}

fn collect(cli: &Cli) -> Result<Snapshot, Error> {
    ax::ensure_trusted().map_err(|error| match error {
        AxErrorKind::NotTrusted => Error::AccessibilityPermission,
        other => Error::Runtime(other.to_string()),
    })?;

    let displays = collect_displays()?;
    let single_display = displays.len() == 1;
    let frontmost = frontmost_application();
    let focused_window_id = focused_window_id();
    let raw_windows = collect_windows(cli)?;
    let mut windows = Vec::with_capacity(raw_windows.len());

    for (z_order, raw) in raw_windows.into_iter().enumerate() {
        let display_id = if single_display {
            None
        } else {
            best_display_id(&raw.bounds, &displays)
        };
        let is_frontmost_app = frontmost
            .as_ref()
            .is_some_and(|app| app.pid == raw.owner_pid);
        let is_focused = focused_window_id == Some(raw.window_id);
        let mut record = WindowInfo {
            window_id: raw.window_id,
            z_order,
            owner: application_info(raw.owner_pid, raw.owner_name.as_deref()),
            title: raw.title,
            role: None,
            subrole: None,
            bounds: raw.bounds,
            display_id,
            layer: raw.layer,
            alpha: raw.alpha,
            is_onscreen: raw.is_onscreen,
            is_minimized: None,
            is_main: None,
            is_frontmost_app,
            is_focused,
            ax_errors: None,
            raw: None,
        };

        enrich_from_ax(&mut record, cli.verbosity);
        windows.push(record);
    }

    let (display, displays) = if single_display {
        (displays.into_iter().next(), None)
    } else {
        (None, Some(displays))
    };

    Ok(Snapshot {
        frontmost_app: frontmost,
        focused_window_id,
        display,
        displays,
        windows,
        screenshot_path: None,
    })
}

fn collect_displays() -> Result<Vec<DisplayInfo>, Error> {
    let mut count = 0;
    let error = unsafe { CGGetActiveDisplayList(0, std::ptr::null_mut(), &mut count) };
    if error != objc2_core_graphics::CGError::Success {
        return Err(Error::Display(error));
    }

    let mut ids = vec![0_u32; count as usize];
    let error = unsafe { CGGetActiveDisplayList(count, ids.as_mut_ptr(), &mut count) };
    if error != objc2_core_graphics::CGError::Success {
        return Err(Error::Display(error));
    }

    let main_id = CGMainDisplayID();
    Ok(ids
        .into_iter()
        .take(count as usize)
        .map(|id| {
            let bounds = CGDisplayBounds(id);
            DisplayInfo {
                id,
                main: id == main_id,
                bounds: Rect::from_xywh(
                    bounds.origin.x,
                    bounds.origin.y,
                    bounds.size.width,
                    bounds.size.height,
                ),
                pixel_width: CGDisplayPixelsWide(id),
                pixel_height: CGDisplayPixelsHigh(id),
            }
        })
        .collect())
}

fn collect_windows(cli: &Cli) -> Result<Vec<RawWindow>, Error> {
    let mut options = if cli.all || cli.include_hidden {
        CGWindowListOption::OptionAll
    } else {
        CGWindowListOption::OptionOnScreenOnly
    };
    if !cli.all && !cli.include_system {
        options |= CGWindowListOption::ExcludeDesktopElements;
    }

    let array = CGWindowListCopyWindowInfo(options, 0).ok_or(Error::WindowListUnavailable)?;
    let entries: CFRetained<CFArray<CFDictionary<CFString, CFType>>> =
        unsafe { CFRetained::cast_unchecked(array) };

    entries
        .iter()
        .enumerate()
        .filter_map(|(z_order, entry)| parse_window(&entry, z_order, cli).transpose())
        .collect()
}

fn parse_window(
    entry: &CFDictionary<CFString, CFType>,
    _z_order: usize,
    cli: &Cli,
) -> Result<Option<RawWindow>, Error> {
    let Some(window_id) =
        number(entry, unsafe { kCGWindowNumber }).and_then(|v| u32::try_from(v).ok())
    else {
        return Ok(None);
    };
    let Some(bounds) = bounds(entry, unsafe { kCGWindowBounds }) else {
        return Ok(None);
    };
    let owner_pid = number(entry, unsafe { kCGWindowOwnerPID })
        .and_then(|v| i32::try_from(v).ok())
        .unwrap_or(-1);
    let owner_name = string(entry, unsafe { kCGWindowOwnerName });
    let is_onscreen = boolean(entry, unsafe { kCGWindowIsOnscreen }).unwrap_or(false);
    let alpha = float(entry, unsafe { kCGWindowAlpha }).unwrap_or(1.0);
    let layer = number(entry, unsafe { kCGWindowLayer })
        .and_then(|v| i32::try_from(v).ok())
        .unwrap_or_default();

    if !cli.all {
        if bounds.area() <= 0.0 || alpha <= 0.0 || owner_pid <= 0 {
            return Ok(None);
        }
        if !cli.include_hidden && !is_onscreen {
            return Ok(None);
        }
        if !cli.include_system && is_system_window(owner_name.as_deref(), layer) {
            return Ok(None);
        }
    }
    if cli.window_id.is_some_and(|id| id != window_id)
        || cli.pid.is_some_and(|pid| pid != owner_pid)
        || cli.app.as_ref().is_some_and(|app| {
            owner_name.as_deref() != Some(app.as_str())
                && application_bundle_id(owner_pid).as_deref() != Some(app.as_str())
        })
    {
        return Ok(None);
    }

    Ok(Some(RawWindow {
        window_id,
        owner_pid,
        owner_name,
        title: string(entry, unsafe { kCGWindowName }),
        bounds,
        layer,
        alpha,
        is_onscreen,
    }))
}

struct RawWindow {
    window_id: u32,
    owner_pid: i32,
    owner_name: Option<String>,
    title: Option<String>,
    bounds: Rect,
    layer: i32,
    alpha: f32,
    is_onscreen: bool,
}

fn enrich_from_ax(window: &mut WindowInfo, verbosity: u8) {
    let mut errors = BTreeMap::new();
    let application = AxElement::application(window.owner.pid);
    let ax_window = match application.get(attributes::WINDOWS) {
        Ok(Some(ax_windows)) => ax_windows
            .into_iter()
            .find(|candidate| candidate.native_window_id().ok().flatten() == Some(window.window_id))
            .or_else(|| {
                application
                    .get(attributes::FOCUSED_WINDOW)
                    .ok()
                    .flatten()
                    .filter(|candidate| {
                        candidate.native_window_id().ok().flatten() == Some(window.window_id)
                    })
            }),
        Ok(None) => None,
        Err(error) => {
            errors.insert("AXWindows".to_owned(), error.to_string());
            None
        }
    };

    let Some(ax_window) = ax_window else {
        if verbosity >= 2 && errors.is_empty() {
            errors.insert(
                "window_match".to_owned(),
                "AX window could not be matched".to_owned(),
            );
        }
        window.ax_errors = (verbosity >= 2 && !errors.is_empty()).then_some(errors);
        return;
    };

    read_attr(
        &ax_window,
        attributes::TITLE,
        &mut window.title,
        "AXTitle",
        &mut errors,
    );
    read_attr(
        &ax_window,
        attributes::ROLE,
        &mut window.role,
        "AXRole",
        &mut errors,
    );
    read_attr(
        &ax_window,
        attributes::SUBROLE,
        &mut window.subrole,
        "AXSubrole",
        &mut errors,
    );
    read_attr(
        &ax_window,
        attributes::MINIMIZED,
        &mut window.is_minimized,
        "AXMinimized",
        &mut errors,
    );
    read_attr(
        &ax_window,
        attributes::MAIN,
        &mut window.is_main,
        "AXMain",
        &mut errors,
    );

    if verbosity >= 2 {
        let mut raw = BTreeMap::new();
        raw.insert(
            "owner_name".to_owned(),
            window.owner.name.clone().unwrap_or_default(),
        );
        raw.insert("layer".to_owned(), window.layer.to_string());
        raw.insert("alpha".to_owned(), window.alpha.to_string());
        window.raw = Some(raw);
    }
    window.ax_errors = (verbosity >= 2 && !errors.is_empty()).then_some(errors);
}

fn read_attr<T: ax::FromAxValue>(
    element: &AxElement,
    attribute: ax::AxAttribute<T>,
    destination: &mut Option<T>,
    name: &str,
    errors: &mut BTreeMap<String, String>,
) {
    match element.get(attribute) {
        Ok(value) => *destination = value,
        Err(error) => {
            errors.insert(name.to_owned(), error.to_string());
        }
    }
}

fn focused_window_id() -> Option<u32> {
    ax::System::new()
        .focused_window()
        .ok()
        .flatten()
        .and_then(|window| window.native_window_id().ok().flatten())
}

fn frontmost_application() -> Option<ApplicationInfo> {
    let application = NSWorkspace::sharedWorkspace().frontmostApplication()?;
    Some(application_info_from_running_application(&application))
}

fn application_info(pid: i32, owner_name: Option<&str>) -> ApplicationInfo {
    NSRunningApplication::runningApplicationWithProcessIdentifier(pid)
        .map(|application| application_info_from_running_application(&application))
        .unwrap_or_else(|| ApplicationInfo {
            pid,
            name: owner_name.map(str::to_owned),
            bundle_id: None,
        })
}

fn application_info_from_running_application(
    application: &NSRunningApplication,
) -> ApplicationInfo {
    ApplicationInfo {
        pid: application.processIdentifier(),
        name: application.localizedName().map(|value| value.to_string()),
        bundle_id: application
            .bundleIdentifier()
            .map(|value| value.to_string()),
    }
}

fn application_bundle_id(pid: i32) -> Option<String> {
    NSRunningApplication::runningApplicationWithProcessIdentifier(pid).and_then(|application| {
        application
            .bundleIdentifier()
            .map(|value| value.to_string())
    })
}

fn best_display_id(bounds: &Rect, displays: &[DisplayInfo]) -> Option<u32> {
    displays
        .iter()
        .max_by(|left, right| {
            left.bounds
                .intersection_area(bounds)
                .total_cmp(&right.bounds.intersection_area(bounds))
                .then_with(|| right.main.cmp(&left.main))
                .then_with(|| right.id.cmp(&left.id))
        })
        .map(|display| display.id)
}

fn is_system_window(owner_name: Option<&str>, layer: i32) -> bool {
    if layer < 0 {
        return true;
    }
    matches!(
        owner_name.map(|name| name.to_ascii_lowercase()),
        Some(name) if matches!(
            name.as_str(),
            "dock" | "window server" | "systemuiserver" | "control center" | "controlcenter" | "notification center" | "loginwindow"
        )
    )
}

fn number(dictionary: &CFDictionary<CFString, CFType>, key: &CFString) -> Option<i64> {
    dictionary.get(key)?.downcast::<CFNumber>().ok()?.as_i64()
}

fn float(dictionary: &CFDictionary<CFString, CFType>, key: &CFString) -> Option<f32> {
    dictionary.get(key)?.downcast::<CFNumber>().ok()?.as_f32()
}

fn boolean(dictionary: &CFDictionary<CFString, CFType>, key: &CFString) -> Option<bool> {
    dictionary
        .get(key)?
        .downcast::<objc2_core_foundation::CFBoolean>()
        .ok()
        .map(|value| value.as_bool())
}

fn string(dictionary: &CFDictionary<CFString, CFType>, key: &CFString) -> Option<String> {
    dictionary
        .get(key)?
        .downcast::<CFString>()
        .ok()
        .map(|value| value.to_string())
}

fn bounds(dictionary: &CFDictionary<CFString, CFType>, key: &CFString) -> Option<Rect> {
    let value = dictionary.get(key)?;
    let nested = value.downcast::<CFDictionary>().ok()?;
    let nested: CFRetained<CFDictionary<CFString, CFType>> =
        unsafe { CFRetained::cast_unchecked(nested) };
    Some(Rect::from_xywh(
        number_by_name(&nested, "X")? as f64,
        number_by_name(&nested, "Y")? as f64,
        number_by_name(&nested, "Width")? as f64,
        number_by_name(&nested, "Height")? as f64,
    ))
}

fn number_by_name(dictionary: &CFDictionary<CFString, CFType>, name: &str) -> Option<i64> {
    let key = CFString::from_str(name);
    number(dictionary, &key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_association_uses_maximum_overlap() {
        let displays = vec![
            DisplayInfo {
                id: 1,
                main: true,
                bounds: Rect::from_xywh(0.0, 0.0, 100.0, 100.0),
                pixel_width: 100,
                pixel_height: 100,
            },
            DisplayInfo {
                id: 2,
                main: false,
                bounds: Rect::from_xywh(100.0, 0.0, 100.0, 100.0),
                pixel_width: 100,
                pixel_height: 100,
            },
        ];
        assert_eq!(
            best_display_id(&Rect::from_xywh(90.0, 0.0, 30.0, 30.0), &displays),
            Some(2)
        );
    }

    #[test]
    fn system_shell_windows_are_filtered_by_default() {
        assert!(is_system_window(Some("Dock"), 0));
        assert!(!is_system_window(Some("Safari"), 0));
    }
}
