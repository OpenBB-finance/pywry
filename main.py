import time

import numpy as np
import plotly.graph_objects as go
from pywry import PyWry


def main():
    backend = PyWry()
    fig = go.Figure(
        data=go.Scatter(
            x=list(np.random.randn(500)),
            y=list(np.random.randn(500)),
            mode='markers',
            marker=dict(
                size=16,
                color=list(np.random.randn(500)),
                colorscale='Viridis',
                showscale=True,
            ),
        )
    )
    fig.update_layout(title_text='Testing Plotly in Webview', template='plotly_dark')
    backend.send_fig(fig)
    backend.send_fig(fig)
    backend.send_fig(fig)
    backend.send_fig(fig)
    backend.send_fig(fig)
    backend.send_fig(fig)


if __name__ == "__main__":
    main()
    time.sleep(30)
