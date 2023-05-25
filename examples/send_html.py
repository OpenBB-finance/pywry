import asyncio
import sys
from pathlib import Path

from pywry import PyWry


async def main_loop():
    while True:
        await asyncio.sleep(1)


if __name__ == "__main__":
    try:
        handler = PyWry()

        # We can send HTML directly as a string.
        handler.send_html(
            html="<h1 style='color: red;'>Welcome to PyWry!</h1>",
            title="PyWry Demo",
        )

        # Alternatively, we can send HTML as a Path object.
        html_path = Path(__file__).parent / "example.html"
        handler.send_html(html=html_path, title="PyWry Demo")
        handler.start()

        # PyWry creates a new thread for the backend,
        # so we need to have a loop running in the main thread.
        # otherwise, the program will exit immediately.
        handler.loop.run_until_complete(main_loop())
    except KeyboardInterrupt:
        print("Keyboard interrupt detected. Exiting...")
        sys.exit(0)
