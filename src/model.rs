use std::collections::BTreeMap;

use serde::Serialize;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    #[allow(dead_code)]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

impl Size {
    #[allow(dead_code)]
    pub const fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub const fn new(origin: Point, size: Size) -> Self {
        Self {
            x: origin.x,
            y: origin.y,
            width: size.width,
            height: size.height,
        }
    }

    pub const fn from_xywh(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn area(&self) -> f64 {
        self.width.max(0.0) * self.height.max(0.0)
    }

    pub fn intersection_area(&self, other: &Self) -> f64 {
        let x = (self.x + self.width).min(other.x + other.width) - self.x.max(other.x);
        let y = (self.y + self.height).min(other.y + other.height) - self.y.max(other.y);
        x.max(0.0) * y.max(0.0)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DisplayInfo {
    pub id: u32,
    pub main: bool,
    pub bounds: Rect,
    pub pixel_width: usize,
    pub pixel_height: usize,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ApplicationInfo {
    pub pid: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bundle_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WindowInfo {
    pub window_id: u32,
    pub z_order: usize,
    pub owner: ApplicationInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subrole: Option<String>,
    pub bounds: Rect,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_id: Option<u32>,
    pub layer: i32,
    pub alpha: f32,
    pub is_onscreen: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_minimized: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_main: Option<bool>,
    pub is_frontmost_app: bool,
    pub is_focused: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ax_errors: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub frontmost_app: Option<ApplicationInfo>,
    pub focused_window_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<DisplayInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub displays: Option<Vec<DisplayInfo>>,
    pub windows: Vec<WindowInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screenshot_path: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intersection_area_is_zero_for_disjoint_rectangles() {
        let left = Rect::from_xywh(0.0, 0.0, 10.0, 10.0);
        let right = Rect::from_xywh(20.0, 0.0, 10.0, 10.0);
        assert_eq!(left.intersection_area(&right), 0.0);
    }

    #[test]
    fn intersection_area_handles_partial_overlap() {
        let left = Rect::from_xywh(0.0, 0.0, 10.0, 10.0);
        let right = Rect::from_xywh(5.0, 5.0, 10.0, 10.0);
        assert_eq!(left.intersection_area(&right), 25.0);
    }
}
