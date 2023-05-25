import asyncio
import atexit
import os
import re
import socket
import sys
import threading
import time
from pathlib import Path
from subprocess import PIPE, STDOUT
from typing import List, Optional

import numpy as np
import psutil
from backend import pywry_backend


class Main:
    def __init__(self):
        self.processes: List[psutil.Process] = []
        self.parent_path = Path(__file__).parent

    def kill_processes(self) -> None:
        """Kills all processes started by this class."""
        for process in [p for p in self.processes if p.is_running()]:
            for child in process.children(recursive=True):
                if child.is_running():
                    child.kill()

            process.kill()

    def check_processes(self, name: Optional[str] = None) -> str:
        """Check if a process is already running, and returns the url."""

        for process in self.processes:
            if not process.is_running():
                self.processes.remove(process)
                continue

            cmdline = " ".join(process.cmdline())
            port = re.findall(r"--port=(\d+)", cmdline)
            port = port[0] if port else ""

            if re.findall(r"-m\s+.*streamlit_run|streamlit_run", cmdline):
                return f"http://localhost:{port}/{name}"

        return ""

    @staticmethod
    def get_free_port() -> int:
        """Search for a random free port number."""
        not_free = True
        while not_free:
            port = np.random.randint(7000, 7999)
            with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
                res = sock.connect_ex(("localhost", port))
                if res != 0:
                    not_free = False
        return port

    def call_streamlit(self) -> None:
        """Create a streamlit call command.

        A metafunction that creates a launch command for a streamlit dashboard.
        """

        streamlit_run = Path(__file__).parent / "streamlit_run.py"
        python_path = streamlit_run.relative_to(self.parent_path).with_suffix("")
        cmd = [sys.executable, "-m", ".".join(python_path.parts)]

        process_check = self.check_processes()

        if not process_check:
            port = self.get_free_port()
            os.environ["PYTHONPATH"] = str(self.parent_path)
            cmd += [f"--port={port}"]

            self.processes.append(
                psutil.Popen(
                    cmd,
                    stdout=PIPE,
                    stderr=STDOUT,
                    stdin=PIPE,
                    env=os.environ,
                    cwd=str(self.parent_path),
                )
            )
            atexit.register(self.kill_processes)

            print("Waiting for streamlit to start. This may take a few seconds.")

            thread = threading.Thread(
                target=non_blocking_streamlit,
                args=(self.processes[-1],),
                daemon=True,
            )
            thread.start()
            time.sleep(6 if sys.platform == "darwin" else 3)

            if not self.processes[-1].is_running():
                self.processes.remove(self.processes[-1])
                print("Error: streamlit server failed to start.\n")
                return

        if self.check_processes():
            pywry_backend().send_url(
                url=self.check_processes(""),
                title="CSV File Viewer",
                width=1000,
                height=800,
            )

    async def loop(self) -> None:
        """Main loop."""
        while True:
            await asyncio.sleep(1)

    def run(self) -> None:
        """Run the main loop."""
        pywry_backend().start()
        self.call_streamlit()
        pywry_backend().loop.run_until_complete(self.loop())


def non_blocking_streamlit(process: psutil.Popen) -> None:
    """We need this or else streamlit engine will not run the modules."""
    while process.is_running():
        process.communicate()


if __name__ == "__main__":
    try:
        # PyWry creates a new thread for the backend,
        # so we need to have a loop running in the main thread.
        # otherwise, the program will exit immediately.
        asyncio.create_task(Main().run())
    except KeyboardInterrupt:
        print("Keyboard interrupt detected. Exiting...")
        sys.exit(0)
