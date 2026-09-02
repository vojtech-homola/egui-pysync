use std::ffi::{c_char, c_void};
use std::time::Duration;

use egui_states::c_api::{DynamicServer, Error, ErrorKind};

use crate::abi::{
    bool_arg, borrowed_value, copy_bytes, copy_u32s, data_type, data_view, egui_states_data_view_t,
    egui_states_image_view_t, egui_states_object_type_t, egui_states_server_t,
    egui_states_signal_event_t, egui_states_status_t, egui_states_string_view_t,
    egui_states_value_t, ffi_call, image_view, input_slice, invalid, required_mut, required_ref,
    set_box, set_value_box, string_view,
};

unsafe fn server<'a>(pointer: *const egui_states_server_t) -> Result<&'a DynamicServer, Error> {
    Ok(&unsafe { required_ref(pointer, "server")? }.inner)
}

unsafe fn object_type<'a>(
    pointer: *const egui_states_object_type_t,
) -> Result<&'a egui_states::ObjectType, Error> {
    Ok(&unsafe { required_ref(pointer, "object_type")? }.inner)
}

unsafe fn value<'a>(
    pointer: *const egui_states_value_t,
    name: &str,
) -> Result<&'a egui_states::c_api::DynamicValue, Error> {
    Ok(&unsafe { required_ref(pointer, name)? }.inner)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_create(
    version: *const u64,
    output: *mut *mut egui_states_server_t,
) -> egui_states_status_t {
    ffi_call(|| {
        let version = if version.is_null() {
            None
        } else {
            Some(*unsafe { required_ref(version, "version")? })
        };
        unsafe {
            set_box(
                output,
                egui_states_server_t {
                    inner: DynamicServer::new(version),
                },
                "out_server",
            )
        }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_finalize(
    server_pointer: *const egui_states_server_t,
) -> egui_states_status_t {
    ffi_call(|| unsafe { server(server_pointer)? }.finalize())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_start(
    server_pointer: *const egui_states_server_t,
    port: u16,
    ip_address: *const u8,
    token: *const egui_states_string_view_t,
) -> egui_states_status_t {
    ffi_call(|| {
        let server = unsafe { server(server_pointer)? };
        let ip = if ip_address.is_null() {
            None
        } else {
            let bytes = unsafe { input_slice(ip_address, 4, "ip_address")? };
            Some([bytes[0], bytes[1], bytes[2], bytes[3]])
        };
        let token = if token.is_null() {
            None
        } else {
            let token = unsafe { required_ref(token, "token")? };
            let token = unsafe { string_view(*token, "token")? };
            Some(token.to_owned())
        };
        server.start(port, ip, token)
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_stop(
    server_pointer: *const egui_states_server_t,
) -> egui_states_status_t {
    ffi_call(|| {
        unsafe { server(server_pointer)? }.stop();
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_disconnect_client(
    server_pointer: *const egui_states_server_t,
) -> egui_states_status_t {
    ffi_call(|| {
        unsafe { server(server_pointer)? }.disconnect_client();
        Ok(())
    })
}

macro_rules! server_bool_query {
    ($name:ident, $method:ident) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(
            server_pointer: *const egui_states_server_t,
            output: *mut u8,
        ) -> egui_states_status_t {
            ffi_call(|| {
                let value = unsafe { server(server_pointer)? }.$method();
                *unsafe { required_mut(output, "out_value")? } = u8::from(value);
                Ok(())
            })
        }
    };
}

server_bool_query!(egui_states_server_is_running, is_running);
server_bool_query!(egui_states_server_is_connected, is_connected);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_update(
    server_pointer: *const egui_states_server_t,
    duration_seconds: *const f32,
) -> egui_states_status_t {
    ffi_call(|| {
        let duration = if duration_seconds.is_null() {
            None
        } else {
            let duration = *unsafe { required_ref(duration_seconds, "duration_seconds")? };
            if !duration.is_finite() || duration < 0.0 {
                return Err(invalid("duration_seconds must be finite and non-negative"));
            }
            Some(duration)
        };
        unsafe { server(server_pointer)? }.update(duration)
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_id_to_name(
    server_pointer: *const egui_states_server_t,
    id: u64,
    destination: *mut c_char,
    capacity: usize,
    required: *mut usize,
) -> egui_states_status_t {
    ffi_call(|| {
        let name = unsafe { server(server_pointer)? }.id_to_name(id)?;
        unsafe {
            copy_bytes(
                name.as_bytes(),
                destination.cast::<u8>(),
                capacity,
                required,
            )
        }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_add_value(
    server_pointer: *const egui_states_server_t,
    name: egui_states_string_view_t,
    type_pointer: *const egui_states_object_type_t,
    initial: *const egui_states_value_t,
    queue: u8,
    output: *mut u64,
) -> egui_states_status_t {
    ffi_call(|| {
        let id = unsafe { server(server_pointer)? }.add_value(
            unsafe { string_view(name, "name")? },
            unsafe { object_type(type_pointer)? },
            unsafe { value(initial, "initial")? },
            bool_arg(queue, "queue")?,
        )?;
        *unsafe { required_mut(output, "out_id")? } = id;
        Ok(())
    })
}

macro_rules! add_typed_state {
    ($name:ident, $method:ident) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(
            server_pointer: *const egui_states_server_t,
            name: egui_states_string_view_t,
            type_pointer: *const egui_states_object_type_t,
            output: *mut u64,
        ) -> egui_states_status_t {
            ffi_call(|| {
                let id = unsafe { server(server_pointer)? }
                    .$method(unsafe { string_view(name, "name")? }, unsafe {
                        object_type(type_pointer)?
                    })?;
                *unsafe { required_mut(output, "out_id")? } = id;
                Ok(())
            })
        }
    };
}

add_typed_state!(egui_states_server_add_value_take, add_value_take);
add_typed_state!(egui_states_server_add_list, add_list);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_add_static(
    server_pointer: *const egui_states_server_t,
    name: egui_states_string_view_t,
    type_pointer: *const egui_states_object_type_t,
    initial: *const egui_states_value_t,
    output: *mut u64,
) -> egui_states_status_t {
    ffi_call(|| {
        let id = unsafe { server(server_pointer)? }.add_static(
            unsafe { string_view(name, "name")? },
            unsafe { object_type(type_pointer)? },
            unsafe { value(initial, "initial")? },
        )?;
        *unsafe { required_mut(output, "out_id")? } = id;
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_add_signal(
    server_pointer: *const egui_states_server_t,
    name: egui_states_string_view_t,
    type_pointer: *const egui_states_object_type_t,
    queue: u8,
    output: *mut u64,
) -> egui_states_status_t {
    ffi_call(|| {
        let id = unsafe { server(server_pointer)? }.add_signal(
            unsafe { string_view(name, "name")? },
            unsafe { object_type(type_pointer)? },
            bool_arg(queue, "queue")?,
        )?;
        *unsafe { required_mut(output, "out_id")? } = id;
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_add_map(
    server_pointer: *const egui_states_server_t,
    name: egui_states_string_view_t,
    key_type: *const egui_states_object_type_t,
    value_type: *const egui_states_object_type_t,
    output: *mut u64,
) -> egui_states_status_t {
    ffi_call(|| {
        let id = unsafe { server(server_pointer)? }.add_map(
            unsafe { string_view(name, "name")? },
            unsafe { object_type(key_type)? },
            unsafe { object_type(value_type)? },
        )?;
        *unsafe { required_mut(output, "out_id")? } = id;
        Ok(())
    })
}

macro_rules! add_name_only_state {
    ($name:ident, $method:ident) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(
            server_pointer: *const egui_states_server_t,
            name: egui_states_string_view_t,
            output: *mut u64,
        ) -> egui_states_status_t {
            ffi_call(|| {
                let id = unsafe { server(server_pointer)? }
                    .$method(unsafe { string_view(name, "name")? })?;
                *unsafe { required_mut(output, "out_id")? } = id;
                Ok(())
            })
        }
    };
}

add_name_only_state!(egui_states_server_add_image, add_image);
add_name_only_state!(egui_states_server_add_image_multi, add_image_multi);

macro_rules! add_data_state {
    ($name:ident, $method:ident) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(
            server_pointer: *const egui_states_server_t,
            name: egui_states_string_view_t,
            data_type_value: u32,
            output: *mut u64,
        ) -> egui_states_status_t {
            ffi_call(|| {
                let id = unsafe { server(server_pointer)? }.$method(
                    unsafe { string_view(name, "name")? },
                    data_type(data_type_value)?,
                )?;
                *unsafe { required_mut(output, "out_id")? } = id;
                Ok(())
            })
        }
    };
}

add_data_state!(egui_states_server_add_data, add_data);
add_data_state!(egui_states_server_add_data_take, add_data_take);
add_data_state!(egui_states_server_add_data_multi, add_data_multi);
add_data_state!(egui_states_server_add_data_multi_take, add_data_multi_take);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_value_get(
    server_pointer: *const egui_states_server_t,
    id: u64,
    output: *mut *mut egui_states_value_t,
) -> egui_states_status_t {
    ffi_call(|| {
        let value = unsafe { server(server_pointer)? }.value_get(id)?;
        unsafe { set_value_box(output, value) }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_value_set(
    server_pointer: *const egui_states_server_t,
    id: u64,
    value_pointer: *const egui_states_value_t,
    set_signal: u8,
    update: u8,
) -> egui_states_status_t {
    ffi_call(|| {
        unsafe { server(server_pointer)? }.value_set(
            id,
            unsafe { value(value_pointer, "value")? },
            bool_arg(set_signal, "set_signal")?,
            bool_arg(update, "update")?,
        )
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_value_take_set(
    server_pointer: *const egui_states_server_t,
    id: u64,
    value_pointer: *const egui_states_value_t,
    blocking: u8,
    update: u8,
) -> egui_states_status_t {
    ffi_call(|| {
        unsafe { server(server_pointer)? }.value_take_set(
            id,
            unsafe { value(value_pointer, "value")? },
            bool_arg(blocking, "blocking")?,
            bool_arg(update, "update")?,
        )
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_static_get(
    server_pointer: *const egui_states_server_t,
    id: u64,
    output: *mut *mut egui_states_value_t,
) -> egui_states_status_t {
    ffi_call(|| {
        let value = unsafe { server(server_pointer)? }.static_get(id)?;
        unsafe { set_value_box(output, value) }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_static_set(
    server_pointer: *const egui_states_server_t,
    id: u64,
    value_pointer: *const egui_states_value_t,
    update: u8,
) -> egui_states_status_t {
    ffi_call(|| {
        unsafe { server(server_pointer)? }.static_set(
            id,
            unsafe { value(value_pointer, "value")? },
            bool_arg(update, "update")?,
        )
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_signal_set(
    server_pointer: *const egui_states_server_t,
    id: u64,
    value_pointer: *const egui_states_value_t,
) -> egui_states_status_t {
    ffi_call(|| {
        unsafe { server(server_pointer)? }.signal_set(id, unsafe { value(value_pointer, "value")? })
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_signal_register(
    server_pointer: *const egui_states_server_t,
    id: u64,
    register_signal: u8,
    with_previous: u8,
) -> egui_states_status_t {
    ffi_call(|| {
        unsafe { server(server_pointer)? }.signal_register(
            id,
            bool_arg(register_signal, "register_signal")?,
            bool_arg(with_previous, "with_previous")?,
        )
    })
}

macro_rules! signal_mode {
    ($name:ident, $method:ident) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(
            server_pointer: *const egui_states_server_t,
            id: u64,
        ) -> egui_states_status_t {
            ffi_call(|| unsafe { server(server_pointer)? }.$method(id))
        }
    };
}

signal_mode!(egui_states_server_signal_set_to_queue, signal_set_to_queue);
signal_mode!(
    egui_states_server_signal_set_to_single,
    signal_set_to_single
);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_signal_wait(
    server_pointer: *const egui_states_server_t,
    timeout_milliseconds: u64,
    output: *mut *mut egui_states_signal_event_t,
) -> egui_states_status_t {
    ffi_call(|| {
        let timeout = if timeout_milliseconds == u64::MAX {
            None
        } else {
            Some(Duration::from_millis(timeout_milliseconds))
        };
        let event = unsafe { server(server_pointer)? }.signal_wait(timeout)?;
        unsafe {
            set_box(
                output,
                egui_states_signal_event_t { inner: event },
                "out_event",
            )
        }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_signal_event_id(
    event: *const egui_states_signal_event_t,
    output: *mut u64,
) -> egui_states_status_t {
    ffi_call(|| {
        let event = unsafe { required_ref(event, "event")? };
        *unsafe { required_mut(output, "out_id")? } = event.inner.id();
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_signal_event_value(
    event: *const egui_states_signal_event_t,
    output: *mut *const egui_states_value_t,
) -> egui_states_status_t {
    ffi_call(|| {
        let event = unsafe { required_ref(event, "event")? };
        *unsafe { required_mut(output, "out_value")? } = borrowed_value(event.inner.value());
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_signal_event_has_previous(
    event: *const egui_states_signal_event_t,
    output: *mut u8,
) -> egui_states_status_t {
    ffi_call(|| {
        let event = unsafe { required_ref(event, "event")? };
        *unsafe { required_mut(output, "out_has_previous")? } =
            u8::from(event.inner.previous().is_some());
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_signal_event_previous(
    event: *const egui_states_signal_event_t,
    output: *mut *const egui_states_value_t,
) -> egui_states_status_t {
    ffi_call(|| {
        let event = unsafe { required_ref(event, "event")? };
        let previous = event
            .inner
            .previous()
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "signal event has no previous value"))?;
        *unsafe { required_mut(output, "out_value")? } = borrowed_value(previous);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_list_set(
    server_pointer: *const egui_states_server_t,
    id: u64,
    list: *const egui_states_value_t,
    update: u8,
) -> egui_states_status_t {
    ffi_call(|| {
        unsafe { server(server_pointer)? }.list_set(
            id,
            unsafe { value(list, "list")? },
            bool_arg(update, "update")?,
        )
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_list_get(
    server_pointer: *const egui_states_server_t,
    id: u64,
    output: *mut *mut egui_states_value_t,
) -> egui_states_status_t {
    ffi_call(|| {
        let value = unsafe { server(server_pointer)? }.list_get(id)?;
        unsafe { set_value_box(output, value) }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_list_set_item(
    server_pointer: *const egui_states_server_t,
    id: u64,
    index: usize,
    item: *const egui_states_value_t,
    update: u8,
) -> egui_states_status_t {
    ffi_call(|| {
        unsafe { server(server_pointer)? }.list_set_item(
            id,
            index,
            unsafe { value(item, "item")? },
            bool_arg(update, "update")?,
        )
    })
}

macro_rules! list_item_output {
    ($name:ident, $method:ident $(, $update:ident)?) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(
            server_pointer: *const egui_states_server_t,
            id: u64,
            index: usize,
            $( $update: u8, )?
            output: *mut *mut egui_states_value_t,
        ) -> egui_states_status_t {
            ffi_call(|| {
                let result = unsafe { server(server_pointer)? }
                    .$method(id, index $(, bool_arg($update, "update")? )?)?;
                unsafe { set_value_box(output, result) }
            })
        }
    };
}

list_item_output!(egui_states_server_list_get_item, list_get_item);
list_item_output!(
    egui_states_server_list_remove_item,
    list_remove_item,
    update
);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_list_append_item(
    server_pointer: *const egui_states_server_t,
    id: u64,
    item: *const egui_states_value_t,
    update: u8,
) -> egui_states_status_t {
    ffi_call(|| {
        unsafe { server(server_pointer)? }.list_append_item(
            id,
            unsafe { value(item, "item")? },
            bool_arg(update, "update")?,
        )
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_list_len(
    server_pointer: *const egui_states_server_t,
    id: u64,
    output: *mut usize,
) -> egui_states_status_t {
    ffi_call(|| {
        *unsafe { required_mut(output, "out_length")? } =
            unsafe { server(server_pointer)? }.list_len(id)?;
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_map_set(
    server_pointer: *const egui_states_server_t,
    id: u64,
    map: *const egui_states_value_t,
    update: u8,
) -> egui_states_status_t {
    ffi_call(|| {
        unsafe { server(server_pointer)? }.map_set(
            id,
            unsafe { value(map, "map")? },
            bool_arg(update, "update")?,
        )
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_map_get(
    server_pointer: *const egui_states_server_t,
    id: u64,
    output: *mut *mut egui_states_value_t,
) -> egui_states_status_t {
    ffi_call(|| {
        let value = unsafe { server(server_pointer)? }.map_get(id)?;
        unsafe { set_value_box(output, value) }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_map_set_item(
    server_pointer: *const egui_states_server_t,
    id: u64,
    key: *const egui_states_value_t,
    item: *const egui_states_value_t,
    update: u8,
) -> egui_states_status_t {
    ffi_call(|| {
        unsafe { server(server_pointer)? }.map_set_item(
            id,
            unsafe { value(key, "key")? },
            unsafe { value(item, "item")? },
            bool_arg(update, "update")?,
        )
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_map_get_item(
    server_pointer: *const egui_states_server_t,
    id: u64,
    key: *const egui_states_value_t,
    output: *mut *mut egui_states_value_t,
) -> egui_states_status_t {
    ffi_call(|| {
        let result =
            unsafe { server(server_pointer)? }.map_get_item(id, unsafe { value(key, "key")? })?;
        unsafe { set_value_box(output, result) }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_map_remove_item(
    server_pointer: *const egui_states_server_t,
    id: u64,
    key: *const egui_states_value_t,
    update: u8,
    output: *mut *mut egui_states_value_t,
) -> egui_states_status_t {
    ffi_call(|| {
        let result = unsafe { server(server_pointer)? }.map_remove_item(
            id,
            unsafe { value(key, "key")? },
            bool_arg(update, "update")?,
        )?;
        unsafe { set_value_box(output, result) }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_map_len(
    server_pointer: *const egui_states_server_t,
    id: u64,
    output: *mut usize,
) -> egui_states_status_t {
    ffi_call(|| {
        *unsafe { required_mut(output, "out_length")? } =
            unsafe { server(server_pointer)? }.map_len(id)?;
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_image_size(
    server_pointer: *const egui_states_server_t,
    id: u64,
    out_height: *mut usize,
    out_width: *mut usize,
) -> egui_states_status_t {
    ffi_call(|| {
        let [height, width] = unsafe { server(server_pointer)? }.image_size(id)?;
        *unsafe { required_mut(out_height, "out_height")? } = height;
        *unsafe { required_mut(out_width, "out_width")? } = width;
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_image_get(
    server_pointer: *const egui_states_server_t,
    id: u64,
    destination: *mut u8,
    capacity: usize,
    required: *mut usize,
    out_height: *mut usize,
    out_width: *mut usize,
) -> egui_states_status_t {
    ffi_call(|| {
        let (image, [height, width]) = unsafe { server(server_pointer)? }.image_get(id)?;
        *unsafe { required_mut(out_height, "out_height")? } = height;
        *unsafe { required_mut(out_width, "out_width")? } = width;
        unsafe { copy_bytes(&image, destination, capacity, required) }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_image_set(
    server_pointer: *const egui_states_server_t,
    id: u64,
    image: egui_states_image_view_t,
    update: u8,
) -> egui_states_status_t {
    ffi_call(|| {
        let image = unsafe { image_view(image)? };
        unsafe { server(server_pointer)? }.image_set(id, &image, bool_arg(update, "update")?)
    })
}

unsafe fn rgba(pointer: *const u8) -> Result<[u8; 4], Error> {
    let rgba = unsafe { input_slice(pointer, 4, "rgba")? };
    Ok([rgba[0], rgba[1], rgba[2], rgba[3]])
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_image_set_all(
    server_pointer: *const egui_states_server_t,
    id: u64,
    height: usize,
    width: usize,
    rgba_pointer: *const u8,
    update: u8,
) -> egui_states_status_t {
    ffi_call(|| {
        unsafe { server(server_pointer)? }.image_set_all(
            id,
            [height, width],
            unsafe { rgba(rgba_pointer)? },
            bool_arg(update, "update")?,
        )
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_image_update(
    server_pointer: *const egui_states_server_t,
    id: u64,
    image: egui_states_image_view_t,
    origin_y: usize,
    origin_x: usize,
    update: u8,
    force: u8,
) -> egui_states_status_t {
    ffi_call(|| {
        let image = unsafe { image_view(image)? };
        unsafe { server(server_pointer)? }.image_update(
            id,
            [origin_y, origin_x],
            &image,
            bool_arg(update, "update")?,
            bool_arg(force, "force")?,
        )
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_image_multi_size(
    server_pointer: *const egui_states_server_t,
    id: u64,
    index: u32,
    out_height: *mut usize,
    out_width: *mut usize,
) -> egui_states_status_t {
    ffi_call(|| {
        let [height, width] = unsafe { server(server_pointer)? }.image_multi_size(id, index)?;
        *unsafe { required_mut(out_height, "out_height")? } = height;
        *unsafe { required_mut(out_width, "out_width")? } = width;
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_image_multi_get(
    server_pointer: *const egui_states_server_t,
    id: u64,
    index: u32,
    destination: *mut u8,
    capacity: usize,
    required: *mut usize,
    out_height: *mut usize,
    out_width: *mut usize,
) -> egui_states_status_t {
    ffi_call(|| {
        let (image, [height, width]) =
            unsafe { server(server_pointer)? }.image_multi_get(id, index)?;
        *unsafe { required_mut(out_height, "out_height")? } = height;
        *unsafe { required_mut(out_width, "out_width")? } = width;
        unsafe { copy_bytes(&image, destination, capacity, required) }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_image_multi_set(
    server_pointer: *const egui_states_server_t,
    id: u64,
    index: u32,
    image: egui_states_image_view_t,
    update: u8,
) -> egui_states_status_t {
    ffi_call(|| {
        let image = unsafe { image_view(image)? };
        unsafe { server(server_pointer)? }.image_multi_set(
            id,
            index,
            &image,
            bool_arg(update, "update")?,
        )
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_image_multi_set_all(
    server_pointer: *const egui_states_server_t,
    id: u64,
    index: u32,
    height: usize,
    width: usize,
    rgba_pointer: *const u8,
    update: u8,
) -> egui_states_status_t {
    ffi_call(|| {
        unsafe { server(server_pointer)? }.image_multi_set_all(
            id,
            index,
            [height, width],
            unsafe { rgba(rgba_pointer)? },
            bool_arg(update, "update")?,
        )
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_image_multi_update(
    server_pointer: *const egui_states_server_t,
    id: u64,
    index: u32,
    image: egui_states_image_view_t,
    origin_y: usize,
    origin_x: usize,
    update: u8,
    force: u8,
) -> egui_states_status_t {
    ffi_call(|| {
        let image = unsafe { image_view(image)? };
        unsafe { server(server_pointer)? }.image_multi_update(
            id,
            index,
            [origin_y, origin_x],
            &image,
            bool_arg(update, "update")?,
            bool_arg(force, "force")?,
        )
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_image_multi_remove_index(
    server_pointer: *const egui_states_server_t,
    id: u64,
    index: u32,
    update: u8,
) -> egui_states_status_t {
    ffi_call(|| {
        unsafe { server(server_pointer)? }.image_multi_remove_index(
            id,
            index,
            bool_arg(update, "update")?,
        )
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_image_multi_reset(
    server_pointer: *const egui_states_server_t,
    id: u64,
    update: u8,
) -> egui_states_status_t {
    ffi_call(|| {
        unsafe { server(server_pointer)? }.image_multi_reset(id, bool_arg(update, "update")?)
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_image_multi_len(
    server_pointer: *const egui_states_server_t,
    id: u64,
    output: *mut usize,
) -> egui_states_status_t {
    ffi_call(|| {
        *unsafe { required_mut(output, "out_length")? } =
            unsafe { server(server_pointer)? }.image_multi_len(id)?;
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_image_multi_contains(
    server_pointer: *const egui_states_server_t,
    id: u64,
    index: u32,
    output: *mut u8,
) -> egui_states_status_t {
    ffi_call(|| {
        *unsafe { required_mut(output, "out_contains")? } =
            u8::from(unsafe { server(server_pointer)? }.image_multi_contains(id, index)?);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_image_multi_indices(
    server_pointer: *const egui_states_server_t,
    id: u64,
    destination: *mut u32,
    capacity: usize,
    required: *mut usize,
) -> egui_states_status_t {
    ffi_call(|| {
        let indices = unsafe { server(server_pointer)? }.image_multi_indices(id)?;
        unsafe { copy_u32s(&indices, destination, capacity, required) }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_data_get(
    server_pointer: *const egui_states_server_t,
    id: u64,
    destination: *mut c_void,
    capacity: usize,
    required: *mut usize,
) -> egui_states_status_t {
    ffi_call(|| {
        let bytes = unsafe { server(server_pointer)? }.data_get(id)?;
        unsafe { copy_bytes(&bytes, destination.cast::<u8>(), capacity, required) }
    })
}

macro_rules! data_input_operation {
    ($name:ident, $method:ident $(, $extra_name:ident : $extra_type:ty)*) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(
            server_pointer: *const egui_states_server_t,
            id: u64,
            data: egui_states_data_view_t,
            $( $extra_name: $extra_type, )*
            update: u8,
        ) -> egui_states_status_t {
            ffi_call(|| {
                let bytes = unsafe { data_view(data)? };
                unsafe { server(server_pointer)? }.$method(
                    id,
                    bytes,
                    data.element_count,
                    $( $extra_name, )*
                    bool_arg(update, "update")?,
                )
            })
        }
    };
}

data_input_operation!(egui_states_server_data_set, data_set);
data_input_operation!(egui_states_server_data_add, data_add);
data_input_operation!(egui_states_server_data_replace, data_replace, index: usize);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_data_remove(
    server_pointer: *const egui_states_server_t,
    id: u64,
    index: usize,
    count: usize,
    update: u8,
) -> egui_states_status_t {
    ffi_call(|| {
        unsafe { server(server_pointer)? }.data_remove(
            id,
            index,
            count,
            bool_arg(update, "update")?,
        )
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_data_clear(
    server_pointer: *const egui_states_server_t,
    id: u64,
    update: u8,
) -> egui_states_status_t {
    ffi_call(|| unsafe { server(server_pointer)? }.data_clear(id, bool_arg(update, "update")?))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_data_take_set(
    server_pointer: *const egui_states_server_t,
    id: u64,
    data: egui_states_data_view_t,
    blocking: u8,
    update: u8,
    cache: u8,
) -> egui_states_status_t {
    ffi_call(|| {
        let bytes = unsafe { data_view(data)? };
        unsafe { server(server_pointer)? }.data_take_set(
            id,
            bytes,
            data.element_count,
            bool_arg(blocking, "blocking")?,
            bool_arg(update, "update")?,
            bool_arg(cache, "cache")?,
        )
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_data_multi_get(
    server_pointer: *const egui_states_server_t,
    id: u64,
    index: u32,
    destination: *mut c_void,
    capacity: usize,
    required: *mut usize,
) -> egui_states_status_t {
    ffi_call(|| {
        let bytes = unsafe { server(server_pointer)? }.data_multi_get(id, index)?;
        unsafe { copy_bytes(&bytes, destination.cast::<u8>(), capacity, required) }
    })
}

macro_rules! data_multi_input_operation {
    ($name:ident, $method:ident $(, $extra_name:ident : $extra_type:ty)*) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(
            server_pointer: *const egui_states_server_t,
            id: u64,
            index: u32,
            data: egui_states_data_view_t,
            $( $extra_name: $extra_type, )*
            update: u8,
        ) -> egui_states_status_t {
            ffi_call(|| {
                let bytes = unsafe { data_view(data)? };
                unsafe { server(server_pointer)? }.$method(
                    id,
                    index,
                    bytes,
                    data.element_count,
                    $( $extra_name, )*
                    bool_arg(update, "update")?,
                )
            })
        }
    };
}

data_multi_input_operation!(egui_states_server_data_multi_set, data_multi_set);
data_multi_input_operation!(egui_states_server_data_multi_add, data_multi_add);
data_multi_input_operation!(
    egui_states_server_data_multi_replace,
    data_multi_replace,
    data_index: usize
);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_data_multi_remove(
    server_pointer: *const egui_states_server_t,
    id: u64,
    index: u32,
    data_index: usize,
    count: usize,
    update: u8,
) -> egui_states_status_t {
    ffi_call(|| {
        unsafe { server(server_pointer)? }.data_multi_remove(
            id,
            index,
            data_index,
            count,
            bool_arg(update, "update")?,
        )
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_data_multi_clear(
    server_pointer: *const egui_states_server_t,
    id: u64,
    index: u32,
    update: u8,
) -> egui_states_status_t {
    ffi_call(|| {
        unsafe { server(server_pointer)? }.data_multi_clear(id, index, bool_arg(update, "update")?)
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_data_multi_remove_index(
    server_pointer: *const egui_states_server_t,
    id: u64,
    index: u32,
    update: u8,
) -> egui_states_status_t {
    ffi_call(|| {
        unsafe { server(server_pointer)? }.data_multi_remove_index(
            id,
            index,
            bool_arg(update, "update")?,
        )
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_data_multi_reset(
    server_pointer: *const egui_states_server_t,
    id: u64,
    update: u8,
) -> egui_states_status_t {
    ffi_call(|| {
        unsafe { server(server_pointer)? }.data_multi_reset(id, bool_arg(update, "update")?)
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_data_multi_take_set(
    server_pointer: *const egui_states_server_t,
    id: u64,
    index: u32,
    data: egui_states_data_view_t,
    blocking: u8,
    update: u8,
    cache: u8,
) -> egui_states_status_t {
    ffi_call(|| {
        let bytes = unsafe { data_view(data)? };
        unsafe { server(server_pointer)? }.data_multi_take_set(
            id,
            index,
            bytes,
            data.element_count,
            bool_arg(blocking, "blocking")?,
            bool_arg(update, "update")?,
            bool_arg(cache, "cache")?,
        )
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_data_multi_take_remove_index(
    server_pointer: *const egui_states_server_t,
    id: u64,
    index: u32,
    update: u8,
) -> egui_states_status_t {
    ffi_call(|| {
        unsafe { server(server_pointer)? }.data_multi_take_remove_index(
            id,
            index,
            bool_arg(update, "update")?,
        )
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn egui_states_server_data_multi_take_reset(
    server_pointer: *const egui_states_server_t,
    id: u64,
    update: u8,
) -> egui_states_status_t {
    ffi_call(|| {
        unsafe { server(server_pointer)? }.data_multi_take_reset(id, bool_arg(update, "update")?)
    })
}
