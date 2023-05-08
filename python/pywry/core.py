import asyncio
import atexit
import json
import os
import re
import subprocess
import sys
import threading
import traceback
from asyncio.exceptions import CancelledError, IncompleteReadError, TimeoutError
from pathlib import Path
from typing import List, Optional, Union

import setproctitle

from pywry import pywry

__all__ = ["PyWry", "BackendFailedToStart"]

AsyncioException = (
    CancelledError,
    IncompleteReadError,
    TimeoutError,
    ConnectionResetError,
)


ACCEPTED_KEYS_TYPES = {
    "html_str": str,
    "html_path": (str, Path),
    "title": str,
    "icon": (str, Path),
    "json_data": (dict, str),
    "height": int,
    "width": int,
    "download_path": (str, Path),
    "export_image": (str, Path),
}


class BackendFailedToStart(Exception):
    """Raised when the backend fails to start"""

    def __init__(self, message: str):
        super().__init__(message)


class PyWry:
    """This class handles the wry functionality, by spinning up a rust program that
    listens to pipes and shows windows with provided html and json data.
    """

    __version__ = pywry.__version__

    def __new__(cls, *args, **kwargs):  # pylint: disable=unused-argument
        "Makes the class a 'singleton' by only allowing one instance at a time"
        if not hasattr(cls, "instance"):
            cls.instance = super().__new__(cls)
        return cls.instance

    def __init__(
        self,
        daemon: bool = True,
        max_retries: int = 30,
        proc_name: str = "PyWry",
    ):
        self.max_retries: int = max_retries
        self.proc_name: str = proc_name

        self.outgoing: List[str] = []
        self.init_engine: List[str] = []
        self.daemon: bool = daemon
        self.debug: bool = False
        self.shell: bool = False
        self.base = pywry.WindowManager()

        try:
            self.loop: asyncio.AbstractEventLoop = asyncio.get_event_loop()
        except RuntimeError:
            self.loop = asyncio.new_event_loop()
            asyncio.set_event_loop(self.loop)

        self.runner: Optional[asyncio.subprocess.Process] = None
        self.thread: Optional[threading.Thread] = None
        self.subprocess_loop: Optional[asyncio.AbstractEventLoop] = None

        self.lock: threading.Lock = threading.Lock()
        self._is_started: asyncio.Event = asyncio.Event()
        self._is_closed: asyncio.Event = asyncio.Event()
        self._is_closed.set()

        atexit.register(self.close)

    def __del__(self):
        if self._is_started.is_set():
            self.close()

    def send_html(
        self,
        html_str: Optional[str] = None,
        html_path: Optional[Union[str, Path]] = None,
        json_data: Optional[dict] = None,
        title: str = "",
        width: int = 800,
        height: int = 600,
        **kwargs,
    ):
        """Send html to backend.

        Parameters
        ----------
        html_str : str, optional
            HTML string to send to backend.
            If not provided, html_path must be provided, by default None
        html_path : str, optional
            Path to html file to send to backend, by default None
        json_data : dict, optional
            JSON data to send to backend, by default None
        title : str, optional
            Title to display in the window, by default ""
        width : int, optional
            Width of the window, by default 800
        height : int, optional
            Height of the window, by default 600
        """
        self.loop.run_until_complete(self.check_backend())

        kwargs.update(
            dict(
                html_str=html_str,
                html_path=html_path,
                json_data=json_data,
                title=title,
                width=width,
                height=height,
            )
        )

        self.send_outgoing(kwargs)

    def send_outgoing(self, outgoing: dict):
        """Send outgoing data to backend.

        Parameters
        ----------
        outgoing : dict
            Data to send to backend.
        """
        outgoing = self.check_kwargs(outgoing)
        self.outgoing.append(json.dumps(outgoing))

    def check_kwargs(self, kwargs: dict):
        """Check that the outgoing data is valid.

        Parameters
        ----------
        kwargs : dict
            Data to check.

        Returns
        -------
        dict
            Data that has been checked. Invalid data is removed.
            For example, if the user provides a path to a file that does not exist,
            the path is removed from the data. This is done to prevent errors when
            creating the window.

            Paths are converted to strings and resolved to their absolute path.
        """
        output = {}
        for key, value in [
            (key, value) for key, value in kwargs.items() if value is not None
        ]:
            try:
                if not ACCEPTED_KEYS_TYPES.get(key, None):
                    raise ValueError(f"Invalid key: {key}")
                if not isinstance(value, ACCEPTED_KEYS_TYPES[key]):
                    raise TypeError(
                        f"Invalid type for {key}. "
                        f"Expected {ACCEPTED_KEYS_TYPES[key]}, got {type(value)}"
                    )
                if isinstance(value, Path):
                    if value.is_file() and not value.exists() and key != "export_image":
                        raise FileNotFoundError(value)
                    value = str(value.resolve())
            except Exception:
                self.print_debug()
                continue

            output[key] = value

        return output

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
            if self.thread and not self.thread.is_alive():
                self.start()

        except RuntimeError:
            self.print_debug()

    async def handle_start(self):
        """Start the backend."""
        try:
            if self.runner:
                try:
                    self.subprocess_loop.call_soon_threadsafe(self.runner.terminate)
                    self.subprocess_loop.call_soon_threadsafe(self.runner.kill)
                except Exception:
                    pass

                with self.lock:
                    self.runner = None
                    self._is_started.clear()
                    self._is_closed.set()

            kwargs = dict(
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
            cmd = [sys.executable, "-m", "pywry.backend", "--start"]
            if self.debug:
                cmd.append("--debug")

            # for pyinstaller builds
            if hasattr(sys, "frozen"):
                # pylint: disable=E1101,W0212
                exec_name = os.environ.get("PYWRY_EXECUTABLE", "PyWry")
                pywrypath = (Path(sys._MEIPASS) / exec_name).resolve()
                cmd = f"{exec_name} --start{' --debug' if self.debug else ''}"
                if sys.platform == "darwin":
                    cmd = f"'{pywrypath}'"

                kwargs.update(dict(cwd=str(pywrypath.parent)))
                self.shell = True

            env = os.environ.copy()
            env["PYWRY_PROCESS_NAME"] = self.proc_name

            runner = asyncio.create_subprocess_exec(
                *cmd,
                env=env,
                **kwargs,
            )

            if self.shell:
                if isinstance(cmd, list):
                    cmd = " ".join(cmd)
                runner = asyncio.create_subprocess_shell(
                    cmd,
                    env=env,
                    **kwargs,
                )

            runner = await runner

            with self.lock:
                self.runner = runner
                self._is_started.set()
                self._is_closed.clear()

            setproctitle.setproctitle(self.proc_name)

            # Unix machines may need a little more time to start the backend
            if sys.platform != "win32":
                await asyncio.sleep(3)

        except Exception as proc_err:
            raise BackendFailedToStart("Could not start backend") from proc_err

    async def stdout_reader(self):
        """Read stdout from the backend."""
        try:
            while self._is_started.is_set():
                if data := (await self.runner.stdout.readline()).decode().strip():
                    print(data)

                await asyncio.sleep(0.1)

        except Exception as proc_err:
            await self.exception_handler(proc_err)

    async def stderr_reader(self):
        """Read stderr from the backend."""
        # Ignore some messages from the backend that can be confusing to users
        # these messages are not errors, but they are not useful either
        ignore_regex = r"(Wayland|Compositor|webkit_download|NeedDebuggerBreak)"
        try:
            while self._is_started.is_set():
                if data := (await self.runner.stderr.readline()).decode().strip():
                    if data and not re.search(ignore_regex, data):
                        print(data)

                await asyncio.sleep(0.1)

        except Exception as proc_err:
            await self.exception_handler(proc_err)

    async def run_backend(self):
        """Runs the backend and starts the main loop."""
        await self.handle_start()
        with self.lock:
            self.subprocess_loop = asyncio.get_running_loop()

        # We need to create a new task for each reader, otherwise
        # the loop will not be able to run the main task
        self.subprocess_loop.create_task(self.stdout_reader())
        self.subprocess_loop.create_task(self.stderr_reader())

        while not self._is_started.is_set():
            await asyncio.sleep(0.1)
        try:
            if self.init_engine:
                # if there is data in the init_engine list,
                # we send it to the backend and clear the list
                for msg in self.init_engine:
                    self.runner.stdin.write(f"{msg}\n".encode())
                self.init_engine = []

            while self._is_started.is_set():
                try:
                    if self.outgoing:
                        data = self.outgoing.pop(0)
                        with self.lock:
                            self.init_engine.append(data)
                        self.runner.stdin.write(f"{data}\n".encode())
                        await self.runner.stdin.drain()

                        self.init_engine = []

                    await asyncio.sleep(0.1)

                except (BrokenPipeError, ConnectionResetError) as runtime_err:
                    await self.exception_handler(runtime_err)
                    await self.run_backend()

                except AsyncioException as asyncio_err:
                    await self.exception_handler(asyncio_err, subtract=1)
                    await self.run_backend()

        except RuntimeError as runtime_err:
            await self.exception_handler(runtime_err, subtract=1)
            await self.run_backend()

    async def exception_handler(
        self, exc: Exception, subtract: int = 0, sleep: int = 1
    ):
        """Handle exceptions in the backend."""
        self.print_debug()
        with self.lock:
            self._is_started.clear()
            self._is_closed.set()
        if self.max_retries == 0:
            raise BackendFailedToStart("Exceeded max retries") from exc

        self.max_retries -= subtract
        await asyncio.sleep(sleep)

    def run(self):
        """Run the backend."""
        asyncio.run(self.run_backend())

    def start(self, debug: bool = False):
        """Creates a new thread and runs the backend in it."""
        self.debug = debug

        thread = threading.Thread(target=self.run, daemon=self.daemon)
        thread.start()

        with self.lock:
            if self.thread and self.thread.is_alive():
                self.thread.join()
            self.thread = thread

        self.loop.run_until_complete(self.check_backend())

    def close(self, reset: bool = False):  # pylint: disable=unused-argument
        """Close the backend."""
        with self.lock:
            self._is_started.clear()
            self._is_closed.set()

        if self.runner:
            try:
                self.subprocess_loop.call_soon_threadsafe(self.runner.terminate)
                self.subprocess_loop.call_soon_threadsafe(self.runner.kill)
            except Exception:
                pass
