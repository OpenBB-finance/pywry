import asyncio
import atexit
import json
import os
import sys
import threading
from subprocess import PIPE
from typing import List, Optional

import psutil
from pywry import pywry
from websockets.client import connect


class PyWry:
    """This class handles the wry functionality, by spinning up a rust program that
    listens to websockets and shows windows with provided HTML.
    """

    def __new__(cls, *args, **kwargs):  # pylint: disable=unused-argument
        "Makes the class a 'singleton' by only allowing one instance at a time"
        if not hasattr(cls, "instance"):
            cls.instance = super().__new__(cls)
        return cls.instance

    def __init__(self, daemon: bool = True, max_retries: int = 30):
        self.max_retries = max_retries

        self.outgoing: List[str] = []
        self.init_engine: List[str] = []
        self.started = False
        self.daemon = daemon
        self.base = pywry.WindowManager()
        self.runner: Optional[psutil.Popen] = None
        self.procs: List[psutil.Process] = [psutil.Process(os.getpid())]
        self.thread: Optional[threading.Thread] = None

        port = self.get_clean_port()
        self.url = f"ws://localhost:{port}"

        atexit.register(self.close)

    def __del__(self):
        self.close()

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

        if self.max_retries == 0:
            # If the backend is not running and we have tried to connect
            # max_retries times, raise an error
            raise ConnectionError("Exceeded max retries")
        try:
            if not self.runner or not self.runner.is_running():
                self.handle_start()

            if self.thread and not self.thread.is_alive():
                self.start()

        except ConnectionRefusedError:
            self.started = False
            self.max_retries -= 1
            self.check_backend()

    async def send_test(self):
        """Send data to the backend."""
        async with connect(self.url) as websocket:
            await websocket.send("<test>")

    def get_clean_port(self) -> str:
        port = self.base.get_port()
        if port == 0:
            raise ConnectionError("Could not connect to a port")
        return port

    def handle_start(self):
        try:
            if self.runner and self.runner.is_running():
                self.procs.remove(self.runner)
                self.runner.terminate()
                self.runner.wait()

            self.runner = psutil.Popen(
                [
                    sys.executable,
                    "-c",
                    "from pywry.core import start_backend; start_backend()",
                ],
                env=os.environ,
                stderr=PIPE,
            )
            self.procs.append(self.runner)
        except Exception as e:
            raise ConnectionRefusedError("Could not start backend") from e

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

        except Exception as exc:
            if self.max_retries == 0:
                raise ConnectionError("Exceed max retries") from exc
            self.max_retries -= 1
            await asyncio.sleep(1)
            await self.connect()

    def start(self):
        """Creates a websocket connection that remains open"""
        self.check_backend()

        self.thread = threading.Thread(
            target=asyncio.run, args=(self.connect(),), daemon=self.daemon
        )
        self.thread.start()

    def close(self):
        """Close the backend."""
        if self.runner and self.runner.is_running():
            self.runner.terminate()

        _, alive = psutil.wait_procs(self.procs, timeout=3)
        for process in alive:
            process.kill()

        if self.thread and self.thread.is_alive():
            self.thread.join()


def start_backend():
    """Start the backend."""
    try:
        import ctypes  # pylint: disable=import-outside-toplevel

        # We need to set an app id so that the taskbar icon is correct on Windows
        ctypes.windll.shell32.SetCurrentProcessExplicitAppUserModelID("openbb")
    except (AttributeError, ImportError, OSError):
        pass
    backend = PyWry()
    backend.base.start(bool(os.environ.get("DEBUG_MODE", False)))
