from __future__ import annotations

from pathlib import Path

project = "GhGrab"
author = "abhixdd"
copyright = "2026, abhixdd"
release = "1.3.1"
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
html_title = "ghgrab - grab anything you want"
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
