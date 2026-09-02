//! C ABI for the egui-states dynamic state server.

#![allow(non_camel_case_types)]

mod abi;
mod objects;
mod server;

pub use abi::*;

#[cfg(test)]
mod tests {
    use std::ffi::CStr;
    use std::net::Ipv4Addr;
    use std::path::PathBuf;
    use std::process::Command;
    use std::ptr;
    use std::thread;
    use std::time::Duration;

    use egui_states::{ClientBuilder, State, StatesCreator, Value};

    use super::*;
    use crate::objects::*;
    use crate::server::*;

    fn view(value: &'static str) -> egui_states_string_view_t {
        egui_states_string_view_t {
            data: value.as_ptr().cast(),
            length: value.len(),
        }
    }

    #[test]
    fn ffi_values_deep_copy_and_report_errors() {
        unsafe {
            let mut first = ptr::null_mut();
            let mut second = ptr::null_mut();
            assert_eq!(
                egui_states_value_create_i32(1, &mut first),
                egui_states_status_t::EGUI_STATES_STATUS_OK
            );
            assert_eq!(
                egui_states_value_create_string(view("two"), &mut second),
                egui_states_status_t::EGUI_STATES_STATUS_OK
            );
            let children = [first.cast_const(), second.cast_const()];
            let mut sequence = ptr::null_mut();
            assert_eq!(
                egui_states_value_create_sequence(children.as_ptr(), children.len(), &mut sequence),
                egui_states_status_t::EGUI_STATES_STATUS_OK
            );
            egui_states_value_destroy(first);
            egui_states_value_destroy(second);

            let mut length = 0;
            assert_eq!(
                egui_states_value_sequence_len(sequence, &mut length),
                egui_states_status_t::EGUI_STATES_STATUS_OK
            );
            assert_eq!(length, 2);
            let mut scalar = 0_u8;
            assert_eq!(
                egui_states_value_get_u8(sequence, &mut scalar),
                egui_states_status_t::EGUI_STATES_STATUS_TYPE_MISMATCH
            );
            let mut missing_child = ptr::null();
            assert_eq!(
                egui_states_value_sequence_get(sequence, 2, &mut missing_child),
                egui_states_status_t::EGUI_STATES_STATUS_OUT_OF_RANGE
            );
            let mut child = ptr::null();
            egui_states_value_sequence_get(sequence, 1, &mut child);
            let mut string = egui_states_string_view_t {
                data: ptr::null(),
                length: 0,
            };
            egui_states_value_get_string(child, &mut string);
            assert_eq!(
                std::slice::from_raw_parts(string.data.cast::<u8>(), string.length),
                b"two"
            );

            assert_eq!(
                egui_states_value_get_u8(sequence, ptr::null_mut()),
                egui_states_status_t::EGUI_STATES_STATUS_INVALID_ARGUMENT
            );
            let message = CStr::from_ptr(egui_states_last_error_message())
                .to_str()
                .unwrap();
            assert!(message.contains("out_value") || message.contains("expected"));

            let mut none = ptr::null_mut();
            egui_states_value_create_none(&mut none);
            let mut option_value = ptr::null();
            assert_eq!(
                egui_states_value_option_get(none, &mut option_value),
                egui_states_status_t::EGUI_STATES_STATUS_NOT_FOUND
            );
            egui_states_value_destroy(none);
            egui_states_value_destroy(sequence);
        }
    }

