import asyncio
import atexit
import json
import os
import re
import sys
import threading
import traceback
from asyncio.exceptions import CancelledError, IncompleteReadError, TimeoutError
from pathlib import Path
from queue import Queue
from subprocess import PIPE
from typing import List, Optional, Union

import setproctitle

import pywry

__all__ = ["PyWry", "BackendFailedToStart"]

if sys.version_info < (3, 9):
    QueueT = Queue
else:
    QueueT = Queue[dict]

AsyncioException = (
    CancelledError,
    IncompleteReadError,
    TimeoutError,
    ConnectionResetError,
)

# Ignore some messages from the backend that can be confusing to users
# these messages are not errors, but they are not useful either
IGNORE_REGEX = (
    r"(Wayland|Compositor|webkit_download|"
    r"NeedDebuggerBreak|GLib-GIO-CRITICAL|"
    r"EGLDisplay|libEGL|Could not determine|"
    r"Gtk-Message|WARNING|Gtk)"
)

ACCEPTED_KEYS_TYPES = {
    "html": (str, Path),
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
    _bootargs: List[str] = []

    daemon: bool = True
    debug: bool = False
    shell: bool = False
    outgoing: List[str] = []
    init_engine: List[str] = []
    recv: QueueT = Queue()

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
        self.daemon: bool = daemon
        self.max_retries: int = max_retries
        self.proc_name: str = proc_name

        try:
            self.loop: asyncio.AbstractEventLoop = asyncio.get_event_loop()
        except RuntimeError:
            self.loop = asyncio.new_event_loop()
            asyncio.set_event_loop(self.loop)

        self.loop_policy()

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
        html: Union[str, Path],
        json_data: Optional[Union[dict, str]] = None,
        title: str = "",
        width: int = 800,
        height: int = 600,
        **kwargs,
    ):
        """Send html to backend.

        Parameters
        ----------
        html: Union[str, Path]
            HTML to send to backend.
        json_data : Optional[Union[dict, str]], optional
            JSON data to send to backend, by default None
        title : str, optional
            Title to display in the window, by default ""
        width : int, optional
            Width of the window, by default 800
        height : int, optional
            Height of the window, by default 600
        """
        self.check_backend()
        kwargs.update(
            dict(
                html=html, json_data=json_data, title=title, width=width, height=height
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

    def clean_print(self, message: dict):
        """Clean messages from the backend."""
        if message and not re.search(IGNORE_REGEX, message, re.IGNORECASE):
            print(message)

    def check_backend(self):
        """Check if the backend is running."""

        if self.max_retries == 0:
            # If the backend is not running and we have retried
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

            kwargs = dict()
            pywry_path = pywry.find_pywry_bin()
            cmd = [pywry_path] + self._bootargs

            # For pyinstaller builds we need to get the path to the executable
            if hasattr(sys, "frozen"):
                cmd = f"pywry {' '.join(self._bootargs)}"
                if sys.platform == "darwin":
                    cmd = f"'{pywry_path}'"

                self.shell = True
                kwargs.update(dict(cwd=str(pywry_path.parent)))

            env = os.environ.copy()
            kwargs.update(dict(env=env))

            runner = await self.create_subprocess(cmd=cmd, **kwargs)

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

    def print_message(self, message: dict):
        """Print messages from the backend."""
        print_style = {"error": "\033[91m", "info": "\033[93m", "debug": "\033[92m"}
        if (
            key := re.search(r"error|info|debug", ",".join(message.keys()))
        ) is not None:
            return print(f"{print_style[key.group()]}{message[key.group()]}")

        return self.clean_print(message)

    async def recv_message(self, data: str):
        """Creates a new task to process messages from the stdout reader."""
        try:
            message: dict = json.loads(data)
            if message.get("result", None):
                return self.recv.put(message, block=False)
            self.print_message(message)
        except (json.JSONDecodeError, AttributeError):
            self.clean_print(data)

    async def stdout_reader(self):
        """Read stdout from the backend."""
        try:
            while self._is_started.is_set():
                if data := (await self.runner.stdout.readline()).decode().strip():
                    asyncio.create_task(self.recv_message(data))

                await asyncio.sleep(0.02)
        except Exception as proc_err:
            await self.exception_handler(proc_err)

    async def stderr_reader(self):
        """Read stderr from the backend."""
        try:
            while self._is_started.is_set():
                if data := (await self.runner.stderr.readline()).decode().strip():
                    self.clean_print(data)

                await asyncio.sleep(1)
        except Exception as proc_err:
            await self.exception_handler(proc_err)

    def loop_policy(self):
        """Set the loop policy."""
        if sys.platform == "win32":
            asyncio.set_event_loop_policy(asyncio.WindowsProactorEventLoopPolicy())

    async def run_backend(self):
        """Runs the backend and starts the main loop."""
        await self.handle_start()
        with self.lock:
            self.subprocess_loop = asyncio.get_running_loop()
            self.loop_policy()

        # We need to create a new task for each reader, otherwise
        # the loop will not be able to run the main task
        self.subprocess_loop.create_task(self.stdout_reader())

        # We only need to read stderr if we are in debug mode
        if self.debug:
            self.subprocess_loop.create_task(self.stderr_reader())

        try:
            if self.init_engine:
                # if there is data in the init_engine list,
                # we send it to the backend and clear the list
                for msg in self.init_engine:
                    self.runner.stdin.write(f"{msg}\n".encode())
                    await self.runner.stdin.drain()
                self.init_engine.clear()

            while self._is_started.is_set():
                try:
                    if self.outgoing:
                        data = self.outgoing.pop(0)
                        with self.lock:
                            self.init_engine.append(data)
                        self.runner.stdin.write(f"{data}\n".encode())
                        await self.runner.stdin.drain()

                        with self.lock:
                            self.init_engine.clear()

                    await asyncio.sleep(0.5)

                except (BrokenPipeError, ConnectionResetError) as pipe_err:
                    await self.exception_handler(pipe_err)
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

    def start(self, debug: bool = False, headless: bool = False):
        """Creates a new thread and runs the backend in it.

        Parameters
        ----------
        debug : bool, optional
            Whether to print debug messages, by default False
        headless : bool, optional
            Whether to run the backend in headless mode for plotly image exports,
            by default False
        """
        if self._is_started.is_set():
            return
        self.debug = debug

        self._bootargs = []
        for arg, flag in zip([debug, headless], ["debug", "headless"]):
            if arg:
                self._bootargs.append(f"--{flag}")

        thread = threading.Thread(target=self.run, daemon=self.daemon)
        thread.start()

        with self.lock:
            if self.thread and self.thread.is_alive():
                self.thread.join()
            self.thread = thread

        self.check_backend()

    def close(self):
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

    async def create_subprocess(self, cmd: Union[str, List[str]], **kwargs):
        try:
            if self.shell:
                if isinstance(cmd, list):
                    cmd = " ".join(cmd)

                return await asyncio.create_subprocess_shell(
                    cmd, stdin=PIPE, stdout=PIPE, stderr=PIPE, limit=2**64, **kwargs
                )

            return await asyncio.create_subprocess_exec(
                *cmd, stdin=PIPE, stdout=PIPE, stderr=PIPE, limit=2**64, **kwargs
            )
        except NotImplementedError as err:
            await self.exception_handler(err)
