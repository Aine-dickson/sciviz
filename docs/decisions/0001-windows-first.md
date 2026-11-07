# ADR 0001: Windows-first native target

- Status: Accepted
- Decision: Initial application target is Windows desktop.
- Rationale: Focus early validation on one native platform and keep the
  initial host integration small.
- Consequence: wgpu is configured for Vulkan and Direct3D 12 on Windows.
  Cross-platform support remains a future validation goal; the core data
  model must not assume Windows APIs.
