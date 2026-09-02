use std::cell::RefCell;
use std::ffi::{CString, c_char, c_void};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;
use std::slice;
use std::str;

use egui_states::ObjectType;
use egui_states::c_api::{
    DynamicDataType, DynamicImageFormat, DynamicImageView, DynamicServer, DynamicSignalEvent,
    DynamicValue, Error, ErrorKind,
};

pub const ABI_VERSION: u32 = 1;

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum egui_states_status_t {
    EGUI_STATES_STATUS_OK = 0,
    EGUI_STATES_STATUS_INVALID_ARGUMENT = 1,
    EGUI_STATES_STATUS_INVALID_STATE = 2,
    EGUI_STATES_STATUS_TYPE_MISMATCH = 3,
    EGUI_STATES_STATUS_NOT_FOUND = 4,
    EGUI_STATES_STATUS_OUT_OF_RANGE = 5,
    EGUI_STATES_STATUS_BUFFER_TOO_SMALL = 6,
    EGUI_STATES_STATUS_TIMEOUT = 7,
    EGUI_STATES_STATUS_IO = 8,
    EGUI_STATES_STATUS_INTERNAL = 9,
    EGUI_STATES_STATUS_PANIC = 10,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum egui_states_value_kind_t {
    EGUI_STATES_VALUE_U8 = 0,
    EGUI_STATES_VALUE_U16 = 1,
    EGUI_STATES_VALUE_U32 = 2,
    EGUI_STATES_VALUE_U64 = 3,
    EGUI_STATES_VALUE_I8 = 4,
    EGUI_STATES_VALUE_I16 = 5,
    EGUI_STATES_VALUE_I32 = 6,
    EGUI_STATES_VALUE_I64 = 7,
    EGUI_STATES_VALUE_F32 = 8,
    EGUI_STATES_VALUE_F64 = 9,
    EGUI_STATES_VALUE_BOOL = 10,
    EGUI_STATES_VALUE_STRING = 11,
    EGUI_STATES_VALUE_ENUM = 12,
    EGUI_STATES_VALUE_SEQUENCE = 13,
    EGUI_STATES_VALUE_MAP = 14,
    EGUI_STATES_VALUE_OPTION = 15,
    EGUI_STATES_VALUE_EMPTY = 16,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum egui_states_data_type_t {
    EGUI_STATES_DATA_U8 = 0,
    EGUI_STATES_DATA_U16 = 1,
    EGUI_STATES_DATA_U32 = 2,
    EGUI_STATES_DATA_U64 = 3,
    EGUI_STATES_DATA_I8 = 4,
    EGUI_STATES_DATA_I16 = 5,
    EGUI_STATES_DATA_I32 = 6,
    EGUI_STATES_DATA_I64 = 7,
    EGUI_STATES_DATA_F32 = 8,
    EGUI_STATES_DATA_F64 = 9,
}

pub(crate) fn data_type(value: u32) -> Result<DynamicDataType, Error> {
    let value = u8::try_from(value).map_err(|_| invalid(format!("invalid data type {value}")))?;
    DynamicDataType::from_id(value)
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum egui_states_image_format_t {
    EGUI_STATES_IMAGE_GRAY = 0,
    EGUI_STATES_IMAGE_GRAY_ALPHA = 1,
    EGUI_STATES_IMAGE_RGB = 2,
    EGUI_STATES_IMAGE_RGBA = 3,
}

fn image_format(value: u32) -> Result<DynamicImageFormat, Error> {
    match value {
        0 => Ok(DynamicImageFormat::Gray),
        1 => Ok(DynamicImageFormat::GrayAlpha),
        2 => Ok(DynamicImageFormat::Rgb),
        3 => Ok(DynamicImageFormat::Rgba),
        _ => Err(invalid(format!("invalid image format {value}"))),
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct egui_states_string_view_t {
    pub data: *const c_char,
    pub length: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct egui_states_data_view_t {
    pub data: *const c_void,
    pub byte_length: usize,
    pub element_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct egui_states_image_view_t {
    pub data: *const u8,
    pub byte_length: usize,
    pub height: usize,
    pub width: usize,
    pub row_stride: usize,
    pub format: u32,
}

#[repr(C)]
pub struct egui_states_server_t {
    pub(crate) inner: DynamicServer,
}

#[repr(transparent)]
pub struct egui_states_object_type_t {
    pub(crate) inner: ObjectType,
}

#[repr(transparent)]
pub struct egui_states_value_t {
    pub(crate) inner: DynamicValue,
}

#[repr(transparent)]
pub struct egui_states_signal_event_t {
    pub(crate) inner: DynamicSignalEvent,
}

thread_local! {
    static LAST_ERROR: RefCell<CString> = RefCell::new(CString::default());
}

fn set_last_error(message: &str) {
    let sanitized = message.replace('\0', "\\0");
    let value = CString::new(sanitized).unwrap_or_default();
    LAST_ERROR.with(|slot| *slot.borrow_mut() = value);
}

fn status_for(error: &Error) -> egui_states_status_t {
    match error.kind {
        ErrorKind::InvalidArgument => egui_states_status_t::EGUI_STATES_STATUS_INVALID_ARGUMENT,
        ErrorKind::InvalidState => egui_states_status_t::EGUI_STATES_STATUS_INVALID_STATE,
        ErrorKind::TypeMismatch => egui_states_status_t::EGUI_STATES_STATUS_TYPE_MISMATCH,
        ErrorKind::NotFound => egui_states_status_t::EGUI_STATES_STATUS_NOT_FOUND,
        ErrorKind::OutOfRange => egui_states_status_t::EGUI_STATES_STATUS_OUT_OF_RANGE,
        ErrorKind::BufferTooSmall => egui_states_status_t::EGUI_STATES_STATUS_BUFFER_TOO_SMALL,
        ErrorKind::Timeout => egui_states_status_t::EGUI_STATES_STATUS_TIMEOUT,
        ErrorKind::Io => egui_states_status_t::EGUI_STATES_STATUS_IO,
        ErrorKind::Internal => egui_states_status_t::EGUI_STATES_STATUS_INTERNAL,
    }
}

pub(crate) fn ffi_call(
    callback: impl FnOnce() -> egui_states::c_api::Result<()>,
) -> egui_states_status_t {
    match catch_unwind(AssertUnwindSafe(callback)) {
        Ok(Ok(())) => egui_states_status_t::EGUI_STATES_STATUS_OK,
        Ok(Err(error)) => {
            set_last_error(error.message());
            status_for(&error)
        }
        Err(_) => {
            set_last_error("panic caught at the egui-states C ABI boundary");
            egui_states_status_t::EGUI_STATES_STATUS_PANIC
        }
    }
}

pub(crate) fn invalid(message: impl Into<String>) -> Error {
    Error::new(ErrorKind::InvalidArgument, message)
}

pub(crate) unsafe fn required_ref<'a, T>(pointer: *const T, name: &str) -> Result<&'a T, Error> {
    if pointer.is_null() {
        return Err(invalid(format!("{name} must not be null")));
    }
    if !(pointer as usize).is_multiple_of(std::mem::align_of::<T>()) {
        return Err(invalid(format!("{name} is not correctly aligned")));
    }
    Ok(unsafe { &*pointer })
}

pub(crate) unsafe fn required_mut<'a, T>(pointer: *mut T, name: &str) -> Result<&'a mut T, Error> {
    if pointer.is_null() {
        return Err(invalid(format!("{name} must not be null")));
    }
    if !(pointer as usize).is_multiple_of(std::mem::align_of::<T>()) {
        return Err(invalid(format!("{name} is not correctly aligned")));
    }
    Ok(unsafe { &mut *pointer })
}

pub(crate) unsafe fn input_slice<'a, T>(
    pointer: *const T,
    length: usize,
    name: &str,
) -> Result<&'a [T], Error> {
    if length == 0 {
        return Ok(&[]);
    }
    if pointer.is_null() {
        return Err(invalid(format!(
            "{name} must not be null when length is nonzero"
        )));
    }
    if !(pointer as usize).is_multiple_of(std::mem::align_of::<T>()) {
        return Err(invalid(format!("{name} is not correctly aligned")));
    }
    let byte_length = length
        .checked_mul(std::mem::size_of::<T>())
        .ok_or_else(|| invalid(format!("{name} length overflows usize")))?;
    if byte_length > isize::MAX as usize {
        return Err(invalid(format!(
            "{name} exceeds the maximum addressable slice size"
        )));
    }
    Ok(unsafe { slice::from_raw_parts(pointer, length) })
}

pub(crate) unsafe fn string_view<'a>(
    view: egui_states_string_view_t,
    name: &str,
) -> Result<&'a str, Error> {
    let bytes = unsafe { input_slice(view.data.cast::<u8>(), view.length, name)? };
    str::from_utf8(bytes).map_err(|_| invalid(format!("{name} must contain valid UTF-8")))
}

pub(crate) fn bool_arg(value: u8, name: &str) -> Result<bool, Error> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(invalid(format!("{name} must be 0 or 1"))),
    }
}

pub(crate) unsafe fn set_box<T>(output: *mut *mut T, value: T, name: &str) -> Result<(), Error> {
    let output = unsafe { required_mut(output, name)? };
    *output = Box::into_raw(Box::new(value));
    Ok(())
}

pub(crate) unsafe fn set_value_box(
    output: *mut *mut egui_states_value_t,
    value: DynamicValue,
) -> Result<(), Error> {
    unsafe { set_box(output, egui_states_value_t { inner: value }, "out_value") }
}

pub(crate) unsafe fn data_view<'a>(view: egui_states_data_view_t) -> Result<&'a [u8], Error> {
    unsafe { input_slice(view.data.cast::<u8>(), view.byte_length, "data.data") }
}

pub(crate) unsafe fn image_view<'a>(
    view: egui_states_image_view_t,
) -> Result<DynamicImageView<'a>, Error> {
    let data = unsafe { input_slice(view.data, view.byte_length, "image.data")? };
    Ok(DynamicImageView {
        data,
        height: view.height,
        width: view.width,
        row_stride: view.row_stride,
        format: image_format(view.format)?,
    })
}

