import time
import asyncio
from multiprocessing import Process
from websockets.client import connect
from pywry import pywry


class PyWry:
    base = pywry.WindowManager()

    def __init__(self):
        self.port = self.get_clean_port()
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

    def get_clean_port(self) -> str:
        port = self.base.get_port()
        if port == 0:
            raise ConnectionError("Could not connect to a port")
        else:
            return str(port)

    @staticmethod
    def handle_start():
        PyWry.base.start()
