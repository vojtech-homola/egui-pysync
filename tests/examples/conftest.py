"""Opt-in example bindings and native smoke probe preparation."""

import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import socket
import time

import pytest

ROOT = Path(__file__).resolve().parents[2]


def pytest_configure(config):
    """Build the example binaries and generate their Python bindings."""
    command = [
        "cargo",
        "build",
        "-p",
        "counter_gui",
        "-p",
        "showcase_gui",
        "-p",
        "example_smoke_tests",
        "-p",
        "showcase_server",
        "-p",
        "counter_server",
        "--bins",
    ]
    try:
        subprocess.run(command, cwd=ROOT, check=True, capture_output=True, text=True, timeout=600)
        metadata = json.loads(
            subprocess.check_output(
                ["cargo", "metadata", "--no-deps", "--format-version", "1"],
                cwd=ROOT,
                text=True,
            )
        )
        config._example_probe = (
            Path(metadata["target_directory"]) / "debug" / ("example-probe.exe" if os.name == "nt" else "example-probe")
        )
    except (subprocess.CalledProcessError, subprocess.TimeoutExpired, OSError) as error:
        raise pytest.UsageError(
            f"Example preparation failed: {error}\n{getattr(error, 'stdout', '')}\n{getattr(error, 'stderr', '')}"
        ) from error


def free_port():
    """Find an available port on the loopback interface."""
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def wait(predicate):
    """Wait up to five seconds for an example callback's observable result."""
    deadline = time.monotonic() + 5
    while not predicate():
        assert time.monotonic() < deadline, "timed out waiting for example callback"
        time.sleep(0.005)


@pytest.fixture(params=["showcase", "counter"])
def example_server(request):
    """Yield a configured example server and check callback errors on teardown."""
    name = request.param
    directory = ROOT / "example" / name / "python"
    sys.path.insert(0, str(directory))
    try:
        spec = importlib.util.spec_from_file_location(f"{name}_example", directory / "run.py")
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        errors = []
        server = module.setup_server(error_handler=errors.append)
        assert not server.is_running(), "Import/setup must not start a server"
        try:
            yield name, module, server
        finally:
            server.stop()
            assert not errors, f"Unexpected example callback errors: {errors!r}"
    finally:
        sys.path.remove(str(directory))
