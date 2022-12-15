import asyncio
import time
from multiprocessing import Process
import pywry


def start():
    pywry.start()


async def main():
    await pywry.send_html("<h1>Hello Stepitimanager</h1>")


if __name__ == "__main__":
    print("Creating process")
    p = Process(target=start)
    print("Starting process")
    p.start()
    time.sleep(2)
    print("Sending HTML")
    asyncio.run(main())
    print("Sent HTML")
