#![forbid(unsafe_code)]
#![doc = "Graphics-independent foundations for the Scientific Visualization Framework."]

pub mod boundary_axial_glyphs;
pub mod boundary_surface;
pub mod camera;
pub mod camera_navigation;
pub mod chemistry;
pub mod chemistry_view;
pub mod dataset;
pub mod fields;
pub mod geometry;
pub mod isosurface;
pub mod isosurface_field_transfer;
pub mod ray;
pub mod sampling;
pub mod scalar_visualization;
pub mod spatial;
pub mod structure;
pub mod structure_section;
pub mod surface_inspection;
pub mod surface_triangulation;
pub mod unstructured;
pub mod unstructured_topology;
pub mod vector_glyphs;
pub mod vector_visualization;
pub mod vsepr;
pub mod vsepr_view;
pub mod vtk_legacy;

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