    #[test]
    fn ffi_buffer_queries_and_boundary_validation() {
        unsafe {
            let invalid_utf8 = [0xff_u8];
            let invalid_view = egui_states_string_view_t {
                data: invalid_utf8.as_ptr().cast(),
                length: invalid_utf8.len(),
            };
            let mut invalid_value = ptr::null_mut();
            assert_eq!(
                egui_states_value_create_string(invalid_view, &mut invalid_value),
                egui_states_status_t::EGUI_STATES_STATUS_INVALID_ARGUMENT
            );
            assert!(invalid_value.is_null());
            let main_error = CStr::from_ptr(egui_states_last_error_message())
                .to_string_lossy()
                .into_owned();
            let other_error = thread::spawn(|| {
                let mut value = ptr::null_mut();
                assert_eq!(
                    egui_states_value_create_bool(2, &mut value),
                    egui_states_status_t::EGUI_STATES_STATUS_INVALID_ARGUMENT
                );
                CStr::from_ptr(egui_states_last_error_message())
                    .to_string_lossy()
                    .into_owned()
            })
            .join()
            .unwrap();
            assert_ne!(main_error, other_error);
            assert_eq!(
                CStr::from_ptr(egui_states_last_error_message()).to_string_lossy(),
                main_error
            );

            let mut server = ptr::null_mut();
            assert_eq!(
                egui_states_server_create(ptr::null(), &mut server),
                egui_states_status_t::EGUI_STATES_STATUS_OK
            );
            let mut object_type = ptr::null_mut();
            let mut initial = ptr::null_mut();
            egui_states_object_type_i32(&mut object_type);
            egui_states_value_create_i32(0, &mut initial);
            let mut value_id = 0;
            let mut image_id = 0;
            let mut image_multi_id = 0;
            let mut data_id = 0;
            assert_eq!(
                egui_states_server_add_value(
                    server,
                    view("root.value"),
                    object_type,
                    initial,
                    0,
                    &mut value_id,
                ),
                egui_states_status_t::EGUI_STATES_STATUS_OK
            );
            assert_eq!(
                egui_states_server_add_image(server, view("root.image"), &mut image_id),
                egui_states_status_t::EGUI_STATES_STATUS_OK
            );
            assert_eq!(
                egui_states_server_add_image_multi(
                    server,
                    view("root.images"),
                    &mut image_multi_id,
                ),
                egui_states_status_t::EGUI_STATES_STATUS_OK
            );
            assert_eq!(
                egui_states_server_add_data(
                    server,
                    view("root.data"),
                    egui_states_data_type_t::EGUI_STATES_DATA_U16 as u32,
                    &mut data_id,
                ),
                egui_states_status_t::EGUI_STATES_STATUS_OK
            );
            let mut unavailable = ptr::null_mut();
            assert_eq!(
                egui_states_server_value_get(server, value_id, &mut unavailable),
                egui_states_status_t::EGUI_STATES_STATUS_INVALID_STATE
            );
            egui_states_object_type_destroy(object_type);
            egui_states_value_destroy(initial);
            assert_eq!(
                egui_states_server_finalize(server),
                egui_states_status_t::EGUI_STATES_STATUS_OK
            );
            assert_eq!(
                egui_states_server_value_get(server, u64::MAX, &mut unavailable),
                egui_states_status_t::EGUI_STATES_STATUS_NOT_FOUND
            );
            let mut no_event = ptr::null_mut();
            assert_eq!(
                egui_states_server_signal_wait(server, 0, &mut no_event),
                egui_states_status_t::EGUI_STATES_STATUS_TIMEOUT
            );

            let mut required = 0;
            assert_eq!(
                egui_states_server_id_to_name(server, value_id, ptr::null_mut(), 0, &mut required,),
                egui_states_status_t::EGUI_STATES_STATUS_OK
            );
            assert_eq!(required, 10);
            let mut short_name = [0_i8; 9];
            assert_eq!(
                egui_states_server_id_to_name(
                    server,
                    value_id,
                    short_name.as_mut_ptr(),
                    short_name.len(),
                    &mut required,
                ),
                egui_states_status_t::EGUI_STATES_STATUS_BUFFER_TOO_SMALL
            );
            let mut name = [0_i8; 10];
            assert_eq!(
                egui_states_server_id_to_name(
                    server,
                    value_id,
                    name.as_mut_ptr(),
                    name.len(),
                    &mut required,
                ),
                egui_states_status_t::EGUI_STATES_STATUS_OK
            );
            assert_eq!(
                std::slice::from_raw_parts(name.as_ptr().cast::<u8>(), name.len()),
                b"root.value"
            );

            let gray = [1_u8, 2, 99, 3, 4];
            let image = egui_states_image_view_t {
                data: gray.as_ptr(),
                byte_length: gray.len(),
                height: 2,
                width: 2,
                row_stride: 3,
                format: egui_states_image_format_t::EGUI_STATES_IMAGE_GRAY as u32,
            };
            assert_eq!(
                egui_states_server_image_set(server, image_id, image, 0),
                egui_states_status_t::EGUI_STATES_STATUS_OK
            );
            let mut height = 0;
            let mut width = 0;
            required = 0;
            assert_eq!(
                egui_states_server_image_get(
                    server,
                    image_id,
                    ptr::null_mut(),
                    0,
                    &mut required,
                    &mut height,
                    &mut width,
                ),
                egui_states_status_t::EGUI_STATES_STATUS_OK
            );
            assert_eq!((required, height, width), (16, 2, 2));
            let mut short_image = [0_u8; 15];
            assert_eq!(
                egui_states_server_image_get(
                    server,
                    image_id,
                    short_image.as_mut_ptr(),
                    short_image.len(),
                    &mut required,
                    &mut height,
                    &mut width,
                ),
                egui_states_status_t::EGUI_STATES_STATUS_BUFFER_TOO_SMALL
            );
            let mut rgba = [0_u8; 16];
            assert_eq!(
                egui_states_server_image_get(
                    server,
                    image_id,
                    rgba.as_mut_ptr(),
                    rgba.len(),
                    &mut required,
                    &mut height,
                    &mut width,
                ),
                egui_states_status_t::EGUI_STATES_STATUS_OK
            );
            assert_eq!(&rgba[..4], &[1, 1, 1, 255]);

            assert_eq!(
                egui_states_server_image_multi_set(server, image_multi_id, 7, image, 0),
                egui_states_status_t::EGUI_STATES_STATUS_OK
            );
            required = 0;
            assert_eq!(
                egui_states_server_image_multi_indices(
                    server,
                    image_multi_id,
                    ptr::null_mut(),
                    0,
                    &mut required,
                ),
                egui_states_status_t::EGUI_STATES_STATUS_OK
            );
            assert_eq!(required, 1);
            let mut misaligned = [0_u8; 8];
            let misaligned_offset = (0..std::mem::align_of::<u32>())
                .find(|offset| {
                    !(misaligned.as_ptr() as usize + offset)
                        .is_multiple_of(std::mem::align_of::<u32>())
                })
                .unwrap();
            assert_eq!(
                egui_states_server_image_multi_indices(
                    server,
                    image_multi_id,
                    misaligned.as_mut_ptr().add(misaligned_offset).cast(),
                    1,
                    &mut required,
                ),
                egui_states_status_t::EGUI_STATES_STATUS_INVALID_ARGUMENT
            );
            let mut indices = [0_u32; 1];
            assert_eq!(
                egui_states_server_image_multi_indices(
                    server,
                    image_multi_id,
                    indices.as_mut_ptr(),
                    indices.len(),
                    &mut required,
                ),
                egui_states_status_t::EGUI_STATES_STATUS_OK
            );
            assert_eq!(indices, [7]);

            let numeric = [1_u16.to_ne_bytes(), 2_u16.to_ne_bytes()].concat();
            let data = egui_states_data_view_t {
                data: numeric.as_ptr().cast(),
                byte_length: numeric.len(),
                element_count: 2,
            };
            assert_eq!(
                egui_states_server_data_set(server, data_id, data, 0),
                egui_states_status_t::EGUI_STATES_STATUS_OK
            );
            required = 0;
            assert_eq!(
                egui_states_server_data_get(server, data_id, ptr::null_mut(), 0, &mut required,),
                egui_states_status_t::EGUI_STATES_STATUS_OK
            );
            assert_eq!(required, numeric.len());
            let mut short_numeric = [0_u8; 2];
            assert_eq!(
                egui_states_server_data_get(
                    server,
                    data_id,
                    short_numeric.as_mut_ptr().cast(),
                    short_numeric.len(),
                    &mut required,
                ),
                egui_states_status_t::EGUI_STATES_STATUS_BUFFER_TOO_SMALL
            );
            let invalid_data = egui_states_data_view_t {
                element_count: 3,
                ..data
            };
            assert_eq!(
                egui_states_server_data_set(server, data_id, invalid_data, 0),
                egui_states_status_t::EGUI_STATES_STATUS_INVALID_ARGUMENT
            );
            let invalid_stride = egui_states_image_view_t {
                row_stride: 1,
                ..image
            };
            assert_eq!(
                egui_states_server_image_set(server, image_id, invalid_stride, 0),
                egui_states_status_t::EGUI_STATES_STATUS_INVALID_ARGUMENT
            );
            let invalid_format = egui_states_image_view_t {
                format: 99,
                ..image
            };
            assert_eq!(
                egui_states_server_image_set(server, image_id, invalid_format, 0),
                egui_states_status_t::EGUI_STATES_STATUS_INVALID_ARGUMENT
            );

            assert_eq!(
                crate::abi::ffi_call(|| -> egui_states::c_api::Result<()> {
                    panic!("intentional C ABI containment test")
                }),
                egui_states_status_t::EGUI_STATES_STATUS_PANIC
            );
            egui_states_server_destroy(server);
        }
    }

