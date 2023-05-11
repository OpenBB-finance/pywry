import os
import sys

import setproctitle

from pywry import PyWry

__all__ = ["start_backend"]


def start_backend(headless: bool = False, debug: bool = False) -> None:
    """Start the backend."""
    try:
        proc_title = os.environ.get("PYWRY_PROCESS_NAME", "PyWry").replace(" ", "")
        setproctitle.setproctitle(proc_title)
        import ctypes  # pylint: disable=import-outside-toplevel

        # We need to set an app id so that the taskbar icon is correct on Windows
        ctypes.windll.shell32.SetCurrentProcessExplicitAppUserModelID("openbb")
    except (AttributeError, ImportError, OSError):
        pass

    backend = PyWry()
    if headless:
        return backend.base.start_headless(debug)

    backend.base.start(debug)


if __name__ == "__main__":
    sys_args = sys.argv[1:]
    sys_args = [arg.lower() for arg in sys_args]

    if "--start" in sys_args or sys.platform == "darwin":
        headless = "--headless" in sys_args
        debug = (
            "--debug" in sys_args
            or os.environ.get("DEBUG_MODE", "False").lower() == "true"
        )  # noqa
        start_backend(headless=headless, debug=debug)
