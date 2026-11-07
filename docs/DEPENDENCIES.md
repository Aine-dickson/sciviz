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
