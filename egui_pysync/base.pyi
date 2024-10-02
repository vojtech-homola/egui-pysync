from collections.abc import Buffer
from typing import Any

class StateServerBase:
    def __init__(self) -> None: ...
    def start(self) -> None: ...
    def stop(self) -> None: ...
    def is_running(self) -> bool: ...
    def is_connected(self) -> bool: ...
    def disconnect_client(self) -> None: ...
    def update(self, duration: float | None = None) -> None: ...

    # values ----------------------------------------------------------------------
    def set_value(self, value_id: int, value: Any, set_signal: bool, update: bool) -> None: ...
    def get_value(self, value_id: int) -> Any: ...

    # static ----------------------------------------------------------------------
    def set_static(self, value_id: int, value: Any, update: bool) -> None: ...
    def get_static(self, value_id: int) -> Any: ...

    # dignals ---------------------------------------------------------------------
    def get_signal_value(self, thread_id) -> tuple[int, tuple[Any, ...]]: ...
    def set_register_value(self, value_id: int, register: bool) -> None: ...

    # image -----------------------------------------------------------------------
    def set_image(self, value_id: int, image: Buffer, update: bool, rect: list[int] | None = None) -> None: ...
    def set_histogram(self, value_id: int, update: bool, histogram: Buffer | None = None) -> None: ...

    # dict ------------------------------------------------------------------------
    def set_dict(self, value_id: int, value: dict[Any, Any], update: bool) -> None: ...
    def get_dict(self, value_id: int) -> dict[Any, Any]: ...
    def set_dict_item(self, value_id: int, key: Any, value: Any, update: bool) -> None: ...
    def get_dict_item(self, value_id: int, key: Any) -> Any: ...
    def del_dict_item(self, value_id: int, key: Any, update: bool) -> None: ...
    def dict_len(self, value_id: int) -> int: ...

    # list ------------------------------------------------------------------------
    def set_list(self, value_id: int, value: list[Any], update: bool) -> None: ...
    def get_list(self, value_id: int) -> list[Any]: ...
    def set_list_item(self, value_id: int, idx: int, value: Any, update: bool) -> None: ...
    def get_list_item(self, value_id: int, idx: int) -> Any: ...
    def del_list_item(self, value_id: int, idx: int, update: bool) -> None: ...
    def add_list_item(self, value_id: int, value: Any, update: bool) -> None: ...
    def list_len(self, value_id: int) -> int: ...

    # graphs ----------------------------------------------------------------------
    def set_graph(self, value_id: int, graph: Buffer, update: bool) -> None: ...
    def add_graph_points(self, value_id: int, points: Buffer, update: bool) -> None: ...
    def clear_graph(self, value_id: int, update: bool) -> None: ...
