# PyWry Web Viewer

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
>>> from pywry import PyWry
>>> handler = PyWry()
>>> handler.send_html("<h1>Welcome to plotting in PyWry</h1>")
>>> handler.start()
```

### Arguments

| Argument | Description | Default |
| --- | --- | --- |
| `html_str` | The HTML string to display in the webview | `None` |
| `html_path` | The path to the HTML file to display in the webview | `None` |
| `title` | The title of the webview | `PyWry` |

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


## Troubleshooting

### Linux

#### `"/lib/x86_64-linux-gnu/libgio-2.0.so.0: undefined symbol: g_module_open_full"`

This is a known issue with the `gio` library. You can fix it by installing the `libglib2.0-dev` package.
