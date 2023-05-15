import os
import sys

import toml
from setuptools import setup

try:
    from setuptools_rust import Binding, RustExtension
except ImportError:
    import subprocess

    subprocess.check_call([sys.executable, "-m", "pip", "install", "setuptools-rust"])
    from setuptools_rust import Binding, RustExtension


if sys.platform == "linux":
    os.environ["RUSTFLAGS"] = (
        "-C link-args=-Wl,-rpath,$ORIGIN/../lib "
        "-C link-args=-Wl,-rpath,$ORIGIN/../lib64 "
        "-C link-args=-Wl,-rpath,$ORIGIN/../include "
        "-C link-args=-Wl,-rpath,$ORIGIN/../share/pkgconfig "
        "-C link-args=-Wl,-rpath,$ORIGIN/../lib/pkgconfig "
        "-C link-args=-Wl,-rpath,$ORIGIN/../lib64/pkgconfig "
        "-C link-args=-Wl,-rpath,$ORIGIN/../lib/x86_64-linux-gnu "
        "-C link-args=-Wl,-rpath,$ORIGIN/../lib64/x86_64-linux-gnu "
        "-C link-args=-Wl,-rpath,$ORIGIN/../local/lib/pkgconfig "
        "-C link-args=-Wl,-rpath,$ORIGIN/../local/lib64/pkgconfig "
        "-C link-args=-Wl,-rpath,$ORIGIN/../../local/lib "
        "-C link-args=-Wl,-rpath,$ORIGIN/../../local/lib64 "
        "-C link-args=-Wl,-rpath,$ORIGIN/../../local/lib/x86_64-linux-gnu "
        "-C link-args=-Wl,-rpath,$ORIGIN/../../local/lib64/x86_64-linux-gnu "
    )

pyproject = toml.load("pyproject.toml")
version = pyproject["project"]["version"]

setup(
    name="pywry",
    version=version,
    rust_extensions=[
        RustExtension(
            "pywry.pywry",
            "Cargo.toml",
            binding=Binding.PyO3,
            debug=False,
            args=["--release", "--no-default-features"],
        )
    ],
    zip_safe=False,
    setup_requires=["setuptools-rust>=0.10.1", "wheel", "toml"],
    package_dir={"": "python"},
    python_requires=">=3.9",
    include_package_data=False,
    package_data={"pywry": ["py.typed", "pywry.pyi"]},
)
