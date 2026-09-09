# ruff: noqa: D103
import threading
import time

import pytest

from egui_states_test_bindings import (
    State,
    StatesServer,
)
from egui_states_test_bindings.enums import (
    TestEnum as ExampleTestEnum,
)


from .conftest import _free_port, _wait_until, _wait_event


class _Handler:
    def __init__(self) -> None:
        self.plain: list[str] = []
        self.pairs: list[tuple[str, str]] = []

    def on_value(self, value: str) -> None:
        self.plain.append(value)

    def on_value_previous(self, value: str, previous: str) -> None:
        self.pairs.append((value, previous))


def test_value_callbacks_disconnect_and_signal_mode(
    server_bundle: tuple[StatesServer, State, list[Exception]],
) -> None:
    _server, states, _errors = server_bundle

    title_values: list[str] = []
    title_event = threading.Event()

    def on_title(value: str) -> None:
        title_values.append(value)
        if len(title_values) >= 2:
            title_event.set()

    states.values.title.connect(on_title)
    states.values.title.signal_set_to_queue()
    states.values.title.set("first", set_signal=True)
    states.values.title.set("second", set_signal=True)
    _wait_event(title_event)
    assert title_values[:2] == ["first", "second"]

    states.values.title.disconnect(on_title)
    title_event.clear()
    states.values.title.set("third", set_signal=True)
    assert not title_event.wait(0.2)

    ratio_values: list[float] = []
    ratio_event = threading.Event()

    def on_ratio(value: float) -> None:
        ratio_values.append(value)
        ratio_event.set()

    states.values.ratio.connect(on_ratio)
    states.values.ratio.signal_set_to_single()
    states.values.ratio.set(0.9, set_signal=True)
    _wait_event(ratio_event)
    assert ratio_values[-1] == pytest.approx(0.9)

    states.values.ratio.disconnect_all()
    ratio_event.clear()
    states.values.ratio.set(0.1, set_signal=True)
    assert not ratio_event.wait(0.2)


def test_value_connect_previous_receives_replaced_value(
    server_bundle: tuple[StatesServer, State, list[Exception]],
) -> None:
    _server, states, errors = server_bundle

    pairs: list[tuple[str, str]] = []
    event = threading.Event()

    def on_title(value: str, previous: str) -> None:
        pairs.append((value, previous))
        event.set()

    states.values.title.connect_previous(on_title)

    # Queue mode delivers every change, so the chain is contiguous. The initial
    # value seeds it, so even the first change already has a previous.
    states.values.title.signal_set_to_queue()
    states.values.title.set("first", set_signal=True)
    states.values.title.set("second", set_signal=True)
    _wait_until(lambda: len(pairs) >= 2)
    assert pairs[:2] == [("first", ""), ("second", "first")]

    states.values.title.disconnect(on_title)
    event.clear()
    states.values.title.set("third", set_signal=True)
    assert not event.wait(0.2)
    assert errors == []


def test_value_connect_previous_coalesces_to_last_delivered(
    server_bundle: tuple[StatesServer, State, list[Exception]],
) -> None:
    _server, states, errors = server_bundle

    pairs: list[tuple[float, float]] = []
    gate = threading.Event()
    released = threading.Event()

    def on_ratio(value: float, previous: float) -> None:
        pairs.append((value, previous))
        # Block inside the first callback so the changes behind it coalesce
        # instead of being delivered one at a time.
        if len(pairs) == 1:
            gate.set()
            released.wait(1.0)

    states.values.ratio.connect_previous(on_ratio)
    states.values.ratio.signal_set_to_single()

    states.values.ratio.set(1.0, set_signal=True)
    _wait_event(gate)

    # a -> b -> c while the worker is held up; only c survives, and it must
    # report the value the callback was last told about, not the skipped 2.0.
    states.values.ratio.set(2.0, set_signal=True)
    states.values.ratio.set(3.0, set_signal=True)
    released.set()

    _wait_until(lambda: len(pairs) >= 2)
    assert pairs[0] == pytest.approx((1.0, 0.0))
    assert pairs[1] == pytest.approx((3.0, 1.0))
    assert errors == []


