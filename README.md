# PyWry Web Viewer

![PyWryLogo](https://raw.githubusercontent.com/OpenBB-finance/pywry/main/assets/PyWry.png)

Easily create HTML webviewers in python utilizing the [wry](https://github.com/tauri-apps/wry) library. Unlike many HTML viewers that exist for Python - Pywry allows you to run javacsript. PyWry is also a ~2mb footprint for Mac and Windows - Linux will require a few more libraries which are listed below.

Please note: this library is currently in early alpha and is NOT ready for production use.

## Installation

---------------------
PyWry is available on PyPI and can be installed with pip:

```bash
pip install pywry
```

---------------------
For development, you can install from source with the following steps:

- Clone the repository: `git clone https://github.com/OpenBB-finance/pywry.git`
- Install rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- Create a virtual environment: `python -m venv venv`
- Acitvate the environment: `source venv/bin/activate` (Unix) or `venv\Scripts\activate` (Windows)
- Install dependencies: `pip install .[dev]`
- Build the pip package: `maturin build`
- Install the package: `pip install [file path from above] --force-reinstall`

## Usage

```python
import asyncio
import sys

from pywry import PyWry


async def main_loop():
    while True:
        await asyncio.sleep(1)


if __name__ == "__main__":
    try:
        handler = PyWry()
        handler.send_html("<h1 style='color: red;'>Welcome to PyWry!</h1>")
        handler.start()

        # PyWry creates a new thread for the backend,
        # so we need to run the main loop in the main thread.
        # otherwise, the program will exit immediately.
        handler.loop.run_until_complete(main_loop())
    except KeyboardInterrupt:
        print("Keyboard interrupt detected. Exiting...")
        sys.exit(0)
```

## JSON Keys

PyWry uses a JSON object to communicate between the python and rust backends and the javascript
frontend. The following keys are available:

| Key | Type | Description |
| --- | --- | --- |
| `html` | `Path \| str` | The path to the HTML file to be loaded, or HTML string. |
| `title` | `str` | The title of the window. |
| `icon` | `str \| Path` | The path to `png` icon to be used for the window. |
| `json_data` | `str \| dict` | A JSON string or dictionary to be passed to the javascript frontend. (see below) |
| `height` | `int` | The height of the window. |
| `width` | `int` | The width of the window. |
| `download_path` | `str \| Path` | The path to the download directory. |

## Javascript

PyWry allows you to run javascript in the frontend. To do this, you can pass a dictionary
of data to the `json_data` key in the `send_html` method. This dictionary will be converted
to a JSON string and passed to the frontend. You can then access this data in the frontend
by using the `window.json_data` object. For example:

---------------------

### Python

```python
from pathlib import Path
# code from above ...

# change send_html line to:
        handler.send_html(
            html=Path(__file__).parent / "index.html", json_data={"name": "PyWry"}
        )
```

---------------------

### HTML

```html
<html>
    <head>
        <script>
            window.onload = () => {
                // if you passed a JSON string, you will need to parse it first
                if (typeof window.json_data === "string") {
                    window.json_data = JSON.parse(window.json_data);
                }
                document.getElementById("name").innerHTML = window.json_data.name;
            };
        </script>
    </head>
    <body>
        <h1 style='color: red;'>Hello, <span id="name"></span>!</h1>
    </body>
</html>
```

---------------------

## Platform-specific notes

All platforms use [TAO](https://github.com/tauri-apps/tao) to build the window, and wry re-exports it as an application module. Here is the underlying web engine each platform uses, and some dependencies you might need to install.

### Linux

Tao uses [gtk-rs](https://gtk-rs.org/) and its related libraries for window creation and wry also needs [WebKitGTK](https://webkitgtk.org/) for WebView. So please make sure the following packages are installed:

#### Arch Linux / Manjaro

```bash
sudo pacman -S webkit2gtk
```

#### Debian / Ubuntu

```bash
sudo apt install libwebkit2gtk-4.0-dev
```

#### Fedora / CentOS / AlmaLinux

```bash
sudo dnf install gtk3-devel webkit2gtk3-devel
```

### macOS

WebKit is native to macOS, so no additional dependencies are needed.

### Windows

WebView2 provided by Microsoft Edge Chromium is used. So wry supports Windows 7, 8, 10 and 11.

---------------------

### Troubleshooting Linux

#### `"/lib/x86_64-linux-gnu/libgio-2.0.so.0: undefined symbol: g_module_open_full"`

This is a known issue with the `gio` library. You can fix it by installing the `libglib2.0-dev` package.



PyWry is a project that aims to provide Python bindings for WRY, a cross-platform webview library. WRY is a trademark of the Tauri Program within the Commons Conservancy and PyWry is not officially endorsed or supported by them. PyWry is an independent and community-driven effort that respects the original goals and values of Tauri and WRY. PyWry does not claim any ownership or affiliation with WRY or the Tauri Program.
