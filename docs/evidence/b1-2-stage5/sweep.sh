#!/usr/bin/env bash
# Regenerates the scripted camera sweep in sweep/ (18 PNGs). It stands in for the
# hands-on rotate/zoom check: same orbit/zoom functions as mouse navigation, but
# no mouse input. Usage (from the repo root):
#   docs/evidence/b1-2-stage5/sweep.sh [path/to/01-svf-order.vtk] [path/to/svf-explorer]
# Build the binary first: cargo build -p svf-explorer --release
set -euo pipefail
VTK="${1:-$HOME/Downloads/rodero2021/01-svf-order.vtk}"
BIN="${2:-target/release/svf-explorer}"
[ -x "$BIN" ] || BIN="$BIN.exe"
OUT="$(dirname "$0")/sweep"
mkdir -p "$OUT"

GLYPHS=(--vtk-cell-axial fibres --vtk-cell-tags ID=1,2)
RHO=(--vtk-scalar RHO.dat --vtk-scalar-exclude -10)

rho()   { local n="$1"; shift; "$BIN" --vtk "$VTK" "${GLYPHS[@]}" "${RHO[@]}" "$@" --screenshot "$OUT/$n.png"; }
plain() { local n="$1"; shift; "$BIN" --vtk "$VTK" "${GLYPHS[@]}" "$@" --screenshot "$OUT/$n.png"; }

# Full yaw sweep in 45-degree steps.
for y in 000 045 090 135 180 225 270 315; do rho "s-yaw$y" --camera-yaw $((10#$y)) --camera-zoom 1.5; done
# Above and below (allowed pitch for this scene: -118.35 to 59.36).
rho p-yaw270-pitch+55  --camera-yaw 270 --camera-pitch 55  --camera-zoom 1.5
rho p-yaw270-pitch-60  --camera-yaw 270 --camera-pitch -60 --camera-zoom 1.5
rho p-yaw270-pitch-110 --camera-yaw 270 --camera-pitch -110 --camera-zoom 1.5
# Oblique close-ups.
rho   o-yaw225-pitch+30-z3p5       --camera-yaw 225 --camera-pitch 30  --camera-zoom 3.5
rho   o-yaw315-pitch-30-z3p5       --camera-yaw 315 --camera-pitch -30 --camera-zoom 3.5
rho   o-yaw135-pitch+20-z3p5       --camera-yaw 135 --camera-pitch 20  --camera-zoom 3.5
plain o-yaw225-pitch+30-z3p5-plain --camera-yaw 225 --camera-pitch 30  --camera-zoom 3.5
# Zoom extremes (allowed zoom: 0.01 to 100).
rho z-yaw270-z0p5 --camera-yaw 270 --camera-zoom 0.5
rho z-yaw270-z8   --camera-yaw 270 --camera-zoom 8
rho z-yaw270-z20  --camera-yaw 270 --camera-zoom 20
