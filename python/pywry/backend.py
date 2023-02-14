import os
import sys

from pywry import PyWry


def start_backend(debug: bool = False):
    """Start the backend."""
    try:
        import ctypes  # pylint: disable=import-outside-toplevel

        # We need to set an app id so that the taskbar icon is correct on Windows
        ctypes.windll.shell32.SetCurrentProcessExplicitAppUserModelID("openbb")
    except (AttributeError, ImportError, OSError):
        pass
    backend = PyWry()
    backend.base.start(debug)


if __name__ == "__main__":
    sys_args = sys.argv[1:]
    sys_args = [arg.lower() for arg in sys_args]

    if "-start" in sys_args or sys.platform == "darwin":
        debug = "-debug" in sys_args or os.environ.get("DEBUG_MODE", "False").lower() == "true"  # noqa
        start_backend(debug)
