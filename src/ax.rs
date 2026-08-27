use std::ffi::c_void;
use std::ptr;
use std::ptr::NonNull;

use objc2_app_kit::NSWorkspace;
use objc2_application_services::{AXError, AXIsProcessTrusted, AXUIElement, AXValue, AXValueType};
use objc2_core_foundation::{CFArray, CFBoolean, CFNumber, CFRetained, CFString, CFType};
use thiserror::Error;

use crate::model::{Point, Size};

pub type AxResult<T> = Result<T, AxErrorKind>;

#[derive(Debug, Error)]
pub enum AxErrorKind {
    #[error("AX operation failed: {0}")]
    Ax(AxErrorDisplay),
    #[error("AX returned success but returned a null pointer")]
    UnexpectedNull,
    #[error("AX value had the wrong type; expected {expected}")]
    TypeMismatch { expected: &'static str },
    #[error("AX attribute {0} has no value")]
    #[allow(dead_code)]
    MissingValue(&'static str),
    #[error("Accessibility permission has not been granted")]
    NotTrusted,
}

#[derive(Debug, Clone, Copy)]
pub struct AxErrorDisplay(pub AXError);

impl std::fmt::Display for AxErrorDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ({}): {}",
            ax_error_name(self.0),
            self.0.0,
            ax_error_description(self.0)
        )
    }
}

fn ax_error_name(error: AXError) -> &'static str {
    match error {
        AXError::Success => "kAXErrorSuccess",
        AXError::Failure => "kAXErrorFailure",
        AXError::IllegalArgument => "kAXErrorIllegalArgument",
        AXError::InvalidUIElement => "kAXErrorInvalidUIElement",
        AXError::InvalidUIElementObserver => "kAXErrorInvalidUIElementObserver",
        AXError::CannotComplete => "kAXErrorCannotComplete",
        AXError::AttributeUnsupported => "kAXErrorAttributeUnsupported",
        AXError::ActionUnsupported => "kAXErrorActionUnsupported",
        AXError::NotificationUnsupported => "kAXErrorNotificationUnsupported",
        AXError::NotImplemented => "kAXErrorNotImplemented",
        AXError::APIDisabled => "kAXErrorAPIDisabled",
        AXError::NoValue => "kAXErrorNoValue",
        AXError::ParameterizedAttributeUnsupported => "kAXErrorParameterizedAttributeUnsupported",
        AXError::NotEnoughPrecision => "kAXErrorNotEnoughPrecision",
        _ => "kAXErrorUnknown",
    }
}

fn ax_error_description(error: AXError) -> &'static str {
    match error {
        AXError::Success => "no error occurred",
        AXError::Failure => "a system error occurred",
        AXError::IllegalArgument => "an illegal argument was passed",
        AXError::InvalidUIElement => "the AXUIElement is invalid",
        AXError::InvalidUIElementObserver => "the AXObserver is invalid",
        AXError::CannotComplete => "messaging failed or the target application is busy",
        AXError::AttributeUnsupported => "the attribute is not supported by this element",
        AXError::ActionUnsupported => "the action is not supported by this element",
        AXError::NotificationUnsupported => "the notification is not supported by this element",
        AXError::NotImplemented => "the operation is not implemented",
        AXError::APIDisabled => "the accessibility API is disabled",
        AXError::NoValue => "the requested value does not exist",
        AXError::ParameterizedAttributeUnsupported => {
            "the parameterized attribute is not supported by this element"
        }
        AXError::NotEnoughPrecision => "the value cannot be represented with enough precision",
        _ => "an unknown accessibility error occurred",
    }
}

fn check(error: AXError) -> AxResult<()> {
    if error == AXError::Success {
        Ok(())
    } else {
        Err(AxErrorKind::Ax(AxErrorDisplay(error)))
    }
}

pub struct AxAttribute<T> {
    name: &'static str,
    _value: std::marker::PhantomData<fn() -> T>,
}

impl<T> AxAttribute<T> {
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            _value: std::marker::PhantomData,
        }
    }

    pub const fn name(self) -> &'static str {
        self.name
    }
}

impl<T> Copy for AxAttribute<T> {}
impl<T> Clone for AxAttribute<T> {
    fn clone(&self) -> Self {
        *self
    }
}

pub mod attributes {
    use super::{AxAttribute, AxElement};
    use crate::model::{Point, Size};

