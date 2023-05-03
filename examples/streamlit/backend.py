import atexit
import json
import sys
from multiprocessing import current_process
from typing import Optional

from pywry import PyWry

BACKEND = None


# We create a custom backend for PyWry that will be used to send streamlit URLs to the
# browser. This backend is a singleton, so it will only be created once, and will be
# shared across all modules that import it.
class Backend(PyWry):
    """Custom backend for PyWry."""

    def __new__(cls, *args, **kwargs):  # pylint: disable=W0613
        """Create a singleton instance of the backend."""
        if not hasattr(cls, "instance"):
            cls.instance = super().__new__(cls)  # pylint: disable=E1120
        return cls.instance

    def __init__(
        self,
        daemon: bool = True,
        max_retries: int = 30,
        proc_name: str = "PyWry Backend",
    ):
        super().__init__(daemon=daemon, max_retries=max_retries, proc_name=proc_name)
        self.isatty = sys.stdin.isatty() and current_process().name == "MainProcess"
        atexit.register(self.close)

    def send_url(
        self,
        url: str,
        title: str = "",
        width: Optional[int] = None,
        height: Optional[int] = None,
    ):
        """Send a URL to the backend to be displayed in a window.

        Parameters
        ----------
        url : str
            URL to display in the window.
        title : str, optional
            Title to display in the window, by default ""
        width : int, optional
            Width of the window, by default 1200
        height : int, optional
            Height of the window, by default 800
        """
        self.loop.run_until_complete(self.check_backend())
        script = f"""
        <script>
            window.location.replace("{url}");
        </script>
        """
        message = json.dumps(
            {
                "html_str": script,
                "width": width,
                "height": height,
                "title": title,
            }
        )
        self.outgoing.append(message)

    def start(self, debug: bool = False):
        """Start the backend WindowManager process.

        Parameters
        ----------
        debug : bool, optional
            Whether to start in debug mode to see the output and enable dev tools in the
            browser, by default False
        """
        if self.isatty:
            super().start(debug)

    def close(self, reset: bool = False):
        """Close the backend.

        Parameters
        ----------
        reset : bool, optional
            Whether to reset the backend, by default False
        """
        if reset:
            self.max_retries = 50  # pylint: disable=W0201

        super().close(reset)

    async def check_backend(self):
        """Override to check if isatty."""
        if self.isatty:
            await super().check_backend()


def pywry_backend(daemon: bool = True) -> Backend:
    """Get the backend."""
    global BACKEND  # pylint: disable=W0603 # noqa
    if BACKEND is None:
        BACKEND = Backend(daemon)
    return BACKEND
