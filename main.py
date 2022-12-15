import time
import asyncio
from multiprocessing import Process
from websockets import connect
import pywry


def handle_start():
    pywry.start()


class PyWry:
    def __init__(self):
        self.runner = Process(target=handle_start)
        self.runner.start()
        self.url = "ws://127.0.0.1:9000"
        # TODO: replace sleep with a check for the validity of websocket
        time.sleep(3)

    def send_html(self, html: str):
        asyncio.run(self.handle_html(html))

    async def handle_html(self, html: str):
        async with connect(self.url) as websocket:
            await websocket.send(html)

    # TODO: kill process when class is exited


if __name__ == "__main__":
    x = PyWry()
    x.send_html("<h1>The first experiment</h1>")
    x.send_html("<h1>Things are working</h1>")