    pub const TITLE: AxAttribute<String> = AxAttribute::new("AXTitle");
    pub const ROLE: AxAttribute<String> = AxAttribute::new("AXRole");
    pub const SUBROLE: AxAttribute<String> = AxAttribute::new("AXSubrole");
    pub const WINDOWS: AxAttribute<Vec<AxElement>> = AxAttribute::new("AXWindows");
    pub const FOCUSED_APPLICATION: AxAttribute<AxElement> =
        AxAttribute::new("AXFocusedApplication");
    pub const FOCUSED_WINDOW: AxAttribute<AxElement> = AxAttribute::new("AXFocusedWindow");
    #[allow(dead_code)]
    pub const POSITION: AxAttribute<Point> = AxAttribute::new("AXPosition");
    #[allow(dead_code)]
    pub const SIZE: AxAttribute<Size> = AxAttribute::new("AXSize");
    pub const MINIMIZED: AxAttribute<bool> = AxAttribute::new("AXMinimized");
    pub const MAIN: AxAttribute<bool> = AxAttribute::new("AXMain");
    #[allow(dead_code)]
    pub const WINDOW_NUMBER: AxAttribute<i64> = AxAttribute::new("AXWindowNumber");
}

pub struct AxElement {
    inner: CFRetained<AXUIElement>,
}

impl Clone for AxElement {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl AxElement {
    pub fn application(pid: libc::pid_t) -> Self {
        let inner = unsafe { AXUIElement::new_application(pid) };
        Self { inner }
    }

    pub fn system_wide() -> Self {
        let inner = unsafe { AXUIElement::new_system_wide() };
        Self { inner }
    }

    fn as_raw(&self) -> &AXUIElement {
        self.inner.as_ref()
    }

    #[allow(dead_code)]
    pub fn pid(&self) -> AxResult<libc::pid_t> {
        let mut pid = 0;
        let error = unsafe { self.inner.pid(NonNull::from(&mut pid)) };
        check(error)?;
        Ok(pid)
    }

    pub fn get<T: FromAxValue>(&self, attribute: AxAttribute<T>) -> AxResult<Option<T>> {
        self.copy_attribute_raw(attribute.name())?
            .map(T::from_ax_value)
            .transpose()
    }

    #[allow(dead_code)]
    pub fn require<T: FromAxValue>(&self, attribute: AxAttribute<T>) -> AxResult<T> {
        self.get(attribute)?
            .ok_or(AxErrorKind::MissingValue(attribute.name()))
    }

    #[allow(dead_code)]
    pub fn title(&self) -> AxResult<Option<String>> {
        self.get(attributes::TITLE)
    }

    #[allow(dead_code)]
    pub fn role(&self) -> AxResult<Option<String>> {
        self.get(attributes::ROLE)
    }

    #[allow(dead_code)]
    pub fn subrole(&self) -> AxResult<Option<String>> {
        self.get(attributes::SUBROLE)
    }

    #[allow(dead_code)]
    pub fn position(&self) -> AxResult<Option<Point>> {
        self.get(attributes::POSITION)
    }

    #[allow(dead_code)]
    pub fn size(&self) -> AxResult<Option<Size>> {
        self.get(attributes::SIZE)
    }

    #[allow(dead_code)]
    pub fn frame(&self) -> AxResult<Option<crate::model::Rect>> {
        let Some(position) = self.position()? else {
            return Ok(None);
        };
        let Some(size) = self.size()? else {
            return Ok(None);
        };
        Ok(Some(crate::model::Rect::new(position, size)))
    }

    pub fn native_window_id(&self) -> AxResult<Option<u32>> {
        let mut window_id = 0;
        let error = unsafe { _AXUIElementGetWindow(self.as_raw(), &mut window_id) };
        if error == AXError::Success {
            Ok((window_id != 0).then_some(window_id))
        } else {
            Err(AxErrorKind::Ax(AxErrorDisplay(error)))
        }
    }

    fn copy_attribute_raw(&self, name: &str) -> AxResult<Option<CFRetained<CFType>>> {
        let attribute = CFString::from_str(name);
        let mut output: *const CFType = ptr::null();
        let error = unsafe {
            self.inner
                .copy_attribute_value(&attribute, NonNull::from(&mut output))
        };

        if error == AXError::NoValue {
            return Ok(None);
        }
        check(error)?;
        let output = NonNull::new(output.cast_mut()).ok_or(AxErrorKind::UnexpectedNull)?;
        Ok(Some(unsafe { CFRetained::from_raw(output) }))
    }
}

unsafe extern "C" {
    fn _AXUIElementGetWindow(element: &AXUIElement, window_id: *mut u32) -> AXError;
}

pub struct System {
    element: AxElement,
}

impl Default for System {
    fn default() -> Self {
        Self::new()
    }
}

impl System {
    pub fn new() -> Self {
        Self {
            element: AxElement::system_wide(),
        }
    }

    pub fn focused_application(&self) -> AxResult<Option<AxElement>> {
        match self.element.get(attributes::FOCUSED_APPLICATION) {
            Ok(application) => Ok(application),
            Err(AxErrorKind::Ax(AxErrorDisplay(error))) if error == AXError::CannotComplete => {
                Ok(frontmost_element())
            }
            Err(error) => Err(error),
        }
    }

