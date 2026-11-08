#![forbid(unsafe_code)]
#![doc = "Graphics-independent foundations for the Scientific Visualization Framework."]

pub mod boundary_surface;
pub mod camera;
pub mod geometry;
pub mod fields;
pub mod dataset;
pub mod ray;
pub mod sampling;
pub mod spatial;
pub mod surface_triangulation;
pub mod unstructured;
pub mod unstructured_topology;

/// Human-readable project name.
pub const PROJECT_NAME: &str = "Scientific Visualization Framework";

/// Current workspace version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_project_identity() {
        assert_eq!(PROJECT_NAME, "Scientific Visualization Framework");
        assert!(!VERSION.is_empty());
    }
}
