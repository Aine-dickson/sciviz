#!/usr/bin/env bash
# Regenerates the stage 5 evidence PNGs. Usage (from the repo root):
#   docs/evidence/b1-2-stage5/capture.sh [path/to/01-svf-order.vtk] [path/to/svf-explorer]
# Defaults: ~/Downloads/rodero2021/01-svf-order.vtk and target/release/svf-explorer(.exe).
# Build the binary first: cargo build -p svf-explorer --release
set -euo pipefail
VTK="${1:-$HOME/Downloads/rodero2021/01-svf-order.vtk}"
BIN="${2:-target/release/svf-explorer}"
[ -x "$BIN" ] || BIN="$BIN.exe"
OUT="$(dirname "$0")"

GLYPHS=(--vtk-cell-axial fibres --vtk-cell-tags ID=1,2)
RHO=(--vtk-scalar RHO.dat --vtk-scalar-exclude -10)

shoot() { # name, then camera flags
  local name="$1"; shift
  "$BIN" --vtk "$VTK" "${GLYPHS[@]}" "$@" --screenshot "$OUT/$name-plain.png"
  "$BIN" --vtk "$VTK" "${GLYPHS[@]}" "${RHO[@]}" "$@" --screenshot "$OUT/$name-rho.png"
}

shoot v1-yaw270         --camera-yaw 270 --camera-zoom 1.5
shoot v2-yaw180         --camera-yaw 180 --camera-zoom 1.5
shoot v3-yaw090         --camera-yaw 90  --camera-zoom 1.5
shoot v4-yaw270-zoom3p5 --camera-yaw 270 --camera-zoom 3.5
