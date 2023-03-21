from pywry import pywry

__version__ = pywry.__version__
__doc__ = pywry.__doc__

if hasattr(pywry, "__all__"):
    __all__ = pywry.__all__


from .core import PyWry  # noqa: F401
