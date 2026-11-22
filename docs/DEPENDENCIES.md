# Dependency decisions

Versions below are the selected baseline for the initial implementation.
Re-check current release notes before upgrading major versions.

| Dependency | Version | Purpose | Scope |
|---|---:|---|---|
| wgpu | 30.0.1 | Safe cross-platform GPU API; Windows Vulkan/DX12 backends | renderer |
| winit | 0.30.13 | Native window and event lifecycle | renderer / explorer |
| pollster | 0.4 | Block on async GPU initialization in the bootstrap app | explorer |

No general numerical array crate or ECS is included yet.

The selected wgpu release declares Rust 1.87 as its library MSRV. The workspace
uses Rust 1.87 as its baseline. Windows builds should use the MSVC target and
a current Windows SDK.

References:
- https://docs.rs/wgpu/30.0.1/wgpu/
- https://docs.rs/winit/0.30.13/winit/
- https://docs.rs/pollster/0.4.0/pollster/

## Added for B2 step 2

| Dependency | Version | Purpose | Scope |
|---|---:|---|---|
| serde | =1.0.229 (derive) | Deserialize structure files | svf-core |
| ron | =0.12.3 | Rusty Object Notation parser for structure files (accepted B2 decision) | svf-core |

## Decided for labels (2026-11-22, slice L3; see `docs/labels.md`)

| Dependency | Version | Purpose | Scope |
|---|---:|---|---|
| fontdue | 0.9.5 | Rasterise glyphs for the text overlay (labels, key, captions) | svf-render |
| DejaVu Sans (font file, not a crate) | 2.37 | Embedded font; Bitstream Vera licence (bundling allowed), licence text shipped next to it | svf-render |

Chosen by the user over Noto Sans and over deferring. `glyphon` was rejected because it brings a whole shaping stack and is tied to one wgpu release.

