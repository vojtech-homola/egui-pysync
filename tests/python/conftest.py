"""Generate independent bindings before test-module collection."""

import json
import os
from pathlib import Path
import socket
import subprocess
import sys
import threading
import time

import pytest

ROOT = Path(__file__).resolve().parents[2]
GENERATED = ROOT / "tests" / "generated"


def pytest_configure(config):
    command = ["cargo", "build", "-p", "egui_states_test_support", "--bins"]
    try:
        subprocess.run(command, cwd=ROOT, check=True, capture_output=True, text=True, timeout=600)
        # Run every session: Cargo may be fresh even if Python artifacts were removed.
        metadata = json.loads(
            subprocess.check_output(
                ["cargo", "metadata", "--no-deps", "--format-version", "1"],
                cwd=ROOT,
                text=True,
            )
        )
        target = Path(metadata["target_directory"]) / "debug"
        suffix = ".exe" if os.name == "nt" else ""
        subprocess.run(
            [str(target / f"prepare-test-bindings{suffix}")],
            cwd=ROOT,
            check=True,
            capture_output=True,
            text=True,
            timeout=60,
        )
        config._client_probe = target / f"client-probe{suffix}"
    except (subprocess.CalledProcessError, subprocess.TimeoutExpired, OSError) as error:
        raise pytest.UsageError(
            f"Test fixture preparation failed. Run {' '.join(command)} from {ROOT}.\n"
            f"{getattr(error, 'stdout', '')}\n{getattr(error, 'stderr', '')}\n{error}"
        ) from error
    sys.path.insert(0, str(GENERATED))


def _free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        sock.listen(1)
        return int(sock.getsockname()[1])


def _wait_until(predicate, timeout: float = 1.0) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if predicate():
            return
        time.sleep(0.01)
    assert predicate()


def _wait_event(event: threading.Event, timeout: float = 1.0) -> None:
    assert event.wait(timeout), "timed out waiting for callback"


@pytest.fixture
def server_bundle():
    from egui_states_test_bindings import StatesServer

    errors = []
    server = StatesServer(error_handler=errors.append)
    server.start(_free_port(), (127, 0, 0, 1))
    try:
        yield server, server.states, errors
    finally:
        server.stop()
        assert not errors, f"Unexpected callback errors: {errors!r}"
