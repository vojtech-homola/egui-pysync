use std::ffi::c_char;

use egui_states::ObjectType;
use egui_states::c_api::{DynamicValue, Error, ErrorKind};

use crate::abi::{
    bool_arg, borrowed_value, egui_states_object_type_t, egui_states_status_t,
    egui_states_string_view_t, egui_states_value_kind_t, egui_states_value_t, ffi_call,
    input_slice, invalid, required_mut, required_ref, set_box, set_value_box, string_view,
};

fn type_mismatch(expected: &str, value: &DynamicValue) -> Error {
    Error::new(
        ErrorKind::TypeMismatch,
        format!("expected {expected}, received {}", value.kind_name()),
    )
}

unsafe fn set_type(
    output: *mut *mut egui_states_object_type_t,
    object_type: ObjectType,
) -> Result<(), Error> {
    unsafe {
        set_box(
            output,
            egui_states_object_type_t { inner: object_type },
            "out_type",
        )
    }
}

macro_rules! primitive_type_functions {
    ($(($name:ident, $variant:ident)),* $(,)?) => {
        $(
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn $name(
                output: *mut *mut egui_states_object_type_t,
            ) -> egui_states_status_t {
                ffi_call(|| unsafe { set_type(output, ObjectType::$variant) })
            }
        )*
    };
}

