# Tauri Web Viewer

To use just run the following:
- Install rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- Create a virtual environment: `python -m venv .env`
- Acitvate the environment: `source .env/bin/acticate`
- Install dependencies: `pip install .[dev]`
- Add the package into the environment: `maturin develop`


Basic Usage:
```python
>>> import pywry
>>> pywry.show_html("OpenBB Tab", "<h1>Welcome to WRY with python</h1>", False)

```
Note: There is currently an issue if you try to run this inside an X86_64 conda
environment on an M1 machine.


### Arguments

- `title`
- `html_content`
- `hide_output`
