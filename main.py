import time
import asyncio
from multiprocessing import Process
from websockets import connect
import pywry

base = pywry.WindowManager()

class PyWry:
    def __init__(self):
        self.port = base.get_port()
        self.runner = Process(target=self.handle_start)
        self.runner.start()
        self.url = "ws://127.0.0.1:" + str(self.port)
        # TODO: replace sleep with a check for the validity of websocket
        time.sleep(3)

    def send_html(self, html: str):
        asyncio.run(self.handle_html(html))

    async def handle_html(self, html: str):
        async with connect(self.url) as websocket:
            await websocket.send(html)

    @staticmethod
    def handle_start():
        base.start()

    # TODO: kill process when class is exited


if __name__ == "__main__":
    x = PyWry()
    x.send_html("<h1>The first experiment</h1>")
    x.send_html("<h1>Things are working</h1>")
