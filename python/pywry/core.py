import asyncio
import json
from multiprocessing import Process
from pathlib import Path

from pywry import pywry
from websockets.client import connect


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
        if self.runner:
            self.runner.terminate()

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
            raise ConnectionError("Exceeded max retries")
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
