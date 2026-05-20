from __future__ import annotations

from pathlib import Path

project = "GhGrab"
author = "abhixdd"
copyright = "2026, abhixdd"
release = "2.0.0"
version = release

extensions = [
    "myst_parser",
    "sphinx.ext.autosectionlabel",
    "qiskit_sphinx_theme",
]

templates_path = ["_templates"]
exclude_patterns = ["_build", "Thumbs.db", ".DS_Store"]

source_suffix = {
    ".rst": "restructuredtext",
    ".md": "markdown",
}

root_doc = "index"

html_theme = "qiskit-ecosystem"
html_title = f"GhGrab - v{release}"
html_static_path = ["_static"]
html_css_files = ["custom.css"]

autosectionlabel_prefix_document = True
myst_enable_extensions = [
    "colon_fence",
    "deflist",
]

rst_prolog = """
.. |project| replace:: GhGrab
"""

BASE_DIR = Path(__file__).resolve().parent
