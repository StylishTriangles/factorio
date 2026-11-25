from hashlib import sha256
from pathlib import Path


dist = Path("dist")
digest = sha256()
for asset in ("start.js", "pkg/planner.js", "pkg/planner_bg.wasm"):
    digest.update((dist / asset).read_bytes())
version = digest.hexdigest()[:16]

for page in (dist / "index.html", dist / "balancers/index.html"):
    html = page.read_text(encoding="utf-8")
    page.write_text(html.replace("__WASM_VERSION__", version), encoding="utf-8")