pub(crate) unsafe fn copy_bytes(
    bytes: &[u8],
    destination: *mut u8,
    capacity: usize,
    required: *mut usize,
) -> Result<(), Error> {
    let required = unsafe { required_mut(required, "out_required")? };
    *required = bytes.len();
    if destination.is_null() {
        if capacity == 0 {
            return Ok(());
        }
        return Err(invalid("destination is null but capacity is nonzero"));
    }
    if capacity < bytes.len() {
        return Err(Error::new(
            ErrorKind::BufferTooSmall,
            format!(
                "output buffer holds {capacity} bytes, requires {}",
                bytes.len()
            ),
        ));
    }
    unsafe { ptr::copy_nonoverlapping(bytes.as_ptr(), destination, bytes.len()) };
    Ok(())
}

pub(crate) unsafe fn copy_u32s(
    values: &[u32],
    destination: *mut u32,
    capacity: usize,
    required: *mut usize,
) -> Result<(), Error> {
    let required = unsafe { required_mut(required, "out_required")? };
    *required = values.len();
    if destination.is_null() {
        if capacity == 0 {
            return Ok(());
        }
        return Err(invalid("destination is null but capacity is nonzero"));
    }
    if !(destination as usize).is_multiple_of(std::mem::align_of::<u32>()) {
        return Err(invalid("destination is not correctly aligned for uint32_t"));
    }
    if capacity < values.len() {
        return Err(Error::new(
            ErrorKind::BufferTooSmall,
            format!(
                "output buffer holds {capacity} elements, requires {}",
                values.len()
            ),
        ));
    }
    unsafe { ptr::copy_nonoverlapping(values.as_ptr(), destination, values.len()) };
    Ok(())
}

