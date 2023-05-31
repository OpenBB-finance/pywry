import asyncio
import io
import sys
from pathlib import Path
from typing import Union

import numpy as np
import plotly.graph_objects as go
import plotly.io as pio
from backend import pywry_backend
from PIL import Image


# We create a custom figure class that inherits from Plotly's Figure class.
# This allows us to add custom show method that will send the figure to the
# backend if running in a TTY terminal.
class PyWryFigure(go.Figure):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.update_layout(
            template="plotly_dark",
            paper_bgcolor="#000000",
            plot_bgcolor="#000000",
            dragmode="pan",
            hovermode="closest",
        )

    def show(self, *args, **kwargs):
        if pywry_backend().isatty:
            try:
                # We send the figure to the backend to be displayed
                return pywry_backend().send_figure(self)
            except Exception:
                pass

        return pio.show(self, *args, **kwargs)

    def pywry_write_image(
        self,
        filepath: Union[str, Path] = "plotly_image.png",
        scale: int = 1,
        timeout: int = 5,
    ):
        """Convert a Plotly figure to an image.

        filepath : Union[str, Path], optional
            Filepath to save image to, by default "plotly_image.png"
        scale : int, optional
            Image scale, by default 1
        timeout : int, optional
            Timeout for receiving the image, by default 5
        """
        if not pywry_backend().isatty:
            return

        if not isinstance(filepath, Path):
            filepath = Path(filepath)

        img_format = filepath.suffix.lstrip(".").lower()

        if img_format == "jpg":
            img_format = "jpeg"

        if img_format not in ["png", "jpeg", "svg"]:
            raise ValueError(
                f"Invalid image format {img_format}. "
                "Must be one of 'png', 'jpeg', or 'svg'."
            )

        try:
            # We send the figure to the backend to be converted to an image
            response = pywry_backend().figure_write_image(
                self, img_format=img_format, scale=scale, timeout=timeout
            )
            if img_format == "svg":
                filepath.write_bytes(response)
            else:
                imgbytes = io.BytesIO(response)
                image = Image.open(imgbytes)
                image.save(filepath, format=img_format)
        except Exception:
            pass


class Main:
    async def main_loop(self):
        while True:
            await asyncio.sleep(1)

    def run(self):
        fig = PyWryFigure()
        fig.add_scatter(y=np.random.randn(500), mode="markers")
        fig.add_scatter(y=np.random.randn(500) + 1, mode="markers")
        fig.add_scatter(y=np.random.randn(500) + 2, mode="markers")
        fig.update_layout(title="Plotly Figure")

        # We start the backend in headless mode for rendering the image without displaying it.
        pywry_backend().start(headless=True)
        fig.pywry_write_image(scale=1.8, filepath="plotly_image.png")
        fig.pywry_write_image(scale=1.8, filepath="plotly_image.svg")
        fig.pywry_write_image(scale=1.8, filepath="plotly_image.jpg")
        pywry_backend().close()

        # We start the backend in interactive mode for displaying the figure.
        pywry_backend().start()
        fig.show()

        pywry_backend().loop.run_until_complete(self.main_loop())


if __name__ == "__main__":
    try:
        # PyWry creates a new thread for the backend,
        # so we need to run the main loop in the main thread.
        asyncio.create_task(Main().run())
    except KeyboardInterrupt:
        print("Keyboard interrupt detected. Exiting...")
        sys.exit(0)
