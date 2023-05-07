import asyncio
import sys
from pathlib import Path

from pywry import PyWry


async def main_loop():
    while True:
        await asyncio.sleep(1)


if __name__ == "__main__":
    try:
        html_path = Path(__file__).parent / "example.html"
        handler = PyWry()
        handler.send_html(html_path=html_path, title="PyWry Demo")
        handler.start()

        # PyWry creates a new thread for the backend,
        # so we need to run the main loop in the main thread.
        # otherwise, the program will exit immediately.
        handler.loop.run_until_complete(main_loop())
    except KeyboardInterrupt:
        print("Keyboard interrupt detected. Exiting...")
        sys.exit(0)
