import atexit
import base64
import json
import sys
from multiprocessing import current_process
from pathlib import Path

import plotly.graph_objects as go
from pywry import PyWry

BACKEND = None


# We create a custom backend for PyWry that will be used to display the figure in the
# window. This backend is a singleton, so it will only be created once, and will be
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
        self.plotly_html = Path(__file__).parent / "plotly.html"
        atexit.register(self.close)

    def get_plotly_html(self) -> Path:
        """Get the path to the Plotly HTML file."""
        if self.plotly_html.exists():
            return self.plotly_html

        self.max_retries = 0  # pylint: disable=W0201
        raise FileNotFoundError(f"Plotly HTML file not found at {self.plotly_html}.")

    def send_figure(self, fig: go.Figure):
        """Send a Plotly figure to the backend.

        Parameters
        ----------
        fig : go.Figure
            Plotly figure to send to backend.
        """
        self.check_backend()
        title = fig.layout.title.text if fig.layout.title else "Plotly Figure"

        json_data = json.loads(fig.to_json())

        outgoing = dict(
            html=self.get_plotly_html(),
            json_data=json_data,
            title=title,
        )
        self.send_outgoing(outgoing)

    def figure_write_image(
        self,
        fig: go.Figure,
        img_format: str = "png",
        scale: int = 1,
        timeout: int = 5,
    ) -> bytes:
        """Convert a Plotly figure to an image.

        Parameters
        ----------
        fig : go.Figure
            Plotly figure to convert to image.
        format : str, optional
            Image format, by default "png"
        scale : int, optional
            Image scale, by default 1
        timeout : int, optional
            Timeout for receiving the image, by default 5
        """
        self.check_backend()

        json_data = json.loads(fig.to_json())
        json_data["scale"] = scale
        json_data["format"] = img_format

        self.send_outgoing(dict(json_data=json_data))

        incoming = self.recv.get(timeout=timeout)
        if incoming.get("result", None):
            # SVG images are already in the correct format
            return (
                incoming.get("result", "").encode("utf-8")
                if img_format == "svg"
                else base64.b64decode(incoming.get("result"))
            )
        else:
            raise RuntimeError("Error converting figure to image.")

    def start(self, debug: bool = False, headless: bool = False):
        """Start the backend WindowManager process.

        Parameters
        ----------
        debug : bool, optional
            Whether to start in debug mode to see the output and
            enable dev tools in the browser, by default False
        headless : bool, optional
            Whether to start in headless mode, by default False
            (this is used for figure_write_image to prevent the
            browser from opening)
        """
        if self.isatty:
            super().start(debug, headless)


def pywry_backend(daemon: bool = True) -> Backend:
    """Get the backend."""
    global BACKEND  # pylint: disable=W0603 # noqa
    if BACKEND is None:
        BACKEND = Backend(daemon)
    return BACKEND
