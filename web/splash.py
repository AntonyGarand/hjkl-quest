"""Puts HJKL Quest's loading screen into the page `nsh package --target web`
wrote: the night sky, the wisp hopping on h j k l, a real download bar and
a vim tip, in place of the default spinner; and an on-screen keyboard for
phones and tablets (mobile.js).

    python3 web/splash.py SITE_DIR

It edits SITE_DIR/index.html in place, and stops with an error if the page
no longer looks as expected (a new nsh version), rather than guessing.
"""

import glob
import gzip
import os
import re
import sys

here = os.path.dirname(os.path.abspath(__file__))
site = sys.argv[1]
page_path = os.path.join(site, "index.html")
page = open(page_path, encoding="utf-8").read()


def read(name):
    return open(os.path.join(here, name), encoding="utf-8").read()


def replace_once(text, old, new, what):
    if text.count(old) != 1:
        sys.exit(f"splash.py: expected one {what} in {page_path}, found {text.count(old)}; has nsh changed its page?")
    return text.replace(old, new)


wasm = glob.glob(os.path.join(site, "*.wasm"))
if len(wasm) != 1:
    sys.exit(f"splash.py: expected one .wasm in {site}, found {wasm}")
size = os.path.getsize(wasm[0])

# The sky's colour from the first paint, and in the browser's chrome.
page = re.sub(r'<meta name="theme-color" content="#[0-9a-fA-F]{6}">', '<meta name="theme-color" content="#0a0f24">', page, count=1)
# Android shrinks the page above its on-screen keyboard instead of
# covering the game with it.
page = replace_once(
    page,
    '<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">',
    '<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover, interactive-widget=resizes-content">',
    "viewport tag",
)
# The progress script goes before the page's own, which starts the download.
script = read("splash.js").replace("__WASM_SIZE__", str(size))
page = replace_once(page, "<script>const fetches", f"<script>\n{script}</script>\n<script>const fetches", "download script")
# The on-screen keyboard for phones and tablets, last.
page = replace_once(page, "</body>", "<script>\n" + read("mobile.js") + "</script>\n</body>", "end of the body")
page = replace_once(page, "</style>\n</head>", read("splash.css") + "</style>\n</head>", "end of the styles")
# Anonymous pageview to Plausible (stats.garand.dev), last in the head.
page = replace_once(page, "</head>", "<script>\n" + read("analytics.js") + "</script>\n</head>", "end of the head")
page = re.sub(
    r'(<div id="status" role="status" aria-live="polite">)<div class="spinner" aria-label="Loading"></div>(</div>)',
    lambda m: m.group(1) + "\n" + read("splash.html") + m.group(2),
    page,
    count=1,
)
if 'class="hq"' not in page:
    sys.exit(f"splash.py: no spinner to replace in {page_path}; has nsh changed its page?")

open(page_path, "w", encoding="utf-8", newline="\n").write(page)
# Pre-compressed copies of the old page would be served stale: gzip it
# again, and drop the brotli copy (Python has no brotli of its own).
if os.path.exists(page_path + ".gz"):
    with gzip.open(page_path + ".gz", "wb", compresslevel=9) as f:
        f.write(page.encode("utf-8"))
if os.path.exists(page_path + ".br"):
    os.remove(page_path + ".br")
print(f"splash.py: loading screen added ({size} bytes of wasm)")
