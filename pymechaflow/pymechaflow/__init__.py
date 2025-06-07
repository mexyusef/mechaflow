"""
MechaFlow Python bindings

A Python interface to the MechaFlow automation framework.
"""

try:
    from .pymechaflow import __version__
except ImportError:
    __version__ = "0.0.1"  # Default version if not built

# Import submodules for easy access
try:
    from .pymechaflow import device, engine
except ImportError:
    # Provide fallback implementations or warnings
    import warnings
    warnings.warn(
        "Native MechaFlow extensions could not be imported. "
        "This could be because the library has not been built yet."
    )

    # Define placeholder modules
    class _PlaceholderModule:
        def __getattr__(self, name):
            raise ImportError(
                f"The {name} component is not available because the "
                "MechaFlow native extension could not be loaded."
            )

    device = _PlaceholderModule()
    engine = _PlaceholderModule()

__all__ = ['device', 'engine', '__version__']
