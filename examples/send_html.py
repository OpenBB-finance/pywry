import asyncio
import sys

from pywry import PyWry


async def main_loop():
    while True:
        await asyncio.sleep(1)


if __name__ == "__main__":
    try:
        handler = PyWry()
        handler.send_html(
            html_str="<h1 style='color: red;'>Welcome to PyWry!</h1>",
            title="PyWry Demo",
        )
        handler.start()

        # PyWry creates a new thread for the backend,
        # so we need to run the main loop in the main thread.
        # otherwise, the program will exit immediately.
        handler.loop.run_until_complete(main_loop())
    except KeyboardInterrupt:
        print("Keyboard interrupt detected. Exiting...")
        sys.exit(0)
