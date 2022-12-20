# Tauri Web Viewer

Easily create HTML webviewers in python utilizing the [yrw](https://github.com/tauri-apps/wry) library.

To use just run the following:
- Install rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- Create a virtual environment: `python -m venv .env`
- Acitvate the environment: `source .env/bin/activate`
- Install dependencies: `pip install .[dev]`
- Add the package into the environment: `maturin develop`


Basic Usage:
```python
>>> import pywry
>>> handler = pywry.PyWry()
>>> handler.send_html("<h1>Welcome to plotting in PyWry</h1>")
```
Note: There is currently an issue if you try to run this inside an X86_64 conda
environment on an M1 machine.


### Arguments

- `title`
- `html_content`
- `hide_output`
