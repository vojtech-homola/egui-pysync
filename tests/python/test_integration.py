"""Run the same native client scenarios as the generated Rust server tests."""

import subprocess

import numpy as np
import pytest
from egui_states_test_bindings import StatesServer
from .conftest import _free_port


def configure(states):
    """Configure the test protocol; callback results are visible to the probe."""
    st = states.integration
    st.value.set(7)
    st.items.set([1, 2])
    st.cached.set(np.array([8, 9], dtype=np.uint8), cache=True)
    st.cached_multi[91].set(np.array([500, 600], dtype=np.uint16), cache=True)
    st.value.connect(lambda value: st.callback_value.set(value, update=True))

    def command(number):
        if number == 1:
            st.value.set(21, update=True)
        elif number == 2:
            st.items.add_item(st.value.get(), update=True)
            st.map.set_item(9, 900, update=True)
        elif number == 3:
            st.take.set("payload", update=True)
            st.empty.set(update=True)
            st.data.set(np.array([0, 1, 255], dtype=np.uint8), update=True)
            st.multi[7].set(np.array([100, 200], dtype=np.uint16), update=True)
            st.multi[42].set(np.array([], dtype=np.uint16), update=True)
        elif number == 4:
            st.take.set("", update=True)
            st.data.set(np.array([], dtype=np.uint8), update=True)
        elif number in {10, 11, 12, 13}:

            def send(second):
                if number == 10:
                    st.take.set("" if second else "first", blocking=True, update=True)
                elif number == 11:
                    st.empty.set(blocking=True, update=True)
                elif number == 12:
                    st.data.set(np.array([] if second else [1, 2], dtype=np.uint8), blocking=True, update=True)
                else:
                    st.multi[73].set(
                        np.array([] if second else [100, 200], dtype=np.uint16), blocking=True, update=True
                    )

            send(False)
            st.phase.set(1, update=True)
            send(True)
            st.phase.set(2, update=True)
        elif number == 99:
            st.phase.set(99, update=True)
        else:
            raise AssertionError(f"unknown command: {number}")

    st.command.connect(command)


@pytest.mark.parametrize(
    "scenario", ["sync", "takes", "blocking-value", "blocking-empty", "blocking-data", "blocking-multi", "cache"]
)
def test_python_server_with_native_client(request, scenario):
    """Verify the Python server against a shared native client scenario."""
    errors = []
    server = StatesServer(error_handler=errors.append)
    configure(server.states)
    port = _free_port()
    try:
        server.start(port, (127, 0, 0, 1))
        result = subprocess.run(
            [str(request.config._client_probe), str(port), scenario],
            check=False,
            capture_output=True,
            text=True,
            timeout=30,
        )
        assert result.returncode == 0, f"{scenario}:\n{result.stdout}\n{result.stderr}"
    finally:
        server.stop()
        assert not errors, f"Unexpected callback errors: {errors!r}"
