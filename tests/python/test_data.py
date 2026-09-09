# ruff: noqa: D103

import numpy as np
import pytest

from egui_states_test_bindings import (
    State,
    StatesServer,
)


def test_data_array_methods(server_bundle: tuple[StatesServer, State, list[Exception]]) -> None:
    _server, states, _errors = server_bundle

    bytes_data = np.arange(6, dtype=np.uint8)
    states.data.bytes.set(bytes_data)
    states.data.bytes.add(np.array([6, 7], dtype=np.uint8))
    states.data.bytes.replace(np.array([50, 51], dtype=np.uint8), 2)
    states.data.bytes.remove(1, 2)
    np.testing.assert_array_equal(
        states.data.bytes.get(),
        np.array([0, 51, 4, 5, 6, 7], dtype=np.uint8),
    )
    states.data.bytes.clear()
    np.testing.assert_array_equal(states.data.bytes.get(), np.array([], dtype=np.uint8))

    samples = np.linspace(0.0, 1.0, 5, dtype=np.float32)
    states.data.samples.set(samples)
    np.testing.assert_allclose(states.data.samples.get(), samples)

    nested_buffer = np.arange(4, dtype=np.uint16)
    states.data.nested.buffer.set(nested_buffer)
    states.data.nested.buffer.add(np.array([4, 5], dtype=np.uint16))
    np.testing.assert_array_equal(
        states.data.nested.buffer.get(),
        np.array([0, 1, 2, 3, 4, 5], dtype=np.uint16),
    )


def test_multi_data_methods(server_bundle: tuple[StatesServer, State, list[Exception]]) -> None:
    _server, states, _errors = server_bundle

    bytes_zero = states.multi_data.bytes[0]
    bytes_zero.set(np.array([0, 1, 2, 3], dtype=np.uint8))
    bytes_zero.add(np.array([4, 5], dtype=np.uint8))
    bytes_zero.replace(np.array([50, 51], dtype=np.uint8), 2)
    bytes_zero.remove(1, 2)
    np.testing.assert_array_equal(bytes_zero.get(), np.array([0, 51, 4, 5], dtype=np.uint8))
    bytes_zero.clear()
    np.testing.assert_array_equal(bytes_zero.get(), np.array([], dtype=np.uint8))

    bytes_one = states.multi_data.bytes.get(1)
    bytes_one.set(np.array([9, 8, 7], dtype=np.uint8))
    bytes_one.add(np.array([6], dtype=np.uint8))
    np.testing.assert_array_equal(bytes_one.get(), np.array([9, 8, 7, 6], dtype=np.uint8))
    states.multi_data.bytes.remove_index(1)
    with pytest.raises(ValueError, match="DataMulti index not found"):
        states.multi_data.bytes[1].get()

    states.multi_data.bytes[2].set(np.array([42, 43], dtype=np.uint8))
    states.multi_data.bytes.remove_index(2)
    with pytest.raises(ValueError, match="DataMulti index not found"):
        states.multi_data.bytes[2].get()

    samples = np.linspace(0.0, 1.0, 5, dtype=np.float32)
    states.multi_data.samples[3].set(samples)
    states.multi_data.samples[3].add(np.array([1.25], dtype=np.float32))
    np.testing.assert_allclose(
        states.multi_data.samples[3].get(),
        np.array([0.0, 0.25, 0.5, 0.75, 1.0, 1.25], dtype=np.float32),
    )

    nested_buffer = states.multi_data.nested.buffer[4]
    nested_buffer.set(np.array([0, 1, 2, 3], dtype=np.uint16))
    nested_buffer.replace(np.array([10, 11], dtype=np.uint16), 1)
    nested_buffer.add(np.array([12], dtype=np.uint16))
    np.testing.assert_array_equal(
        nested_buffer.get(),
        np.array([0, 10, 11, 3, 12], dtype=np.uint16),
    )

    states.multi_data.bytes[5].set(np.array([1, 2, 3], dtype=np.uint8))
    states.multi_data.bytes.reset()
    with pytest.raises(ValueError, match="DataMulti index not found"):
        states.multi_data.bytes[0].get()
    with pytest.raises(ValueError, match="DataMulti index not found"):
        states.multi_data.bytes[5].get()


def test_value_take_methods(server_bundle: tuple[StatesServer, State, list[Exception]]) -> None:
    _server, states, _errors = server_bundle

    states.value_take.take_text.set("queued take", update=True)
    states.value_take.take_empty.set(update=True)


def test_data_take_methods(server_bundle: tuple[StatesServer, State, list[Exception]]) -> None:
    _server, states, _errors = server_bundle

    buffer_data = np.array([0, 1, 2, 3, 4], dtype=np.uint8)
    states.data_take.take_buffer.set(buffer_data, blocking=True, update=True)

    samples = np.linspace(0.0, 1.0, 5, dtype=np.float32)
    states.data_take.take_samples.set(samples, blocking=False, update=True)


def test_data_multi_take_methods(server_bundle: tuple[StatesServer, State, list[Exception]]) -> None:
    _server, states, _errors = server_bundle

    states.data_multi_take.bytes[0].set(np.array([1, 2, 3], dtype=np.uint8), update=True)
    states.data_multi_take.bytes[1].set(np.array([4, 5], dtype=np.uint8), blocking=True, update=True)

    states.data_multi_take.samples[0].set(np.linspace(0.0, 1.0, 4, dtype=np.float32), update=True)

    states.data_multi_take.nested.buffer[2].set(np.array([100, 200], dtype=np.uint16), update=True)

    states.data_multi_take.bytes.remove_index(0, update=True)
    states.data_multi_take.bytes.reset(update=True)
