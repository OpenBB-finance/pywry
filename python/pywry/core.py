import asyncio
import atexit
import json
import os
import subprocess
import sys
import threading
import traceback
from asyncio.exceptions import IncompleteReadError
from typing import List, Optional

import psutil
from pywry import pywry
from websockets.client import connect
from websockets.exceptions import ConnectionClosedError


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

    def __init__(self, daemon: bool = True, max_retries: int = 5):
        self.max_retries = max_retries

        self.outgoing: List[str] = []
        self.init_engine: List[str] = []
        self.daemon = daemon
        self.started = False
        self.debug = False
        self.shell = False
        self.base = pywry.WindowManager()

        self.runner: Optional[psutil.Popen] = None
        self.procs: List[psutil.Process] = []
        self.thread: Optional[threading.Thread] = None

        port = self.get_clean_port()
        self.url = f"ws://localhost:{port}"

    def __del__(self):
        if self.started:
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
        self.check_backend()
        message = json.dumps(
            {"html_str": html_str, "html_path": html_path, "title": title}
        )
        self.outgoing.append(message)

    def print_debug(self):
        """Print debug messages from the backend."""
        if self.debug:
            traceback.print_exc()

    def check_backend(self):
        """Check if the backend is running."""

        if self.max_retries == 0:
            # If the backend is not running and we have tried to connect
            # max_retries times, raise an error
            raise BackendFailedToStart("Exceededed max retries")
        try:
            if not self.runner or not self.runner.is_running():
                self.handle_start()

            if self.thread and not self.thread.is_alive():
                self.start()

        except (ConnectionRefusedError, RuntimeError):
            self.print_debug()
            self.handle_start()
        except psutil.ZombieProcess:
            self.print_debug()
            self.hard_restart()

    def hard_restart(self):
        """Hard restart the backend."""
        self.max_retries -= 1
        self.close(True)
        self.handle_start()
        self.start()

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

    def handle_start(self):
        """Start the backend."""
        try:
            if self.runner and self.runner.is_running():
                self.runner.terminate()
                self.runner.wait()
                self.procs.remove(self.runner)

            port = self.get_clean_port()
            self.url = f"ws://localhost:{port}"

            kwargs = {}
            if not hasattr(sys, "frozen"):
                cmd = [sys.executable, "-m", "pywry.backend", "-start"]
                kwargs = {"stderr": subprocess.PIPE}
            else:
                pywrypath = os.path.join(
                    # pylint: disable=E1101,W0212
                    sys._MEIPASS,
                    "pywry_backend",
                )
                cmd = [
                    f"'{pywrypath}'",
                    "-start",
                ]
                if sys.platform == "darwin":
                    cmd.pop(-1)

                kwargs = {
                    "stdout": subprocess.PIPE,
                    "stderr": subprocess.STDOUT,
                    "stdin": subprocess.PIPE,
                }
                self.shell = True

            if self.debug and sys.platform != "darwin":
                cmd.append("-debug")

            self.runner = psutil.Popen(
                cmd, env=os.environ, shell=self.shell, **kwargs  # nosec
            )
            self.procs.append(self.runner)

        except psutil.ZombieProcess:
            self.print_debug()
            self.hard_restart()

        except Exception as proc_err:
            raise BackendFailedToStart("Could not start backend") from proc_err

    async def connect(self):
        """Connects to backend and maintains the connection until main thread is closed."""
        # wait for the backend to start
        await asyncio.sleep(3)
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
                        self.init_engine.append(data)

                        await websocket.send(data)
                        self.init_engine = []

                    await asyncio.sleep(0.1)

        except (IncompleteReadError, ConnectionClosedError) as conn_err:
            self.print_debug()
            if self.max_retries == 0:
                raise BackendFailedToStart("Exceeded max retries") from conn_err
            self.hard_restart()
            await asyncio.sleep(2)
            await self.connect()

        except (ConnectionRefusedError, ConnectionResetError) as exc:
            self.print_debug()
            self.check_backend()

            if self.max_retries == 0:
                raise BackendFailedToStart("Exceeded max retries") from exc
            self.max_retries -= 1

            await asyncio.sleep(2)
            await self.connect()

    def start(self, debug: bool = False):
        """Creates a websocket connection that remains open"""
        self.debug = debug
        self.check_backend()

        try:
            if self.thread and self.thread.is_alive():
                self.thread.join()
        except RuntimeError:
            self.thread = None

        self.thread = threading.Thread(
            target=asyncio.run, args=(self.connect(),), daemon=self.daemon
        )
        self.thread.start()

        self.started = True

        if psutil.Process(os.getpid()) not in self.procs:
            self.procs.append(psutil.Process(os.getpid()))
            atexit.register(self.close)

    def close(self, reset: bool = False):
        """Close the backend."""
        if self.runner and self.runner.is_running():
            self.runner.terminate()
            self.runner.wait()
            self.procs.remove(self.runner)

        if not reset:
            _, alive = psutil.wait_procs(self.procs, timeout=3)
            for process in alive:
                process.kill()

            if self.thread and self.thread.is_alive():
                self.thread.join()

        self.runner = None
        self.thread = None
