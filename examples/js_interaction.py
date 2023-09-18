import asyncio
import json
import sys
from pathlib import Path

from pywry import PyWry


class Backend(PyWry):
    """Custom backend for PyWry."""

    async def get_results(self) -> dict:
        """Wait for completion of interactive task and return the data.

        Returns
        -------
        dict
            The data returned from pywry backend.
        """
        while True:
            try:
                data: dict = self.recv.get(block=False) or {}
                if data.get("result", False):
                    return json.loads(data["result"])
            except Exception:  # pylint: disable=W0703
                pass

            await asyncio.sleep(1)

    def send_interaction(self, title, params: dict, filepath: Path) -> dict:
        self.check_backend()

        outgoing = dict(
            html=filepath.resolve(),
            json_data=params,
            title=title,
            width=400,
            height=400,
        )
        self.send_outgoing(outgoing)

        try:
            return self.loop.run_until_complete(self.get_results())
        except KeyboardInterrupt:
            print("\nKeyboard interrupt detected. Exiting...")
            return {}


async def main_loop():
    while True:
        await asyncio.sleep(1)


if __name__ == "__main__":
    try:
        handler = Backend()
        handler.start()

        # We send a list of parameters to the backend. The backend will create
        # an HTML form with inputs for each parameter and a submit button.
        # When the user submits the form, the data will be sent back
        # to the Python process.
        args = handler.send_interaction(
            title="PyWry Example",
            params={"required": ["first_name"], "optional": ["last_name"]},
            filepath=Path(__file__).parent / "js_interaction.html",
        )

        # We can now access the data returned from the form.
        result = f"Hello {args.get('first_name', 'World')} {args.get('last_name', '')}"
        handler.send_html(html=result, title="PyWry Example")

        handler.loop.run_until_complete(main_loop())

    except KeyboardInterrupt:
        print("Keyboard interrupt detected. Exiting...")
        sys.exit(0)
