#ifndef EGUI_STATES_H
#define EGUI_STATES_H

#include <stddef.h>
#include <stdint.h>

#define EGUI_STATES_ABI_VERSION 1u
#define EGUI_STATES_WAIT_FOREVER UINT64_MAX

#if defined(_WIN32) && !defined(EGUI_STATES_STATIC)
#  if defined(EGUI_STATES_BUILD)
#    define EGUI_STATES_API __declspec(dllexport)
#  else
#    define EGUI_STATES_API __declspec(dllimport)
#  endif
#elif defined(__GNUC__) || defined(__clang__)
#  define EGUI_STATES_API __attribute__((visibility("default")))
#else
#  define EGUI_STATES_API
#endif

#ifdef __cplusplus
extern "C" {
#endif

/*
 * All fallible calls return a status and store a thread-local diagnostic on
 * failure. The pointer returned by egui_states_last_error_message remains
 * valid until the next failing call on the same thread.
 *
 * Type and value objects are immutable and may be shared between threads.
 * Composite constructors deep-copy their children. Accessors for nested
 * values and signal-event values return borrowed pointers owned by the root
 * object/event; do not destroy them.
 *
 * Input strings and buffers are borrowed only for the duration of the call.
 * For copy-out functions, a null destination with zero capacity is a sizing
 * query and writes the required byte/element count. A non-null undersized
 * destination returns EGUI_STATES_STATUS_BUFFER_TOO_SMALL. Copied strings are
 * UTF-8 byte sequences and are not NUL-terminated.
 *
 * Registration/finalization/start/stop transitions must be externally
 * serialized. Other server operations are thread-safe after finalization.
 * Destroy a server only after every concurrent call has returned.
 */

typedef struct egui_states_server_t egui_states_server_t;
typedef struct egui_states_object_type_t egui_states_object_type_t;
typedef struct egui_states_value_t egui_states_value_t;
typedef struct egui_states_signal_event_t egui_states_signal_event_t;

typedef enum egui_states_status_t {
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
    EGUI_STATES_STATUS_PANIC = 10
} egui_states_status_t;

typedef enum egui_states_value_kind_t {
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
    EGUI_STATES_VALUE_EMPTY = 16
} egui_states_value_kind_t;

typedef enum egui_states_data_type_t {
    EGUI_STATES_DATA_U8 = 0,
    EGUI_STATES_DATA_U16 = 1,
    EGUI_STATES_DATA_U32 = 2,
    EGUI_STATES_DATA_U64 = 3,
    EGUI_STATES_DATA_I8 = 4,
    EGUI_STATES_DATA_I16 = 5,
    EGUI_STATES_DATA_I32 = 6,
    EGUI_STATES_DATA_I64 = 7,
    EGUI_STATES_DATA_F32 = 8,
    EGUI_STATES_DATA_F64 = 9
} egui_states_data_type_t;

typedef enum egui_states_image_format_t {
    EGUI_STATES_IMAGE_GRAY = 0,
    EGUI_STATES_IMAGE_GRAY_ALPHA = 1,
    EGUI_STATES_IMAGE_RGB = 2,
    EGUI_STATES_IMAGE_RGBA = 3
} egui_states_image_format_t;

typedef struct egui_states_string_view_t {
    const char *data;
    size_t length;
} egui_states_string_view_t;

typedef struct egui_states_data_view_t {
    const void *data;
    size_t byte_length;
    size_t element_count;
} egui_states_data_view_t;

typedef struct egui_states_image_view_t {
    const uint8_t *data;
    size_t byte_length;
    size_t height;
    size_t width;
    size_t row_stride; /* Zero means tightly packed. */
    uint32_t format;   /* One of egui_states_image_format_t. */
} egui_states_image_view_t;

EGUI_STATES_API uint32_t egui_states_abi_version(void);
EGUI_STATES_API const char *egui_states_last_error_message(void);
EGUI_STATES_API void egui_states_server_destroy(egui_states_server_t *server);
EGUI_STATES_API void egui_states_object_type_destroy(egui_states_object_type_t *object_type);
EGUI_STATES_API void egui_states_value_destroy(egui_states_value_t *value);
EGUI_STATES_API void egui_states_signal_event_destroy(egui_states_signal_event_t *event);

/* Runtime protocol type descriptors. Every successful call returns ownership. */
EGUI_STATES_API egui_states_status_t egui_states_object_type_u8(egui_states_object_type_t **out_type);
EGUI_STATES_API egui_states_status_t egui_states_object_type_u16(egui_states_object_type_t **out_type);
EGUI_STATES_API egui_states_status_t egui_states_object_type_u32(egui_states_object_type_t **out_type);
EGUI_STATES_API egui_states_status_t egui_states_object_type_u64(egui_states_object_type_t **out_type);
EGUI_STATES_API egui_states_status_t egui_states_object_type_i8(egui_states_object_type_t **out_type);
EGUI_STATES_API egui_states_status_t egui_states_object_type_i16(egui_states_object_type_t **out_type);
EGUI_STATES_API egui_states_status_t egui_states_object_type_i32(egui_states_object_type_t **out_type);
EGUI_STATES_API egui_states_status_t egui_states_object_type_i64(egui_states_object_type_t **out_type);
EGUI_STATES_API egui_states_status_t egui_states_object_type_f32(egui_states_object_type_t **out_type);
EGUI_STATES_API egui_states_status_t egui_states_object_type_f64(egui_states_object_type_t **out_type);
EGUI_STATES_API egui_states_status_t egui_states_object_type_bool(egui_states_object_type_t **out_type);
EGUI_STATES_API egui_states_status_t egui_states_object_type_string(egui_states_object_type_t **out_type);
EGUI_STATES_API egui_states_status_t egui_states_object_type_empty(egui_states_object_type_t **out_type);
EGUI_STATES_API egui_states_status_t egui_states_object_type_clone(const egui_states_object_type_t *object_type, egui_states_object_type_t **out_type);
EGUI_STATES_API egui_states_status_t egui_states_object_type_option(const egui_states_object_type_t *inner, egui_states_object_type_t **out_type);
EGUI_STATES_API egui_states_status_t egui_states_object_type_tuple(const egui_states_object_type_t *const *elements, size_t count, egui_states_object_type_t **out_type);
EGUI_STATES_API egui_states_status_t egui_states_object_type_struct(egui_states_string_view_t name, const egui_states_string_view_t *field_names, const egui_states_object_type_t *const *field_types, size_t count, egui_states_object_type_t **out_type);
EGUI_STATES_API egui_states_status_t egui_states_object_type_list(const egui_states_object_type_t *element, uint32_t size, egui_states_object_type_t **out_type);
EGUI_STATES_API egui_states_status_t egui_states_object_type_vec(const egui_states_object_type_t *element, egui_states_object_type_t **out_type);
EGUI_STATES_API egui_states_status_t egui_states_object_type_map(const egui_states_object_type_t *key, const egui_states_object_type_t *value, egui_states_object_type_t **out_type);
EGUI_STATES_API egui_states_status_t egui_states_object_type_enum(egui_states_string_view_t name, const egui_states_string_view_t *variant_names, const int32_t *discriminants, size_t count, egui_states_object_type_t **out_type);

/* Immutable dynamic values. */
EGUI_STATES_API egui_states_status_t egui_states_value_create_u8(uint8_t value, egui_states_value_t **out_value);
EGUI_STATES_API egui_states_status_t egui_states_value_create_u16(uint16_t value, egui_states_value_t **out_value);
EGUI_STATES_API egui_states_status_t egui_states_value_create_u32(uint32_t value, egui_states_value_t **out_value);
EGUI_STATES_API egui_states_status_t egui_states_value_create_u64(uint64_t value, egui_states_value_t **out_value);
EGUI_STATES_API egui_states_status_t egui_states_value_create_i8(int8_t value, egui_states_value_t **out_value);
EGUI_STATES_API egui_states_status_t egui_states_value_create_i16(int16_t value, egui_states_value_t **out_value);
EGUI_STATES_API egui_states_status_t egui_states_value_create_i32(int32_t value, egui_states_value_t **out_value);
EGUI_STATES_API egui_states_status_t egui_states_value_create_i64(int64_t value, egui_states_value_t **out_value);
EGUI_STATES_API egui_states_status_t egui_states_value_create_f32(float value, egui_states_value_t **out_value);
EGUI_STATES_API egui_states_status_t egui_states_value_create_f64(double value, egui_states_value_t **out_value);
EGUI_STATES_API egui_states_status_t egui_states_value_create_bool(uint8_t value, egui_states_value_t **out_value);
EGUI_STATES_API egui_states_status_t egui_states_value_create_string(egui_states_string_view_t value, egui_states_value_t **out_value);
EGUI_STATES_API egui_states_status_t egui_states_value_create_enum(uint32_t ordinal, egui_states_value_t **out_value);
EGUI_STATES_API egui_states_status_t egui_states_value_create_sequence(const egui_states_value_t *const *elements, size_t count, egui_states_value_t **out_value);
EGUI_STATES_API egui_states_status_t egui_states_value_create_map(const egui_states_value_t *const *keys, const egui_states_value_t *const *values, size_t count, egui_states_value_t **out_value);
EGUI_STATES_API egui_states_status_t egui_states_value_create_none(egui_states_value_t **out_value);
EGUI_STATES_API egui_states_status_t egui_states_value_create_some(const egui_states_value_t *value, egui_states_value_t **out_value);
EGUI_STATES_API egui_states_status_t egui_states_value_create_empty(egui_states_value_t **out_value);
EGUI_STATES_API egui_states_status_t egui_states_value_clone(const egui_states_value_t *value, egui_states_value_t **out_value);
EGUI_STATES_API egui_states_status_t egui_states_value_kind(const egui_states_value_t *value, egui_states_value_kind_t *out_kind);
EGUI_STATES_API egui_states_status_t egui_states_value_get_u8(const egui_states_value_t *value, uint8_t *out_value);
EGUI_STATES_API egui_states_status_t egui_states_value_get_u16(const egui_states_value_t *value, uint16_t *out_value);
EGUI_STATES_API egui_states_status_t egui_states_value_get_u32(const egui_states_value_t *value, uint32_t *out_value);
EGUI_STATES_API egui_states_status_t egui_states_value_get_u64(const egui_states_value_t *value, uint64_t *out_value);
EGUI_STATES_API egui_states_status_t egui_states_value_get_i8(const egui_states_value_t *value, int8_t *out_value);
EGUI_STATES_API egui_states_status_t egui_states_value_get_i16(const egui_states_value_t *value, int16_t *out_value);
EGUI_STATES_API egui_states_status_t egui_states_value_get_i32(const egui_states_value_t *value, int32_t *out_value);
EGUI_STATES_API egui_states_status_t egui_states_value_get_i64(const egui_states_value_t *value, int64_t *out_value);
EGUI_STATES_API egui_states_status_t egui_states_value_get_f32(const egui_states_value_t *value, float *out_value);
EGUI_STATES_API egui_states_status_t egui_states_value_get_f64(const egui_states_value_t *value, double *out_value);
EGUI_STATES_API egui_states_status_t egui_states_value_get_bool(const egui_states_value_t *value, uint8_t *out_value);
EGUI_STATES_API egui_states_status_t egui_states_value_get_string(const egui_states_value_t *value, egui_states_string_view_t *out_value);
EGUI_STATES_API egui_states_status_t egui_states_value_get_enum(const egui_states_value_t *value, uint32_t *out_ordinal);
EGUI_STATES_API egui_states_status_t egui_states_value_sequence_len(const egui_states_value_t *value, size_t *out_length);
EGUI_STATES_API egui_states_status_t egui_states_value_sequence_get(const egui_states_value_t *value, size_t index, const egui_states_value_t **out_value);
EGUI_STATES_API egui_states_status_t egui_states_value_map_len(const egui_states_value_t *value, size_t *out_length);
EGUI_STATES_API egui_states_status_t egui_states_value_map_key(const egui_states_value_t *value, size_t index, const egui_states_value_t **out_key);
EGUI_STATES_API egui_states_status_t egui_states_value_map_value(const egui_states_value_t *value, size_t index, const egui_states_value_t **out_value);
EGUI_STATES_API egui_states_status_t egui_states_value_option_has_value(const egui_states_value_t *value, uint8_t *out_has_value);
EGUI_STATES_API egui_states_status_t egui_states_value_option_get(const egui_states_value_t *value, const egui_states_value_t **out_value);

/* Server lifecycle and registration. Optional arguments are represented by null pointers. */
EGUI_STATES_API egui_states_status_t egui_states_server_create(const uint64_t *version, egui_states_server_t **out_server);
EGUI_STATES_API egui_states_status_t egui_states_server_finalize(const egui_states_server_t *server);
EGUI_STATES_API egui_states_status_t egui_states_server_start(const egui_states_server_t *server, uint16_t port, const uint8_t *ip_address_4_bytes, const egui_states_string_view_t *token);
EGUI_STATES_API egui_states_status_t egui_states_server_stop(const egui_states_server_t *server);
EGUI_STATES_API egui_states_status_t egui_states_server_disconnect_client(const egui_states_server_t *server);
EGUI_STATES_API egui_states_status_t egui_states_server_is_running(const egui_states_server_t *server, uint8_t *out_value);
EGUI_STATES_API egui_states_status_t egui_states_server_is_connected(const egui_states_server_t *server, uint8_t *out_value);
EGUI_STATES_API egui_states_status_t egui_states_server_update(const egui_states_server_t *server, const float *duration_seconds);
EGUI_STATES_API egui_states_status_t egui_states_server_id_to_name(const egui_states_server_t *server, uint64_t id, char *destination, size_t capacity, size_t *out_required);
EGUI_STATES_API egui_states_status_t egui_states_server_add_value(const egui_states_server_t *server, egui_states_string_view_t name, const egui_states_object_type_t *object_type, const egui_states_value_t *initial, uint8_t queue, uint64_t *out_id);
EGUI_STATES_API egui_states_status_t egui_states_server_add_value_take(const egui_states_server_t *server, egui_states_string_view_t name, const egui_states_object_type_t *object_type, uint64_t *out_id);
EGUI_STATES_API egui_states_status_t egui_states_server_add_static(const egui_states_server_t *server, egui_states_string_view_t name, const egui_states_object_type_t *object_type, const egui_states_value_t *initial, uint64_t *out_id);
EGUI_STATES_API egui_states_status_t egui_states_server_add_signal(const egui_states_server_t *server, egui_states_string_view_t name, const egui_states_object_type_t *object_type, uint8_t queue, uint64_t *out_id);
EGUI_STATES_API egui_states_status_t egui_states_server_add_list(const egui_states_server_t *server, egui_states_string_view_t name, const egui_states_object_type_t *element_type, uint64_t *out_id);
EGUI_STATES_API egui_states_status_t egui_states_server_add_map(const egui_states_server_t *server, egui_states_string_view_t name, const egui_states_object_type_t *key_type, const egui_states_object_type_t *value_type, uint64_t *out_id);
EGUI_STATES_API egui_states_status_t egui_states_server_add_image(const egui_states_server_t *server, egui_states_string_view_t name, uint64_t *out_id);
EGUI_STATES_API egui_states_status_t egui_states_server_add_image_multi(const egui_states_server_t *server, egui_states_string_view_t name, uint64_t *out_id);
EGUI_STATES_API egui_states_status_t egui_states_server_add_data(const egui_states_server_t *server, egui_states_string_view_t name, uint32_t data_type, uint64_t *out_id);
EGUI_STATES_API egui_states_status_t egui_states_server_add_data_take(const egui_states_server_t *server, egui_states_string_view_t name, uint32_t data_type, uint64_t *out_id);
EGUI_STATES_API egui_states_status_t egui_states_server_add_data_multi(const egui_states_server_t *server, egui_states_string_view_t name, uint32_t data_type, uint64_t *out_id);
EGUI_STATES_API egui_states_status_t egui_states_server_add_data_multi_take(const egui_states_server_t *server, egui_states_string_view_t name, uint32_t data_type, uint64_t *out_id);

/* Values, statics, signals, lists, and maps. Returned values are owned unless marked const. */
EGUI_STATES_API egui_states_status_t egui_states_server_value_get(const egui_states_server_t *server, uint64_t id, egui_states_value_t **out_value);
EGUI_STATES_API egui_states_status_t egui_states_server_value_set(const egui_states_server_t *server, uint64_t id, const egui_states_value_t *value, uint8_t set_signal, uint8_t update);
EGUI_STATES_API egui_states_status_t egui_states_server_value_take_set(const egui_states_server_t *server, uint64_t id, const egui_states_value_t *value, uint8_t blocking, uint8_t update);
EGUI_STATES_API egui_states_status_t egui_states_server_static_get(const egui_states_server_t *server, uint64_t id, egui_states_value_t **out_value);
EGUI_STATES_API egui_states_status_t egui_states_server_static_set(const egui_states_server_t *server, uint64_t id, const egui_states_value_t *value, uint8_t update);
EGUI_STATES_API egui_states_status_t egui_states_server_signal_set(const egui_states_server_t *server, uint64_t id, const egui_states_value_t *value);
EGUI_STATES_API egui_states_status_t egui_states_server_signal_register(const egui_states_server_t *server, uint64_t id, uint8_t register_signal, uint8_t with_previous);
EGUI_STATES_API egui_states_status_t egui_states_server_signal_set_to_queue(const egui_states_server_t *server, uint64_t id);
EGUI_STATES_API egui_states_status_t egui_states_server_signal_set_to_single(const egui_states_server_t *server, uint64_t id);
/* timeout_milliseconds: 0 tries once; EGUI_STATES_WAIT_FOREVER blocks indefinitely.
 * A successful wait returns an owned event that holds the id's serialization
 * claim. Destroying the event releases that claim and its borrowed values. */
EGUI_STATES_API egui_states_status_t egui_states_server_signal_wait(const egui_states_server_t *server, uint64_t timeout_milliseconds, egui_states_signal_event_t **out_event);
EGUI_STATES_API egui_states_status_t egui_states_signal_event_id(const egui_states_signal_event_t *event, uint64_t *out_id);
EGUI_STATES_API egui_states_status_t egui_states_signal_event_value(const egui_states_signal_event_t *event, const egui_states_value_t **out_value);
EGUI_STATES_API egui_states_status_t egui_states_signal_event_has_previous(const egui_states_signal_event_t *event, uint8_t *out_has_previous);
EGUI_STATES_API egui_states_status_t egui_states_signal_event_previous(const egui_states_signal_event_t *event, const egui_states_value_t **out_value);
EGUI_STATES_API egui_states_status_t egui_states_server_list_set(const egui_states_server_t *server, uint64_t id, const egui_states_value_t *list, uint8_t update);
EGUI_STATES_API egui_states_status_t egui_states_server_list_get(const egui_states_server_t *server, uint64_t id, egui_states_value_t **out_list);
EGUI_STATES_API egui_states_status_t egui_states_server_list_set_item(const egui_states_server_t *server, uint64_t id, size_t index, const egui_states_value_t *item, uint8_t update);
EGUI_STATES_API egui_states_status_t egui_states_server_list_get_item(const egui_states_server_t *server, uint64_t id, size_t index, egui_states_value_t **out_item);
EGUI_STATES_API egui_states_status_t egui_states_server_list_remove_item(const egui_states_server_t *server, uint64_t id, size_t index, uint8_t update, egui_states_value_t **out_item);
EGUI_STATES_API egui_states_status_t egui_states_server_list_append_item(const egui_states_server_t *server, uint64_t id, const egui_states_value_t *item, uint8_t update);
EGUI_STATES_API egui_states_status_t egui_states_server_list_len(const egui_states_server_t *server, uint64_t id, size_t *out_length);
EGUI_STATES_API egui_states_status_t egui_states_server_map_set(const egui_states_server_t *server, uint64_t id, const egui_states_value_t *map, uint8_t update);
EGUI_STATES_API egui_states_status_t egui_states_server_map_get(const egui_states_server_t *server, uint64_t id, egui_states_value_t **out_map);
EGUI_STATES_API egui_states_status_t egui_states_server_map_set_item(const egui_states_server_t *server, uint64_t id, const egui_states_value_t *key, const egui_states_value_t *item, uint8_t update);
EGUI_STATES_API egui_states_status_t egui_states_server_map_get_item(const egui_states_server_t *server, uint64_t id, const egui_states_value_t *key, egui_states_value_t **out_item);
EGUI_STATES_API egui_states_status_t egui_states_server_map_remove_item(const egui_states_server_t *server, uint64_t id, const egui_states_value_t *key, uint8_t update, egui_states_value_t **out_item);
EGUI_STATES_API egui_states_status_t egui_states_server_map_len(const egui_states_server_t *server, uint64_t id, size_t *out_length);

/* Images. Getters copy packed RGBA bytes and report [height, width]. */
EGUI_STATES_API egui_states_status_t egui_states_server_image_size(const egui_states_server_t *server, uint64_t id, size_t *out_height, size_t *out_width);
EGUI_STATES_API egui_states_status_t egui_states_server_image_get(const egui_states_server_t *server, uint64_t id, uint8_t *destination, size_t capacity, size_t *out_required, size_t *out_height, size_t *out_width);
EGUI_STATES_API egui_states_status_t egui_states_server_image_set(const egui_states_server_t *server, uint64_t id, egui_states_image_view_t image, uint8_t update);
EGUI_STATES_API egui_states_status_t egui_states_server_image_set_all(const egui_states_server_t *server, uint64_t id, size_t height, size_t width, const uint8_t *rgba_4_bytes, uint8_t update);
EGUI_STATES_API egui_states_status_t egui_states_server_image_update(const egui_states_server_t *server, uint64_t id, egui_states_image_view_t image, size_t origin_y, size_t origin_x, uint8_t update, uint8_t force);
EGUI_STATES_API egui_states_status_t egui_states_server_image_multi_size(const egui_states_server_t *server, uint64_t id, uint32_t index, size_t *out_height, size_t *out_width);
EGUI_STATES_API egui_states_status_t egui_states_server_image_multi_get(const egui_states_server_t *server, uint64_t id, uint32_t index, uint8_t *destination, size_t capacity, size_t *out_required, size_t *out_height, size_t *out_width);
EGUI_STATES_API egui_states_status_t egui_states_server_image_multi_set(const egui_states_server_t *server, uint64_t id, uint32_t index, egui_states_image_view_t image, uint8_t update);
EGUI_STATES_API egui_states_status_t egui_states_server_image_multi_set_all(const egui_states_server_t *server, uint64_t id, uint32_t index, size_t height, size_t width, const uint8_t *rgba_4_bytes, uint8_t update);
EGUI_STATES_API egui_states_status_t egui_states_server_image_multi_update(const egui_states_server_t *server, uint64_t id, uint32_t index, egui_states_image_view_t image, size_t origin_y, size_t origin_x, uint8_t update, uint8_t force);
EGUI_STATES_API egui_states_status_t egui_states_server_image_multi_remove_index(const egui_states_server_t *server, uint64_t id, uint32_t index, uint8_t update);
EGUI_STATES_API egui_states_status_t egui_states_server_image_multi_reset(const egui_states_server_t *server, uint64_t id, uint8_t update);
EGUI_STATES_API egui_states_status_t egui_states_server_image_multi_len(const egui_states_server_t *server, uint64_t id, size_t *out_length);
EGUI_STATES_API egui_states_status_t egui_states_server_image_multi_contains(const egui_states_server_t *server, uint64_t id, uint32_t index, uint8_t *out_contains);
EGUI_STATES_API egui_states_status_t egui_states_server_image_multi_indices(const egui_states_server_t *server, uint64_t id, uint32_t *destination, size_t capacity, size_t *out_required);

/* Native-endian numeric buffers. Capacities and required sizes are bytes. */
EGUI_STATES_API egui_states_status_t egui_states_server_data_get(const egui_states_server_t *server, uint64_t id, void *destination, size_t capacity, size_t *out_required);
EGUI_STATES_API egui_states_status_t egui_states_server_data_set(const egui_states_server_t *server, uint64_t id, egui_states_data_view_t data, uint8_t update);
EGUI_STATES_API egui_states_status_t egui_states_server_data_add(const egui_states_server_t *server, uint64_t id, egui_states_data_view_t data, uint8_t update);
EGUI_STATES_API egui_states_status_t egui_states_server_data_replace(const egui_states_server_t *server, uint64_t id, egui_states_data_view_t data, size_t index, uint8_t update);
EGUI_STATES_API egui_states_status_t egui_states_server_data_remove(const egui_states_server_t *server, uint64_t id, size_t index, size_t count, uint8_t update);
EGUI_STATES_API egui_states_status_t egui_states_server_data_clear(const egui_states_server_t *server, uint64_t id, uint8_t update);
EGUI_STATES_API egui_states_status_t egui_states_server_data_take_set(const egui_states_server_t *server, uint64_t id, egui_states_data_view_t data, uint8_t blocking, uint8_t update, uint8_t cache);
EGUI_STATES_API egui_states_status_t egui_states_server_data_multi_get(const egui_states_server_t *server, uint64_t id, uint32_t index, void *destination, size_t capacity, size_t *out_required);
EGUI_STATES_API egui_states_status_t egui_states_server_data_multi_set(const egui_states_server_t *server, uint64_t id, uint32_t index, egui_states_data_view_t data, uint8_t update);
EGUI_STATES_API egui_states_status_t egui_states_server_data_multi_add(const egui_states_server_t *server, uint64_t id, uint32_t index, egui_states_data_view_t data, uint8_t update);
EGUI_STATES_API egui_states_status_t egui_states_server_data_multi_replace(const egui_states_server_t *server, uint64_t id, uint32_t index, egui_states_data_view_t data, size_t data_index, uint8_t update);
EGUI_STATES_API egui_states_status_t egui_states_server_data_multi_remove(const egui_states_server_t *server, uint64_t id, uint32_t index, size_t data_index, size_t count, uint8_t update);
EGUI_STATES_API egui_states_status_t egui_states_server_data_multi_clear(const egui_states_server_t *server, uint64_t id, uint32_t index, uint8_t update);
EGUI_STATES_API egui_states_status_t egui_states_server_data_multi_remove_index(const egui_states_server_t *server, uint64_t id, uint32_t index, uint8_t update);
EGUI_STATES_API egui_states_status_t egui_states_server_data_multi_reset(const egui_states_server_t *server, uint64_t id, uint8_t update);
EGUI_STATES_API egui_states_status_t egui_states_server_data_multi_take_set(const egui_states_server_t *server, uint64_t id, uint32_t index, egui_states_data_view_t data, uint8_t blocking, uint8_t update, uint8_t cache);
EGUI_STATES_API egui_states_status_t egui_states_server_data_multi_take_remove_index(const egui_states_server_t *server, uint64_t id, uint32_t index, uint8_t update);
EGUI_STATES_API egui_states_status_t egui_states_server_data_multi_take_reset(const egui_states_server_t *server, uint64_t id, uint8_t update);

#ifdef __cplusplus
}
#endif

#endif /* EGUI_STATES_H */
