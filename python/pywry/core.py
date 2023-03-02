import asyncio
import atexit
import json
import os
import subprocess
import sys
import threading
import traceback
from asyncio.exceptions import IncompleteReadError
from pathlib import Path
from typing import List, Optional

import psutil
from websockets.client import connect
from websockets.exceptions import ConnectionClosedError

from pywry import pywry


class BackendFailedToStart(Exception):
    """Raised when the backend fails to start"""

    def __init__(self, message: str):
        super().__init__(message)


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
        self.daemon = daemon
        self.debug = False
        self.shell = False
        self.base = pywry.WindowManager()

        try:
            self.loop: asyncio.AbstractEventLoop = asyncio.get_event_loop()
        except RuntimeError:
            self.loop = asyncio.new_event_loop()
            asyncio.set_event_loop(self.loop)

        self.runner: Optional[psutil.Popen] = None
        self.procs: List[psutil.Process] = []
        self.thread: Optional[threading.Thread] = None

        self.lock: threading.Lock = threading.Lock()
        self._is_started: asyncio.Event = asyncio.Event()
        self._is_closed: asyncio.Event = asyncio.Event()
        self._is_closed.set()

        port = self.get_clean_port()
        self.url = f"ws://localhost:{port}"

        atexit.register(self.close)

    def __del__(self):
        if self._is_started.is_set():
            self.close()
        else:
            self.procs.clear()

    def send_html(self, html_str: str = "", html_path: str = "", title: str = ""):
        """Send html to backend.

        Parameters
        ----------
        html_str : str
            HTML string to send to backend.
        html_path : str, optional
            Path to html file to send to backend, by default ""
        title : str, optional
            Title to display in the window, by default ""
        """
        self.loop.run_until_complete(self.check_backend())
        message = json.dumps(
            {"html_str": html_str, "html_path": html_path, "title": title}
        )
        self.outgoing.append(message)

    def print_debug(self):
        """Print debug messages from the backend."""
        if self.debug:
            traceback.print_exc()

    async def check_backend(self):
        """Check if the backend is running."""

        if self.max_retries == 0:
            # If the backend is not running and we have tried to connect
            # max_retries times, raise an error
            raise BackendFailedToStart("Exceededed max retries")
        try:
            if not self.runner or not self.runner.is_running():
                await self.handle_start()

            if self.thread and not self.thread.is_alive():
                self.start()

        except (ConnectionRefusedError, RuntimeError, psutil.ZombieProcess):
            self.print_debug()
            await self.handle_start()

    async def send_test(self):
        """Send data to the backend."""
        async with connect(self.url) as websocket:
            await websocket.send("<test>")

    def get_clean_port(self) -> str:
        """Get a clean port to use for the backend."""
        port = self.base.get_port()
        if port == 0:
            raise ConnectionError("Could not connect to a port")
        return port

    async def handle_start(self):
        """Start the backend."""
        try:
            port = self.get_clean_port()
            if self.runner and self.runner.is_running():
                _, alive = psutil.wait_procs([self.runner], timeout=2)
                if alive:
                    self.procs.remove(self.runner)
                    self.runner.terminate()
                    self.runner.wait(1)

                with self.lock:
                    self.runner = None
                    self._is_started.clear()
                    self._is_closed.set()
                    self.url = f"ws://localhost:{port}"

            kwargs = {}
            if not hasattr(sys, "frozen"):
                cmd = [sys.executable, "-m", "pywry.backend", "-start"]
                kwargs = {"stderr": subprocess.PIPE}
            else:
                # pylint: disable=E1101,W0212
                pywrypath = (Path(sys._MEIPASS) / "OpenBBPlotsBackend").resolve()
                if sys.platform == "win32":
                    cmd = f"{pywrypath} -start{' -debug' if self.debug else ''}"
                if sys.platform == "darwin":
                    cmd = f"'{pywrypath}'"

                kwargs = {
                    "stdout": subprocess.PIPE,
                    "stderr": subprocess.STDOUT,
                    "stdin": subprocess.PIPE,
                }
                self.shell = True

            self.runner = psutil.Popen(
                cmd,
                env=os.environ,
                shell=self.shell,
                **kwargs,  # nosec
            )
            self.procs.append(self.runner)

            await asyncio.sleep(2)
            with self.lock:
                self._is_started.set()
                self._is_closed.clear()

        except psutil.ZombieProcess:
            self.print_debug()
            await self.handle_start()

        except Exception as proc_err:
            raise BackendFailedToStart("Could not start backend") from proc_err

    async def connect(self):
        """Connects to backend and maintains the connection until main thread is closed."""  # noqa: E501

        # We wait for the backend to start
        while not self._is_started.is_set():
            await asyncio.sleep(0.1)

        await asyncio.sleep(1)

        try:
            async with connect(
                self.url,
                open_timeout=6,
                timeout=1,
                ping_interval=None,
                ping_timeout=None,
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
                        with self.lock:
                            self.init_engine.append(data)

                        await websocket.send(data)
                        self.init_engine = []

                    await asyncio.sleep(0.1)

        except (IncompleteReadError, ConnectionClosedError) as conn_err:
            self.print_debug()
            with self.lock:
                self._is_started.clear()
                self._is_closed.set()
            await self.handle_start()
            if self.max_retries == 0:
                raise BackendFailedToStart("Exceeded max retries") from conn_err
            await asyncio.sleep(2)
            await self.connect()

        except (ConnectionRefusedError, ConnectionResetError) as exc:
            self.print_debug()
            with self.lock:
                self._is_started.clear()
                self._is_closed.set()
            await self.handle_start()
            if self.max_retries == 0:
                raise BackendFailedToStart("Exceeded max retries") from exc
            self.max_retries -= 1

            await asyncio.sleep(1)
            await self.connect()

    def run(self):
        """Run the backend."""
        asyncio.run(self.connect())

    def start(self, debug: bool = False):
        """Creates a websocket connection that remains open"""
        self.debug = debug

        self.thread = threading.Thread(target=self.run, daemon=self.daemon)
        self.thread.start()

        self.loop.run_until_complete(self.check_backend())

    def close(self, reset: bool = False):
        """Close the backend."""
        if self.runner and self.runner.is_running():
            self.procs.remove(self.runner)
            self.runner.terminate()
            self.runner.wait()

        if not reset:
            for process in [p for p in self.procs if p.is_running()]:
                for child in process.children(recursive=True):
                    child.kill()
