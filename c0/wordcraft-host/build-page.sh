#!/usr/bin/env bash
# Size tuning belongs to the guest page (44 MiB wasm cap in the C0X pin), not the workspace's native releases.
# Same recipe that took C0pdf from 46.4 MB to 30.4 MB on 2026-10-28 (K-71).
set -euo pipefail
cd "$(dirname "$0")"
export CARGO_PROFILE_RELEASE_OPT_LEVEL=z
export CARGO_PROFILE_RELEASE_LTO=fat
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
export CARGO_PROFILE_RELEASE_STRIP=debuginfo
# Keep upstream's panic strategy: document entry points rely on catch_unwind.
if [[ -n "${CRAFT_FONTS_DIR:-}" && ! -d "$CRAFT_FONTS_DIR" ]]; then unset CRAFT_FONTS_DIR; fi
NO_COLOR=true trunk build --release "$@"
