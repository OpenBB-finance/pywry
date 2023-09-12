import os
import sys
import sysconfig
from pathlib import Path


def find_pywry_bin() -> Path:
    """Find the pywry binary."""
    if hasattr(sys, "frozen"):
        # pylint: disable=E1101,W0212
        path = Path(sys._MEIPASS).resolve()

        for bin_path in path.rglob("pywry*"):
            if bin_path.is_file():
                return bin_path

    config_var = sysconfig.get_config_var("EXE")

    bin_path = (Path(sysconfig.get_path("scripts")) / "pywry").with_suffix(config_var)
    if bin_path.is_file():
        return bin_path

    if sys.version_info >= (3, 10):
        user_scheme = sysconfig.get_preferred_scheme("user")
    elif os.name == "nt":
        user_scheme = "nt_user"
    elif sys.platform == "darwin" and sys._framework:
        user_scheme = "osx_framework_user"
    else:
        user_scheme = "posix_user"

    path = Path(sysconfig.get_path("scripts", scheme=user_scheme))
    bin_path = (path / "pywry").with_suffix(config_var)
    if bin_path.is_file():
        return bin_path

    raise FileNotFoundError("Could not find pywry binary")
