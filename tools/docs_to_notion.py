"""Package docs/ as a zip in Notion's Markdown import layout.

Notion imports a page with sub-pages as `Title.md` next to a folder `Title/`
holding the sub-pages. This script maps the repository layout onto that:

    docs/index.md                    -> fluidnet.md
    docs/00-start-here.md            -> fluidnet/Start Here.md
    docs/02-engineering/index.md     -> fluidnet/02 Engineering.md
    docs/02-engineering/solvers.md   -> fluidnet/02 Engineering/Transient and steady-state solvers.md

Page file names are the page titles (first "# " line). Relative links between
pages are rewritten to the new paths. The Markdown in docs/ stays the source;
rerun this after changes and import the zip in Notion (Import > Markdown & CSV).

Usage:  python tools/docs_to_notion.py [OUT.zip]
"""
from __future__ import annotations

import posixpath
import re
import sys
import zipfile
from pathlib import Path
from urllib.parse import quote, unquote

ROOT = Path(__file__).resolve().parents[1]
DOCS = ROOT / "docs"
ROOT_TITLE = "fluidnet"
LINK = re.compile(r"\]\(([^)#\s]+\.md)(#[^)]*)?\)")


def title_of(p: Path) -> str:
    for line in p.read_text(encoding="utf-8").splitlines():
        if line.startswith("# "):
            return re.sub(r'[\\/:*?"<>|]', "-", line[2:].strip())
    return p.stem


def build_map() -> dict[Path, str]:
    """Source file -> path inside the zip."""
    m: dict[Path, str] = {DOCS / "index.md": f"{ROOT_TITLE}.md"}
    for p in sorted(DOCS.glob("*.md")):
        if p.name != "index.md":
            m[p] = f"{ROOT_TITLE}/{title_of(p)}.md"
    for d in sorted(x for x in DOCS.iterdir() if x.is_dir() and (x / "index.md").exists()):
        sec = title_of(d / "index.md")
        m[d / "index.md"] = f"{ROOT_TITLE}/{sec}.md"
        for p in sorted(d.glob("*.md")):
            if p.name != "index.md":
                m[p] = f"{ROOT_TITLE}/{sec}/{title_of(p)}.md"
    return m


def rewrite(text: str, src: Path, m: dict[Path, str]) -> str:
    here = posixpath.dirname(m[src])

    def sub(mt: re.Match) -> str:
        target = (src.parent / unquote(mt.group(1))).resolve()
        if target not in m:
            return mt.group(0)
        rel = posixpath.relpath(m[target], here or ".")
        return f"]({quote(rel)}{mt.group(2) or ''})"

    return LINK.sub(sub, text)


def main() -> None:
    out = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "fluidnet-docs-notion.zip"
    m = build_map()
    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
        for src, dst in m.items():
            z.writestr(dst, rewrite(src.read_text(encoding="utf-8"), src, m))
    print(f"wrote {out} ({len(m)} pages)")


if __name__ == "__main__":
    main()