    struct ClientState {
        value: Value<i32>,
    }

    impl State for ClientState {
        const NAME: &'static str = "ClientState";

        fn new(c: &mut impl StatesCreator) -> Self {
            Self {
                value: c.value("value", 0),
            }
        }
    }

    #[test]
    fn c_abi_server_exchanges_a_value_with_rust_client() {
        unsafe {
            let listener = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
            let port = listener.local_addr().unwrap().port();
            drop(listener);

            let mut server = ptr::null_mut();
            assert_eq!(
                egui_states_server_create(ptr::null(), &mut server),
                egui_states_status_t::EGUI_STATES_STATUS_OK
            );
            let mut object_type = ptr::null_mut();
            egui_states_object_type_i32(&mut object_type);
            let mut initial = ptr::null_mut();
            egui_states_value_create_i32(0, &mut initial);
            let mut id = 0;
            assert_eq!(
                egui_states_server_add_value(
                    server,
                    view("root.value"),
                    object_type,
                    initial,
                    0,
                    &mut id,
                ),
                egui_states_status_t::EGUI_STATES_STATUS_OK
            );
            egui_states_object_type_destroy(object_type);
            egui_states_value_destroy(initial);
            egui_states_server_finalize(server);
            egui_states_server_signal_register(server, id, 1, 0);
            let ip = [127, 0, 0, 1];
            assert_eq!(
                egui_states_server_start(server, port, ip.as_ptr(), ptr::null()),
                egui_states_status_t::EGUI_STATES_STATUS_OK
            );

            let (state, client) = ClientBuilder::<ClientState>::new().build(port);
            let mut connected = 0;
            'connect: for _ in 0..20 {
                client.connect();
                for _ in 0..10 {
                    thread::sleep(Duration::from_millis(10));
                    egui_states_server_is_connected(server, &mut connected);
                    if connected == 1 {
                        break 'connect;
                    }
                }
            }
            assert_eq!(connected, 1);

            let mut updated = ptr::null_mut();
            egui_states_value_create_i32(42, &mut updated);
            assert_eq!(
                egui_states_server_value_set(server, id, updated, 0, 0),
                egui_states_status_t::EGUI_STATES_STATUS_OK
            );
            egui_states_value_destroy(updated);
            for _ in 0..100 {
                if state.value.get() == 42 {
                    break;
                }
                thread::sleep(Duration::from_millis(10));
            }
            assert_eq!(state.value.get(), 42);

            state.value.set_signal(9);
            let mut event = ptr::null_mut();
            assert_eq!(
                egui_states_server_signal_wait(server, 1_000, &mut event),
                egui_states_status_t::EGUI_STATES_STATUS_OK
            );
            let mut event_value = ptr::null();
            egui_states_signal_event_value(event, &mut event_value);
            let mut received = 0;
            egui_states_value_get_i32(event_value, &mut received);
            assert_eq!(received, 9);
            egui_states_signal_event_destroy(event);

            client.disconnect();
            egui_states_server_stop(server);
            egui_states_server_destroy(server);
        }
    }

    #[test]
    fn public_header_compiles_as_c11_and_cpp() {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let fixture = manifest.join("tests/header_smoke.c");
        let include = manifest.join("include");
        for (compiler, language, standard) in [("cc", "c", "c11"), ("c++", "c++", "c++17")] {
            let result = Command::new(compiler)
                .arg(format!("-std={standard}"))
                .args([
                    "-Wall",
                    "-Wextra",
                    "-Werror",
                    "-fsyntax-only",
                    "-x",
                    language,
                ])
                .arg(format!("-I{}", include.display()))
                .arg(&fixture)
                .status();
            match result {
                Ok(status) => assert!(status.success(), "{compiler} rejected the public header"),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => panic!("failed to execute {compiler}: {error}"),
            }
        }
    }
}
