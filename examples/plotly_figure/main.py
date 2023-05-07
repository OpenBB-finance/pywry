import asyncio
import sys

import numpy as np
import plotly.graph_objects as go
import plotly.io as pio

from backend import pywry_backend


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


class Main:
    async def main_loop(self):
        while True:
            await asyncio.sleep(1)

    def run(self):
        pywry_backend().start()

        fig = PyWryFigure()
        fig.add_scatter(y=np.random.randn(500), mode="markers")
        fig.add_scatter(y=np.random.randn(500) + 1, mode="markers")
        fig.add_scatter(y=np.random.randn(500) + 2, mode="markers")
        fig.update_layout(title="Plotly Figure")
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
