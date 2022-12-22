import asyncio
import json
import threading
from multiprocessing import Process
from typing import List, Optional

from pywry import pywry
from websockets.client import connect


class PyWry:
    """This class handles the wry functionality, by spinning up a rust program that
    listens to websockets and shows windows with provided HTML.
    """

    def __new__(cls):
        "Makes the class a 'singleton' by only allowing one instance at a time"
        if not hasattr(cls, "instance"):
            cls.instance = super().__new__(cls)
        return cls.instance

    def __init__(self, max_retries: int = 30):
        self.max_retries = max_retries

        self.outgoing: List[str] = []
        self.init_engine: List[str] = []
        self.started = False
        self.daemon = False
        self.base = pywry.WindowManager()

        self.runner: Optional[Process] = Process(
            target=start_backend, daemon=self.daemon
        )
        self.thread: Optional[threading.Thread] = None

        port = self.get_clean_port()
        self.url = f"ws://127.0.0.1:{port}"

    def send_html(self, html: str, title: str = ""):
        """Send html to backend.

        Parameters
        ----------
        html : str
            HTML to send to backend.
        title : str, optional
            Title to display in the window, by default ""
        """
        self.check_backend()
        message = json.dumps({"html": html, "title": title})
        self.outgoing.append(message)

    def check_backend(self):
        """Check if the backend is running."""

        retries = 0
        if retries == self.max_retries:
            # If the backend is not running and we have tried to connect
            # max_retries times, raise an error
            raise ConnectionError("Exceeded max retries")
        try:
            if not self.started:
                self.handle_start()
                self.started = True

            if self.thread and not self.thread.is_alive():
                self.start()

        except ConnectionRefusedError:
            self.started = False
            retries += 1
            self.check_backend()

    async def send_test(self):
        """Send data to the backend."""
        async with connect(self.url) as websocket:
            await websocket.send("<test>")

    def get_clean_port(self) -> str:
        port = self.base.get_port()
        if port == 0:
            raise ConnectionError("Could not connect to a port")
        return str(port)

    def handle_start(self):
        try:
            self.runner.start()
            self.started = True
        except Exception as e:
            print(e)
            self.started = False

    async def connect(self):
        """Connects to backend and maintains the connection until main thread is closed."""
        try:
            async with connect(
                self.url,
                open_timeout=6,
                timeout=1,
                ssl=None,
            ) as websocket:
                if self.init_engine:
                    # if there is data in the init_engine list,
                    # we send it to the backend and clear the list
                    for msg in self.init_engine:
                        await websocket.send(msg)
                    self.init_engine = []

                while True:
                    if self.outgoing:
                        data = self.outgoing.pop(0)
                        self.init_engine.append(data)

                        await websocket.send(data)
                        self.init_engine = []

                    await asyncio.sleep(0.1)

        except ConnectionRefusedError:
            await self.connect()
        except Exception:
            await self.connect()

    def start(self, daemon: bool = False):
        """Connect to backend in a separate thread."""
        self.check_backend()
        self.daemon = daemon

        self.thread = threading.Thread(
            target=asyncio.run, args=(self.connect(),), daemon=daemon
        )
        self.thread.start()


def start_backend():
    """Start the backend."""
    try:
        import ctypes  # pylint: disable=import-outside-toplevel

        # We need to set an app id so that the taskbar icon is correct on Windows
        ctypes.windll.shell32.SetCurrentProcessExplicitAppUserModelID("openbb")
    except (AttributeError, ImportError, OSError):
        pass
    backend = PyWry()
    backend.base.start()