def test_value_disconnect_takes_either_variant(
    server_bundle: tuple[StatesServer, State, list[Exception]],
) -> None:
    _server, states, errors = server_bundle
    handler = _Handler()

    # Bound methods, so each `handler.on_value` is a fresh object and disconnecting
    # relies on them comparing equal rather than being identical.
    states.values.title.connect(handler.on_value)
    states.values.title.connect_previous(handler.on_value_previous)

    states.values.title.set("changed", set_signal=True)
    _wait_until(lambda: bool(handler.plain) and bool(handler.pairs))

    assert handler.plain == ["changed"]
    assert handler.pairs == [("changed", "")]

    # One disconnect for both, without having to say which kind it was.
    states.values.title.disconnect(handler.on_value_previous)
    states.values.title.set("again", set_signal=True)
    _wait_until(lambda: len(handler.plain) >= 2)
    time.sleep(0.2)
    assert handler.plain == ["changed", "again"]
    assert handler.pairs == [("changed", "")]

    states.values.title.disconnect(handler.on_value)
    states.values.title.set("third", set_signal=True)
    time.sleep(0.2)
    assert handler.plain == ["changed", "again"]

    # Disconnecting again, or disconnecting something never connected, is a no-op
    # rather than a KeyError or ValueError.
    states.values.title.disconnect(handler.on_value)
    states.values.title.disconnect(lambda value: None)
    assert errors == []


def test_value_connect_previous_treats_none_as_a_real_previous_value(
    server_bundle: tuple[StatesServer, State, list[Exception]],
) -> None:
    _server, states, errors = server_bundle

    pairs: list[tuple[int | None, int | None]] = []
    states.values.optional_value.connect_previous(lambda new, previous: pairs.append((new, previous)))
    states.values.optional_value.signal_set_to_queue()

    # An optional value deserializes a previous value of None, which must not be read
    # as "this change carries no previous". Its initial value is None, so the very
    # first change is the one that would be dropped.
    assert states.values.optional_value.get() is None
    states.values.optional_value.set(5, set_signal=True)
    states.values.optional_value.set(None, set_signal=True)
    states.values.optional_value.set(None, set_signal=True)

    _wait_until(lambda: len(pairs) >= 3)
    assert pairs[:3] == [(5, None), (None, 5), (None, None)]
    assert errors == []


def test_value_plain_and_previous_callbacks_coexist(
    server_bundle: tuple[StatesServer, State, list[Exception]],
) -> None:
    _server, states, errors = server_bundle

    plain: list[str] = []
    pairs: list[tuple[str, str]] = []

    states.values.title.connect(plain.append)
    states.values.title.connect_previous(lambda value, previous: pairs.append((value, previous)))

    states.values.title.set("changed", set_signal=True)
    _wait_until(lambda: bool(plain) and bool(pairs))

    assert plain == ["changed"]
    assert pairs == [("changed", "")]

    # disconnect_all has to clear both registries.
    states.values.title.disconnect_all()
    states.values.title.set("again", set_signal=True)
    time.sleep(0.2)
    assert plain == ["changed"]
    assert pairs == [("changed", "")]
    assert errors == []


def test_signal_callbacks_and_disconnect_all(
    server_bundle: tuple[StatesServer, State, list[Exception]],
) -> None:
    _server, states, _errors = server_bundle

    signal_events: list[str] = []
    empty_event = threading.Event()
    number_event = threading.Event()
    enum_event = threading.Event()

    def on_empty() -> None:
        signal_events.append("empty")
        empty_event.set()

    def on_number(value: float) -> None:
        signal_events.append(f"number:{value}")
        number_event.set()

    def on_enum(value: ExampleTestEnum) -> None:
        signal_events.append(f"enum:{value.name}")
        enum_event.set()

    states.signals.empty_signal.connect(on_empty)
    states.signals.number_signal.connect(on_number)
    states.signals.enum_signal.connect(on_enum)

    states.signals.empty_signal.signal_set_to_queue()
    states.signals.empty_signal.set()
    states.signals.number_signal.set(1.25)
    states.signals.enum_signal.set(ExampleTestEnum.B)

    _wait_event(empty_event)
    _wait_event(number_event)
    _wait_event(enum_event)
    assert "empty" in signal_events
    assert "number:1.25" in signal_events
    assert "enum:B" in signal_events

    states.signals.enum_signal.disconnect_all()
    enum_event.clear()
    states.signals.enum_signal.set(ExampleTestEnum.C)
    assert not enum_event.wait(0.2)


def test_error_handler_receives_callback_failures() -> None:
    captured_errors: list[Exception] = []
    error_event = threading.Event()

    def on_error(error: Exception) -> None:
        captured_errors.append(error)
        error_event.set()

    server = StatesServer(error_handler=on_error)
    server.start(_free_port())
    try:

        def explode(_value: float) -> None:
            raise ValueError("boom")

        server.states.signals.number_signal.connect(explode)
        server.states.signals.number_signal.set(3.5)
        _wait_event(error_event)
        _wait_until(lambda: bool(captured_errors))
        assert isinstance(captured_errors[0], ValueError)
        assert str(captured_errors[0]) == "boom"
    finally:
        if server.is_running():
            server.stop()
