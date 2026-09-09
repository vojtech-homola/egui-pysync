# ruff: noqa: D103
import socket

import pytest

from egui_states_test_bindings import (
    StatesServer,
)


from .conftest import _free_port


def test_server_restarts_on_a_different_port() -> None:
    first_port = _free_port()
    second_port = _free_port()
    while second_port == first_port:
        second_port = _free_port()

    server = StatesServer()
    server.start(first_port, (127, 0, 0, 1), "first-token")
    assert server.is_running()
    server.stop()

    server.start(second_port, (127, 0, 0, 1), "second-token")
    try:
        assert server.is_running()
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as probe:
            with pytest.raises(OSError):
                probe.bind(("127.0.0.1", second_port))
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as probe:
            probe.bind(("127.0.0.1", first_port))
    finally:
        server.stop()
