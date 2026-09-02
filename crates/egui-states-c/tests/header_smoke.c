#include "egui_states.h"

static egui_states_string_view_t string_view(const char *data, size_t length) {
    egui_states_string_view_t result = {data, length};
    return result;
}

int header_smoke(void) {
    egui_states_server_t *server = NULL;
    egui_states_object_type_t *type = NULL;
    egui_states_value_t *value = NULL;
    uint64_t id = 0;

    if (egui_states_abi_version() != EGUI_STATES_ABI_VERSION) {
        return 1;
    }
    if (egui_states_server_create(NULL, &server) != EGUI_STATES_STATUS_OK) {
        return 2;
    }
    (void)egui_states_object_type_i32(&type);
    (void)egui_states_value_create_i32(0, &value);
    (void)egui_states_server_add_value(
        server, string_view("root.value", 10), type, value, 0, &id);
    egui_states_value_destroy(value);
    egui_states_object_type_destroy(type);
    egui_states_server_destroy(server);
    return 0;
}
