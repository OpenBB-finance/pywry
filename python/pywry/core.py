import asyncio
import json
from multiprocessing import Process
from pathlib import Path

from plotly import graph_objects as go
from pywry import pywry
from websockets.client import connect


class PlotsBackendError(Exception):
    """Base class for exceptions in this module."""

    def __init__(self):
        self.message = "We've encountered an error while trying to start the Plots backend. Please try again."
        super().__init__(self.message)


def process_plotly_figure(figure: go.Figure):

    style = figure.layout.template.layout.mapbox.style

    if (bg_color := "#111111" if style == "dark" else "white") == "#111111":
        figure.update_layout(
            newshape_line_color="gold",
            modebar=dict(
                orientation="v", bgcolor=bg_color, color="gold", activecolor="#d1030d"
            ),
        )
    figure.update_layout(dragmode="pan")

    return figure


class PyWry:
    base = pywry.WindowManager()

    def __new__(cls):
        # We only want to create one instance of the class
        # so we use the __new__ method to check if the instance already exists
        if not hasattr(cls, "instance"):
            cls.instance = super().__new__(cls)
        return cls.instance

    def __init__(self, max_retries: int = 30):
        self.max_retries = max_retries
        self.plotly = str(self.get_plotly_html()) or ""

        self.port = self.get_clean_port()
        self.runner: Process = Process(target=self.handle_start, daemon=True)
        self.runner.start()
        self.url = "ws://127.0.0.1:" + str(self.port)

        try:
            loop = asyncio.get_running_loop()
        except RuntimeError:
            loop = asyncio.new_event_loop()
            asyncio.set_event_loop(loop)

        self.loop = loop

    def __del__(self):
        self.runner.terminate()

    def send_fig(self, fig: go.Figure):
        """Send figure to qt_backend.

        Parameters
        ----------
        fig : go.Figure
            Plotly figure to send to qt_backend.
        """
        self.check_backend()

        fig = process_plotly_figure(fig)
        self.loop.create_task(
            self.send(json.dumps({"plotly": fig.to_dict(), "html": self.plotly}))
        )

    def send_html(self, html: str, title: str = ""):
        """Send html to qt_backend.

        Parameters
        ----------
        html : str
            HTML to send to qt_backend.
        """
        self.check_backend()
        self.loop.create_task(self.send(json.dumps({"html": html, "title": title})))

    def check_backend(self):
        """Check if the backend is running."""
        if self.max_retries == 0:
            # If the backend is not running and we have tried to connect
            # max_retries times, we raise an error as a fallback to prevent
            # the user from not seeing any plots
            raise PlotsBackendError
        try:
            self.loop.run_until_complete(self.send())
        except ConnectionRefusedError:
            self.max_retries -= 1
            self.check_backend()

    async def send(self, data: str = "<test>"):
        """Send data to the backend."""
        async with connect(self.url) as websocket:
            await asyncio.sleep(1)
            await websocket.send(data)

    def get_clean_port(self) -> str:
        port = self.base.get_port()
        print("Port:", port)
        if port == 0:
            raise ConnectionError("Could not connect to a port")
        else:
            return str(port)

    def get_plotly_html(self) -> Path:
        return Path(__file__).parent.resolve() / "assets" / "plotly.html"

    def handle_start(self):
        self.base.start()
