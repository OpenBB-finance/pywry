import time
import asyncio
from multiprocessing import Process
from websockets.client import connect
import pywry


class PyWry:
    base = pywry.WindowManager()

    def __init__(self):
        self.port = self.base.get_port()
        self.runner = Process(target=self.handle_start)
        self.runner.start()
        self.url = "ws://127.0.0.1:" + str(self.port)
        self.wait_for_connection()

    def __del__(self):
        self.runner.terminate()

    def send_html(self, html: str):
        asyncio.run(self.handle_html(html))

    async def handle_html(self, html: str):
        async with connect(self.url) as websocket:
            await websocket.send(html)

    def wait_for_connection(self):
        i = 0
        while True:
            try:
                asyncio.run(self.handle_html("<test>"))
                break
            except ConnectionRefusedError as e:
                i += 1
                if i > 30:
                    raise e
                else:
                    time.sleep(1)

    @staticmethod
    def handle_start():
        PyWry.base.start()


if __name__ == "__main__":
    x = PyWry()
    x.send_html("<h1>The first experiment</h1>")
    x.send_html("<h1>Things are working</h1>")
