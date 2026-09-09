# ruff: noqa: D103

import pytest

from egui_states_test_bindings import (
    State,
    StatesServer,
)
from egui_states_test_bindings.enums import (
    TestEnum as ExampleTestEnum,
)
from egui_states_test_bindings.enums import (
    TestEnum2 as ExampleTestEnum2,
)
from egui_states_test_bindings.structs import (
    TestStruct as ExampleTestStruct,
)
from egui_states_test_bindings.structs import (
    TestStruct2 as ExampleTestStruct2,
)


def test_server_lifecycle_and_value_roundtrips(server_bundle: tuple[StatesServer, State, list[Exception]]) -> None:
    server, states, _errors = server_bundle

    assert server.is_running()
    assert not server.is_connected()

    states.values.bool_value.set(True)
    states.values.count.set(41)
    states.values.ratio.set(0.75)
    states.values.queued_progress.set(0.5)
    states.values.title.set("title value")
    states.values.optional_value.set(13)
    states.values.fixed_numbers.set([3, 5, 8])
    states.values.test_enum.set(ExampleTestEnum.C)
    states.values.nested.secondary_choice.set(ExampleTestEnum2.Z)
    states.values.nested.selected_enum.set(ExampleTestEnum.B)

    assert states.values.bool_value.get() is True
    assert states.values.count.get() == 41
    assert states.values.ratio.get() == pytest.approx(0.75)
    assert states.values.queued_progress.get() == pytest.approx(0.5)
    assert states.values.title.get() == "title value"
    assert states.values.optional_value.get() == 13
    assert states.values.fixed_numbers.get() == [3, 5, 8]
    assert states.values.test_enum.get() == ExampleTestEnum.C
    assert states.values.nested.secondary_choice.get() == ExampleTestEnum2.Z
    assert states.values.nested.selected_enum.get() == ExampleTestEnum.B

    point = ExampleTestStruct(1.25, -4.5, "origin")
    optional_struct = ExampleTestStruct2(True, 6, "nested")
    states.custom_values.point.set(point)
    states.custom_values.optional_struct.set(optional_struct)

    assert states.custom_values.point.get() == point
    assert states.custom_values.optional_struct.get() == optional_struct


def test_static_value_roundtrips(server_bundle: tuple[StatesServer, State, list[Exception]]) -> None:
    _server, states, _errors = server_bundle

    summary = ExampleTestStruct2(True, 9, "summary")
    states.statics.status_text.set("static text")
    states.statics.summary.set(summary)
    states.statics.pair.set([1.5, 2.5])
    states.statics.nested.label.set("nested static")
    states.statics.nested.enum_hint.set(ExampleTestEnum.B)

    assert states.statics.status_text.get() == "static text"
    assert states.statics.summary.get() == summary
    assert states.statics.pair.get() == pytest.approx([1.5, 2.5])
    assert states.statics.nested.label.get() == "nested static"
    assert states.statics.nested.enum_hint.get() == ExampleTestEnum.B
