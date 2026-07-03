#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

# Branch switches can leave behind partially removed Docusaurus package
# directories, so rebuild the install tree from a clean slate each time.
python3 - <<'PY'
from pathlib import Path
import shutil
import time

node_modules = Path("website/node_modules")
for attempt in range(5):
    if not node_modules.exists():
        break
    try:
        shutil.rmtree(node_modules)
        break
    except OSError:
        if attempt == 4:
            raise
        time.sleep(1)
PY
npm --prefix website ci
