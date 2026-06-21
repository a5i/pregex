"""eregex — an advanced regular expression engine (Python bindings).

This package is the public Python API for the Rust `eregex` engine. The native
extension is built by maturin and lives in the ``eregex.eregex_python``
submodule; everything is re-exported here for a clean ``import eregex``.
"""

from .eregex_python import (  # noqa: F401
    ASCII,
    DOTALL,
    FULLCASE,
    IGNORECASE,
    LOCALE,
    MULTILINE,
    UNICODE,
    VERSION0,
    VERSION1,
    VERBOSE,
    WORD,
    Match,
    PartialMatch,
    Regex,
    __version__,
    compile,
    escape,
    escape_literal_spaces,
    escape_special_only,
    is_match,
    parse_flags,
)

__all__ = [
    # classes
    "Regex",
    "Match",
    "PartialMatch",
    # flag constants
    "IGNORECASE",
    "MULTILINE",
    "DOTALL",
    "UNICODE",
    "ASCII",
    "VERBOSE",
    "FULLCASE",
    "WORD",
    "LOCALE",
    "VERSION0",
    "VERSION1",
    # helpers
    "escape",
    "escape_special_only",
    "escape_literal_spaces",
    "is_match",
    "compile",
    "parse_flags",
    # meta
    "__version__",
]