    pub fn focused_window(&self) -> AxResult<Option<AxElement>> {
        let Some(application) = self.focused_application()? else {
            return Ok(None);
        };
        match application.get(attributes::FOCUSED_WINDOW) {
            Ok(window) => Ok(window),
            Err(AxErrorKind::Ax(AxErrorDisplay(error))) if error == AXError::CannotComplete => {
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }
}

fn frontmost_element() -> Option<AxElement> {
    let application = NSWorkspace::sharedWorkspace().frontmostApplication()?;
    Some(AxElement::application(application.processIdentifier()))
}

pub fn is_trusted() -> bool {
    unsafe { AXIsProcessTrusted() }
}

pub fn ensure_trusted() -> AxResult<()> {
    is_trusted().then_some(()).ok_or(AxErrorKind::NotTrusted)
}

pub trait FromAxValue: Sized {
    const EXPECTED: &'static str;
    fn from_ax_value(value: CFRetained<CFType>) -> AxResult<Self>;
}

impl FromAxValue for String {
    const EXPECTED: &'static str = "CFString";
    fn from_ax_value(value: CFRetained<CFType>) -> AxResult<Self> {
        value
            .downcast::<CFString>()
            .map(|value| value.to_string())
            .map_err(|_| AxErrorKind::TypeMismatch {
                expected: Self::EXPECTED,
            })
    }
}

impl FromAxValue for bool {
    const EXPECTED: &'static str = "CFBoolean";
    fn from_ax_value(value: CFRetained<CFType>) -> AxResult<Self> {
        value
            .downcast::<CFBoolean>()
            .map(|value| value.as_bool())
            .map_err(|_| AxErrorKind::TypeMismatch {
                expected: Self::EXPECTED,
            })
    }
}

impl FromAxValue for i64 {
    const EXPECTED: &'static str = "CFNumber";
    fn from_ax_value(value: CFRetained<CFType>) -> AxResult<Self> {
        value
            .downcast::<CFNumber>()
            .map_err(|_| AxErrorKind::TypeMismatch {
                expected: Self::EXPECTED,
            })?
            .as_i64()
            .ok_or(AxErrorKind::TypeMismatch {
                expected: Self::EXPECTED,
            })
    }
}

impl FromAxValue for AxElement {
    const EXPECTED: &'static str = "AXUIElement";
    fn from_ax_value(value: CFRetained<CFType>) -> AxResult<Self> {
        value
            .downcast::<AXUIElement>()
            .map(|inner| Self { inner })
            .map_err(|_| AxErrorKind::TypeMismatch {
                expected: Self::EXPECTED,
            })
    }
}

impl<T: FromAxValue> FromAxValue for Vec<T> {
    const EXPECTED: &'static str = "CFArray";
    fn from_ax_value(value: CFRetained<CFType>) -> AxResult<Self> {
        let array = value
            .downcast::<CFArray>()
            .map_err(|_| AxErrorKind::TypeMismatch {
                expected: Self::EXPECTED,
            })?;
        let array: CFRetained<CFArray<CFType>> = unsafe { CFRetained::cast_unchecked(array) };
        array.iter().map(T::from_ax_value).collect()
    }
}

#[allow(dead_code)]
fn decode_ax_value<T>(
    value: CFRetained<CFType>,
    value_type: AXValueType,
    expected: &'static str,
    mut output: T,
) -> AxResult<T> {
    let value = value
        .downcast::<AXValue>()
        .map_err(|_| AxErrorKind::TypeMismatch { expected })?;
    let success = unsafe { value.value(value_type, NonNull::from(&mut output).cast::<c_void>()) };
    success
        .then_some(output)
        .ok_or(AxErrorKind::TypeMismatch { expected })
}

impl FromAxValue for Point {
    const EXPECTED: &'static str = "AXValue(CGPoint)";
    fn from_ax_value(value: CFRetained<CFType>) -> AxResult<Self> {
        let point = decode_ax_value(
            value,
            AXValueType::CGPoint,
            Self::EXPECTED,
            objc2_core_foundation::CGPoint::ZERO,
        )?;
        Ok(Point::new(point.x, point.y))
    }
}

impl FromAxValue for Size {
    const EXPECTED: &'static str = "AXValue(CGSize)";
    fn from_ax_value(value: CFRetained<CFType>) -> AxResult<Self> {
        let size = decode_ax_value(
            value,
            AXValueType::CGSize,
            Self::EXPECTED,
            objc2_core_foundation::CGSize::ZERO,
        )?;
        Ok(Size::new(size.width, size.height))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_attributes_keep_native_names() {
        assert_eq!(attributes::TITLE.name(), "AXTitle");
        assert_eq!(attributes::WINDOW_NUMBER.name(), "AXWindowNumber");
    }

    #[test]
    fn unknown_ax_error_names_remain_visible() {
        assert_eq!(ax_error_name(AXError(-32000)), "kAXErrorUnknown");
    }
}