pub(crate) fn borrowed_value(value: &DynamicValue) -> *const egui_states_value_t {
    (value as *const DynamicValue).cast::<egui_states_value_t>()
}

#[unsafe(no_mangle)]
pub extern "C" fn egui_states_abi_version() -> u32 {
    ABI_VERSION
}

#[unsafe(no_mangle)]
pub extern "C" fn egui_states_last_error_message() -> *const c_char {
    LAST_ERROR.with(|slot| slot.borrow().as_ptr())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_destroy(server: *mut egui_states_server_t) {
    if !server.is_null() {
        let _ = catch_unwind(AssertUnwindSafe(|| unsafe { drop(Box::from_raw(server)) }));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_object_type_destroy(
    object_type: *mut egui_states_object_type_t,
) {
    if !object_type.is_null() {
        let _ = catch_unwind(AssertUnwindSafe(|| unsafe {
            drop(Box::from_raw(object_type))
        }));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_value_destroy(value: *mut egui_states_value_t) {
    if !value.is_null() {
        let _ = catch_unwind(AssertUnwindSafe(|| unsafe { drop(Box::from_raw(value)) }));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_signal_event_destroy(event: *mut egui_states_signal_event_t) {
    if !event.is_null() {
        let _ = catch_unwind(AssertUnwindSafe(|| unsafe { drop(Box::from_raw(event)) }));
    }
}