primitive_type_functions! {
    (egui_states_object_type_u8, U8),
    (egui_states_object_type_u16, U16),
    (egui_states_object_type_u32, U32),
    (egui_states_object_type_u64, U64),
    (egui_states_object_type_i8, I8),
    (egui_states_object_type_i16, I16),
    (egui_states_object_type_i32, I32),
    (egui_states_object_type_i64, I64),
    (egui_states_object_type_f32, F32),
    (egui_states_object_type_f64, F64),
    (egui_states_object_type_bool, Bool),
    (egui_states_object_type_string, String),
    (egui_states_object_type_empty, Empty),
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_object_type_clone(
    object_type: *const egui_states_object_type_t,
    output: *mut *mut egui_states_object_type_t,
) -> egui_states_status_t {
    ffi_call(|| {
        let object_type = unsafe { required_ref(object_type, "object_type")? };
        unsafe { set_type(output, object_type.inner.clone()) }
    })
}

unsafe fn object_types(
    values: *const *const egui_states_object_type_t,
    count: usize,
    name: &str,
) -> Result<Vec<ObjectType>, Error> {
    let values = unsafe { input_slice(values, count, name)? };
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let value = unsafe { required_ref(*value, &format!("{name}[{index}]"))? };
            if matches!(value.inner, ObjectType::Empty) {
                return Err(invalid(format!("{name}[{index}] cannot be the empty type")));
            }
            Ok(value.inner.clone())
        })
        .collect()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_object_type_option(
    inner: *const egui_states_object_type_t,
    output: *mut *mut egui_states_object_type_t,
) -> egui_states_status_t {
    ffi_call(|| {
        let inner = unsafe { required_ref(inner, "inner")? };
        unsafe { set_type(output, ObjectType::Option(Box::new(inner.inner.clone()))) }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_object_type_tuple(
    elements: *const *const egui_states_object_type_t,
    count: usize,
    output: *mut *mut egui_states_object_type_t,
) -> egui_states_status_t {
    ffi_call(|| {
        let elements = unsafe { object_types(elements, count, "elements")? };
        unsafe { set_type(output, ObjectType::Tuple(elements)) }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_object_type_struct(
    name: egui_states_string_view_t,
    field_names: *const egui_states_string_view_t,
    field_types: *const *const egui_states_object_type_t,
    count: usize,
    output: *mut *mut egui_states_object_type_t,
) -> egui_states_status_t {
    ffi_call(|| {
        let name = unsafe { string_view(name, "name")? }.to_owned();
        let field_names = unsafe { input_slice(field_names, count, "field_names")? };
        let field_types = unsafe { object_types(field_types, count, "field_types")? };
        let mut fields = Vec::with_capacity(count);
        for (index, (field_name, field_type)) in field_names.iter().zip(field_types).enumerate() {
            fields.push((
                unsafe { string_view(*field_name, &format!("field_names[{index}]"))? }.to_owned(),
                field_type,
            ));
        }
        unsafe { set_type(output, ObjectType::Struct(name, fields)) }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_object_type_list(
    element: *const egui_states_object_type_t,
    size: u32,
    output: *mut *mut egui_states_object_type_t,
) -> egui_states_status_t {
    ffi_call(|| {
        let element = unsafe { required_ref(element, "element")? };
        if matches!(element.inner, ObjectType::Empty) {
            return Err(invalid("fixed lists cannot contain the empty type"));
        }
        unsafe {
            set_type(
                output,
                ObjectType::List(size, Box::new(element.inner.clone())),
            )
        }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_object_type_vec(
    element: *const egui_states_object_type_t,
    output: *mut *mut egui_states_object_type_t,
) -> egui_states_status_t {
    ffi_call(|| {
        let element = unsafe { required_ref(element, "element")? };
        if matches!(element.inner, ObjectType::Empty) {
            return Err(invalid("vectors cannot contain the empty type"));
        }
        unsafe { set_type(output, ObjectType::Vec(Box::new(element.inner.clone()))) }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_object_type_map(
    key: *const egui_states_object_type_t,
    value: *const egui_states_object_type_t,
    output: *mut *mut egui_states_object_type_t,
) -> egui_states_status_t {
    ffi_call(|| {
        let key = unsafe { required_ref(key, "key")? };
        let value = unsafe { required_ref(value, "value")? };
        if matches!(key.inner, ObjectType::Empty) || matches!(value.inner, ObjectType::Empty) {
            return Err(invalid("maps cannot contain the empty type"));
        }
        unsafe {
            set_type(
                output,
                ObjectType::Map(Box::new(key.inner.clone()), Box::new(value.inner.clone())),
            )
        }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_object_type_enum(
    name: egui_states_string_view_t,
    variant_names: *const egui_states_string_view_t,
    discriminants: *const i32,
    count: usize,
    output: *mut *mut egui_states_object_type_t,
) -> egui_states_status_t {
    ffi_call(|| {
        let name = unsafe { string_view(name, "name")? }.to_owned();
        let names = unsafe { input_slice(variant_names, count, "variant_names")? };
        let discriminants = unsafe { input_slice(discriminants, count, "discriminants")? };
        let mut variants = Vec::with_capacity(count);
        for (index, (variant, discriminant)) in names.iter().zip(discriminants).enumerate() {
            variants.push((
                unsafe { string_view(*variant, &format!("variant_names[{index}]"))? }.to_owned(),
                *discriminant,
            ));
        }
        unsafe { set_type(output, ObjectType::Enum(name, variants)) }
    })
}

macro_rules! scalar_value_functions {
    ($(($create:ident, $get:ident, $rust:ty, $variant:ident, $label:literal)),* $(,)?) => {
        $(
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn $create(
                value: $rust,
                output: *mut *mut egui_states_value_t,
            ) -> egui_states_status_t {
                ffi_call(|| unsafe { set_value_box(output, DynamicValue::$variant(value)) })
            }

            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn $get(
                value: *const egui_states_value_t,
                output: *mut $rust,
            ) -> egui_states_status_t {
                ffi_call(|| {
                    let value = unsafe { required_ref(value, "value")? };
                    let output = unsafe { required_mut(output, "out_value")? };
                    match &value.inner {
                        DynamicValue::$variant(value) => {
                            *output = *value;
                            Ok(())
                        }
                        other => Err(type_mismatch($label, other)),
                    }
                })
            }
        )*
    };
}

scalar_value_functions! {
    (egui_states_value_create_u8, egui_states_value_get_u8, u8, U8, "u8"),
    (egui_states_value_create_u16, egui_states_value_get_u16, u16, U16, "u16"),
    (egui_states_value_create_u32, egui_states_value_get_u32, u32, U32, "u32"),
    (egui_states_value_create_u64, egui_states_value_get_u64, u64, U64, "u64"),
    (egui_states_value_create_i8, egui_states_value_get_i8, i8, I8, "i8"),
    (egui_states_value_create_i16, egui_states_value_get_i16, i16, I16, "i16"),
    (egui_states_value_create_i32, egui_states_value_get_i32, i32, I32, "i32"),
    (egui_states_value_create_i64, egui_states_value_get_i64, i64, I64, "i64"),
    (egui_states_value_create_f32, egui_states_value_get_f32, f32, F32, "f32"),
    (egui_states_value_create_f64, egui_states_value_get_f64, f64, F64, "f64"),
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_value_create_bool(
    value: u8,
    output: *mut *mut egui_states_value_t,
) -> egui_states_status_t {
    ffi_call(|| {
        let value = bool_arg(value, "value")?;
        unsafe { set_value_box(output, DynamicValue::Bool(value)) }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_value_get_bool(
    value: *const egui_states_value_t,
    output: *mut u8,
) -> egui_states_status_t {
    ffi_call(|| {
        let value = unsafe { required_ref(value, "value")? };
        let output = unsafe { required_mut(output, "out_value")? };
        match value.inner {
            DynamicValue::Bool(value) => {
                *output = u8::from(value);
                Ok(())
            }
            ref other => Err(type_mismatch("bool", other)),
        }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_value_create_string(
    value: egui_states_string_view_t,
    output: *mut *mut egui_states_value_t,
) -> egui_states_status_t {
    ffi_call(|| {
        let value = unsafe { string_view(value, "value")? }.to_owned();
        unsafe { set_value_box(output, DynamicValue::String(value)) }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_value_get_string(
    value: *const egui_states_value_t,
    output: *mut egui_states_string_view_t,
) -> egui_states_status_t {
    ffi_call(|| {
        let value = unsafe { required_ref(value, "value")? };
        let output = unsafe { required_mut(output, "out_value")? };
        match &value.inner {
            DynamicValue::String(value) => {
                *output = egui_states_string_view_t {
                    data: value.as_ptr().cast::<c_char>(),
                    length: value.len(),
                };
                Ok(())
            }
            other => Err(type_mismatch("string", other)),
        }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_value_create_enum(
    ordinal: u32,
    output: *mut *mut egui_states_value_t,
) -> egui_states_status_t {
    ffi_call(|| unsafe { set_value_box(output, DynamicValue::Enum(ordinal)) })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_value_get_enum(
    value: *const egui_states_value_t,
    output: *mut u32,
) -> egui_states_status_t {
    ffi_call(|| {
        let value = unsafe { required_ref(value, "value")? };
        let output = unsafe { required_mut(output, "out_ordinal")? };
        match value.inner {
            DynamicValue::Enum(value) => {
                *output = value;
                Ok(())
            }
            ref other => Err(type_mismatch("enum", other)),
        }
    })
}

unsafe fn values(
    values: *const *const egui_states_value_t,
    count: usize,
    name: &str,
) -> Result<Vec<DynamicValue>, Error> {
    unsafe { input_slice(values, count, name)? }
        .iter()
        .enumerate()
        .map(|(index, value)| {
            Ok(
                unsafe { required_ref(*value, &format!("{name}[{index}]"))? }
                    .inner
                    .clone(),
            )
        })
        .collect()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_value_create_sequence(
    elements: *const *const egui_states_value_t,
    count: usize,
    output: *mut *mut egui_states_value_t,
) -> egui_states_status_t {
    ffi_call(|| {
        let values = unsafe { values(elements, count, "elements")? };
        unsafe { set_value_box(output, DynamicValue::Sequence(values)) }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_value_create_map(
    keys: *const *const egui_states_value_t,
    values_pointer: *const *const egui_states_value_t,
    count: usize,
    output: *mut *mut egui_states_value_t,
) -> egui_states_status_t {
    ffi_call(|| {
        let keys = unsafe { values(keys, count, "keys")? };
        let values = unsafe { values(values_pointer, count, "values")? };
        unsafe {
            set_value_box(
                output,
                DynamicValue::Map(keys.into_iter().zip(values).collect()),
            )
        }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_value_create_none(
    output: *mut *mut egui_states_value_t,
) -> egui_states_status_t {
    ffi_call(|| unsafe { set_value_box(output, DynamicValue::Option(None)) })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_value_create_some(
    value: *const egui_states_value_t,
    output: *mut *mut egui_states_value_t,
) -> egui_states_status_t {
    ffi_call(|| {
        let value = unsafe { required_ref(value, "value")? };
        unsafe {
            set_value_box(
                output,
                DynamicValue::Option(Some(Box::new(value.inner.clone()))),
            )
        }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_value_create_empty(
    output: *mut *mut egui_states_value_t,
) -> egui_states_status_t {
    ffi_call(|| unsafe { set_value_box(output, DynamicValue::Empty) })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_value_clone(
    value: *const egui_states_value_t,
    output: *mut *mut egui_states_value_t,
) -> egui_states_status_t {
    ffi_call(|| {
        let value = unsafe { required_ref(value, "value")? };
        unsafe { set_value_box(output, value.inner.clone()) }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_value_kind(
    value: *const egui_states_value_t,
    output: *mut egui_states_value_kind_t,
) -> egui_states_status_t {
    ffi_call(|| {
        let value = unsafe { required_ref(value, "value")? };
        let output = unsafe { required_mut(output, "out_kind")? };
        *output = match value.inner {
            DynamicValue::U8(_) => egui_states_value_kind_t::EGUI_STATES_VALUE_U8,
            DynamicValue::U16(_) => egui_states_value_kind_t::EGUI_STATES_VALUE_U16,
            DynamicValue::U32(_) => egui_states_value_kind_t::EGUI_STATES_VALUE_U32,
            DynamicValue::U64(_) => egui_states_value_kind_t::EGUI_STATES_VALUE_U64,
            DynamicValue::I8(_) => egui_states_value_kind_t::EGUI_STATES_VALUE_I8,
            DynamicValue::I16(_) => egui_states_value_kind_t::EGUI_STATES_VALUE_I16,
            DynamicValue::I32(_) => egui_states_value_kind_t::EGUI_STATES_VALUE_I32,
            DynamicValue::I64(_) => egui_states_value_kind_t::EGUI_STATES_VALUE_I64,
            DynamicValue::F32(_) => egui_states_value_kind_t::EGUI_STATES_VALUE_F32,
            DynamicValue::F64(_) => egui_states_value_kind_t::EGUI_STATES_VALUE_F64,
            DynamicValue::Bool(_) => egui_states_value_kind_t::EGUI_STATES_VALUE_BOOL,
            DynamicValue::String(_) => egui_states_value_kind_t::EGUI_STATES_VALUE_STRING,
            DynamicValue::Enum(_) => egui_states_value_kind_t::EGUI_STATES_VALUE_ENUM,
            DynamicValue::Sequence(_) => egui_states_value_kind_t::EGUI_STATES_VALUE_SEQUENCE,
            DynamicValue::Map(_) => egui_states_value_kind_t::EGUI_STATES_VALUE_MAP,
            DynamicValue::Option(_) => egui_states_value_kind_t::EGUI_STATES_VALUE_OPTION,
            DynamicValue::Empty => egui_states_value_kind_t::EGUI_STATES_VALUE_EMPTY,
        };
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_value_sequence_len(
    value: *const egui_states_value_t,
    output: *mut usize,
) -> egui_states_status_t {
    ffi_call(|| {
        let value = unsafe { required_ref(value, "value")? };
        let output = unsafe { required_mut(output, "out_length")? };
        match &value.inner {
            DynamicValue::Sequence(values) => {
                *output = values.len();
                Ok(())
            }
            other => Err(type_mismatch("sequence", other)),
        }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_value_sequence_get(
    value: *const egui_states_value_t,
    index: usize,
    output: *mut *const egui_states_value_t,
) -> egui_states_status_t {
    ffi_call(|| {
        let value = unsafe { required_ref(value, "value")? };
        let output = unsafe { required_mut(output, "out_value")? };
        match &value.inner {
            DynamicValue::Sequence(values) => {
                let value = values.get(index).ok_or_else(|| {
                    Error::new(
                        ErrorKind::OutOfRange,
                        format!("sequence index {index} is out of range"),
                    )
                })?;
                *output = borrowed_value(value);
                Ok(())
            }
            other => Err(type_mismatch("sequence", other)),
        }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_value_map_len(
    value: *const egui_states_value_t,
    output: *mut usize,
) -> egui_states_status_t {
    ffi_call(|| {
        let value = unsafe { required_ref(value, "value")? };
        let output = unsafe { required_mut(output, "out_length")? };
        match &value.inner {
            DynamicValue::Map(values) => {
                *output = values.len();
                Ok(())
            }
            other => Err(type_mismatch("map", other)),
        }
    })
}

unsafe fn map_entry<'a>(
    value: *const egui_states_value_t,
    index: usize,
) -> Result<&'a (DynamicValue, DynamicValue), Error> {
    let value = unsafe { required_ref(value, "value")? };
    match &value.inner {
        DynamicValue::Map(values) => values.get(index).ok_or_else(|| {
            Error::new(
                ErrorKind::OutOfRange,
                format!("map index {index} is out of range"),
            )
        }),
        other => Err(type_mismatch("map", other)),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_value_map_key(
    value: *const egui_states_value_t,
    index: usize,
    output: *mut *const egui_states_value_t,
) -> egui_states_status_t {
    ffi_call(|| {
        let entry = unsafe { map_entry(value, index)? };
        *unsafe { required_mut(output, "out_key")? } = borrowed_value(&entry.0);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_value_map_value(
    value: *const egui_states_value_t,
    index: usize,
    output: *mut *const egui_states_value_t,
) -> egui_states_status_t {
    ffi_call(|| {
        let entry = unsafe { map_entry(value, index)? };
        *unsafe { required_mut(output, "out_value")? } = borrowed_value(&entry.1);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_value_option_has_value(
    value: *const egui_states_value_t,
    output: *mut u8,
) -> egui_states_status_t {
    ffi_call(|| {
        let value = unsafe { required_ref(value, "value")? };
        let output = unsafe { required_mut(output, "out_has_value")? };
        match &value.inner {
            DynamicValue::Option(value) => {
                *output = u8::from(value.is_some());
                Ok(())
            }
            other => Err(type_mismatch("option", other)),
        }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_value_option_get(
    value: *const egui_states_value_t,
    output: *mut *const egui_states_value_t,
) -> egui_states_status_t {
    ffi_call(|| {
        let value = unsafe { required_ref(value, "value")? };
        let output = unsafe { required_mut(output, "out_value")? };
        match &value.inner {
            DynamicValue::Option(Some(value)) => {
                *output = borrowed_value(value);
                Ok(())
            }
            DynamicValue::Option(None) => Err(Error::new(ErrorKind::NotFound, "option is none")),
            other => Err(type_mismatch("option", other)),
        }
    })
}
