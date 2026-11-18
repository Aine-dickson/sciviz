//! Geometry and coordinate transforms for regular three-dimensional image volumes.
//!
//! An image volume stores samples on a regular index lattice. The origin is
//! the world-space centre of voxel (0, 0, 0), and the columns of the
//! index-to-world basis describe the world displacement for one index step
//! along each axis. Spacing and orientation are therefore represented
//! together, without assuming axis alignment or a particular world convention.
//!
//! Voxel storage and intensity calibration are separate concerns and are
//! introduced by higher-level volume types.

use std::{error::Error, fmt};

use crate::spatial::{DMat3, DVec3, WorldPoint3};

/// Geometry validation errors for a regular image volume.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageVolumeGeometryError3 {
    /// Every image dimension must contain at least one sample.
    EmptyDimension { axis: usize },
    /// The product of the three dimensions cannot be represented by usize.
    VoxelCountOverflow,
    /// The origin contains a non-finite coordinate.
    NonFiniteOrigin,
    /// The index-to-world basis contains a non-finite component.
    NonFiniteBasis,
    /// The index-to-world basis is singular or its inverse is not finite.
    NonInvertibleBasis,
}

impl fmt::Display for ImageVolumeGeometryError3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyDimension { axis } => {
                write!(f, "image volume dimension {axis} must be greater than zero")
            }
            Self::VoxelCountOverflow => {
                write!(f, "image volume voxel count overflows usize")
            }
            Self::NonFiniteOrigin => {
                write!(f, "image volume origin must contain only finite coordinates")
            }
            Self::NonFiniteBasis => {
                write!(f, "image volume index-to-world basis must be finite")
            }
            Self::NonInvertibleBasis => {
                write!(f, "image volume index-to-world basis must be invertible")
            }
        }
    }
}

impl Error for ImageVolumeGeometryError3 {}

/// Regular 3D image-lattice geometry, independent of voxel storage.
///
/// Dimensions count voxel sample centres along index axes (i, j, k).
/// Linear storage order is i-fastest, then j, then k:
/// i + nx * (j + ny * k).
///
/// The origin is the world-space centre of voxel (0, 0, 0). The three columns
/// of the index-to-world basis are the world-space displacements for one index
/// step along i, j, and k. Each column therefore includes both axis
/// orientation and physical spacing. The basis may represent oblique or
/// sheared lattices; it is not restricted to axis-aligned images.
///
/// The inverse basis is cached so repeated world-to-index queries do not
/// repeatedly invert the matrix. Geometry is immutable after construction.
#[derive(Clone, Debug, PartialEq)]
pub struct ImageVolumeGeometry3 {
    dimensions: [usize; 3],
    voxel_count: usize,
    origin: WorldPoint3,
    index_to_world_basis: DMat3,
    world_to_index_basis: DMat3,
}

impl ImageVolumeGeometry3 {
    /// Creates a regular image geometry after validating dimensions and its
    /// affine index-to-world transform.
    ///
    /// Matrix columns correspond to index axes (i, j, k). The origin is the
    /// centre of the first voxel, not the outer corner of the volume.
    pub fn new(
        dimensions: [usize; 3],
        origin: WorldPoint3,
        index_to_world_basis: DMat3,
    ) -> Result<Self, ImageVolumeGeometryError3> {
        for (axis, dimension) in dimensions.iter().copied().enumerate() {
            if dimension == 0 {
                return Err(ImageVolumeGeometryError3::EmptyDimension { axis });
            }
        }

        let voxel_count = dimensions[0]
            .checked_mul(dimensions[1])
            .and_then(|count| count.checked_mul(dimensions[2]))
            .ok_or(ImageVolumeGeometryError3::VoxelCountOverflow)?;

        if !origin.to_array().iter().all(|value| value.is_finite()) {
            return Err(ImageVolumeGeometryError3::NonFiniteOrigin);
        }

        if !index_to_world_basis
            .to_cols_array()
            .iter()
            .all(|value| value.is_finite())
        {
            return Err(ImageVolumeGeometryError3::NonFiniteBasis);
        }

        let determinant = index_to_world_basis.determinant();
        if !determinant.is_finite() || determinant == 0.0 {
            return Err(ImageVolumeGeometryError3::NonInvertibleBasis);
        }

        let world_to_index_basis = index_to_world_basis.inverse();
        if !world_to_index_basis
            .to_cols_array()
            .iter()
            .all(|value| value.is_finite())
        {
            return Err(ImageVolumeGeometryError3::NonInvertibleBasis);
        }

        Ok(Self {
            dimensions,
            voxel_count,
            origin,
            index_to_world_basis,
            world_to_index_basis,
        })
    }

    /// Returns the sample dimensions in (i, j, k) order.
    #[must_use]
    pub const fn dimensions(&self) -> [usize; 3] {
        self.dimensions
    }

    /// Returns the total number of voxel sample centres.
    #[must_use]
    pub const fn voxel_count(&self) -> usize {
        self.voxel_count
    }

    /// Returns the world-space centre of voxel (0, 0, 0).
    #[must_use]
    pub const fn origin(&self) -> WorldPoint3 {
        self.origin
    }

    /// Returns the index-to-world basis. Its columns correspond to (i, j, k).
    #[must_use]
    pub const fn index_to_world_basis(&self) -> DMat3 {
        self.index_to_world_basis
    }

    /// Returns the cached world-to-index basis.
    #[must_use]
    pub const fn world_to_index_basis(&self) -> DMat3 {
        self.world_to_index_basis
    }

    /// Returns the linear storage index for an in-bounds integer voxel index.
    ///
    /// The first index (i) varies fastest, followed by j, then k.
    #[must_use]
    pub fn linear_index(&self, index: [usize; 3]) -> Option<usize> {
        let [i, j, k] = index;
        let [nx, ny, nz] = self.dimensions;
        if i >= nx || j >= ny || k >= nz {
            return None;
        }
        Some(i + nx * (j + ny * k))
    }

    /// Converts an in-bounds linear storage index to (i, j, k).
    #[must_use]
    pub fn voxel_index(&self, linear_index: usize) -> Option<[usize; 3]> {
        if linear_index >= self.voxel_count {
            return None;
        }
        let [nx, ny, _] = self.dimensions;
        let plane = nx * ny;
        let k = linear_index / plane;
        let within_plane = linear_index % plane;
        let j = within_plane / nx;
        let i = within_plane % nx;
        Some([i, j, k])
    }

    /// Maps continuous voxel-index coordinates to a world-space position.
    ///
    /// Integer coordinates identify voxel centres. Coordinates are not
    /// restricted to the volume bounds, which permits callers to transform
    /// points on voxel faces or outside the image. Returns None if the input
    /// or transformed world position is non-finite.
    #[must_use]
    pub fn index_to_world(&self, index: [f64; 3]) -> Option<WorldPoint3> {
        if !index.iter().all(|value| value.is_finite()) {
            return None;
        }
        let displacement = self.index_to_world_basis * DVec3::from_array(index);
        let world = self.origin.to_vector() + displacement;
        if !world.is_finite() {
            return None;
        }
        Some(WorldPoint3::from_vector(world))
    }

    /// Maps a world-space position to continuous voxel-index coordinates.
    ///
    /// Integer results identify voxel centres. The returned coordinates are
    /// not rounded or clamped and may lie outside the volume. Returns None
    /// if the input or transformed index is non-finite.
    #[must_use]
    pub fn world_to_continuous_index(&self, world: WorldPoint3) -> Option<[f64; 3]> {
        if !world.to_array().iter().all(|value| value.is_finite()) {
            return None;
        }
        let displacement = world.to_vector() - self.origin.to_vector();
        let index = self.world_to_index_basis * displacement;
        if !index.is_finite() {
            return None;
        }
        Some(index.to_array())
    }

    /// Returns the world-space centre of an in-bounds voxel.
    #[must_use]
    pub fn voxel_center(&self, index: [usize; 3]) -> Option<WorldPoint3> {
        self.linear_index(index)?;
        self.index_to_world(index.map(|value| value as f64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity_geometry(dimensions: [usize; 3]) -> ImageVolumeGeometry3 {
        ImageVolumeGeometry3::new(
            dimensions,
            WorldPoint3::ORIGIN,
            DMat3::IDENTITY,
        )
        .unwrap()
    }

    #[test]
    fn validates_nonzero_dimensions_and_checked_voxel_count() {
        assert_eq!(
            ImageVolumeGeometry3::new(
                [4, 0, 2],
                WorldPoint3::ORIGIN,
                DMat3::IDENTITY,
            )
            .unwrap_err(),
            ImageVolumeGeometryError3::EmptyDimension { axis: 1 }
        );

        assert_eq!(
            ImageVolumeGeometry3::new(
                [usize::MAX, 2, 1],
                WorldPoint3::ORIGIN,
                DMat3::IDENTITY,
            )
            .unwrap_err(),
            ImageVolumeGeometryError3::VoxelCountOverflow
        );
    }

    #[test]
    fn rejects_non_finite_origin_and_basis() {
        assert_eq!(
            ImageVolumeGeometry3::new(
                [1, 1, 1],
                WorldPoint3::new(f64::NAN, 0.0, 0.0),
                DMat3::IDENTITY,
            )
            .unwrap_err(),
            ImageVolumeGeometryError3::NonFiniteOrigin
        );

        let non_finite_basis = DMat3::from_cols(
            DVec3::X,
            DVec3::Y,
            DVec3::new(0.0, 0.0, f64::INFINITY),
        );
        assert_eq!(
            ImageVolumeGeometry3::new(
                [1, 1, 1],
                WorldPoint3::ORIGIN,
                non_finite_basis,
            )
            .unwrap_err(),
            ImageVolumeGeometryError3::NonFiniteBasis
        );
    }

    #[test]
    fn rejects_singular_basis() {
        let singular = DMat3::from_cols(DVec3::X, DVec3::X, DVec3::Z);
        assert_eq!(
            ImageVolumeGeometry3::new(
                [2, 2, 2],
                WorldPoint3::ORIGIN,
                singular,
            )
            .unwrap_err(),
            ImageVolumeGeometryError3::NonInvertibleBasis
        );
    }

    #[test]
    fn maps_indices_through_oblique_affine_basis_and_back() {
        let basis = DMat3::from_cols(
            DVec3::new(0.0, 2.0, 0.0),
            DVec3::new(-3.0, 0.0, 0.0),
            DVec3::new(0.25, 0.0, 0.5),
        );
        let geometry = ImageVolumeGeometry3::new(
            [8, 6, 4],
            WorldPoint3::new(10.0, 20.0, 30.0),
            basis,
        )
        .unwrap();

        let world = geometry.index_to_world([2.0, 3.0, 4.0]).unwrap();
        assert_eq!(world.to_array(), [2.0, 24.0, 32.0]);

        let recovered = geometry.world_to_continuous_index(world).unwrap();
        for (actual, expected) in recovered.into_iter().zip([2.0, 3.0, 4.0]) {
            assert!((actual - expected).abs() < 1.0e-12);
        }
    }

    #[test]
    fn uses_i_fastest_linear_storage_order_and_round_trips_indices() {
        let geometry = identity_geometry([3, 4, 2]);
        assert_eq!(geometry.voxel_count(), 24);
        assert_eq!(geometry.linear_index([2, 1, 1]), Some(17));
        assert_eq!(geometry.voxel_index(17), Some([2, 1, 1]));
        assert_eq!(geometry.voxel_index(24), None);
        assert_eq!(geometry.linear_index([3, 0, 0]), None);

        for linear in 0..geometry.voxel_count() {
            let index = geometry.voxel_index(linear).unwrap();
            assert_eq!(geometry.linear_index(index), Some(linear));
        }
    }

    #[test]
    fn voxel_centres_use_the_origin_as_the_first_sample_centre() {
        let geometry = ImageVolumeGeometry3::new(
            [2, 2, 2],
            WorldPoint3::new(10.0, -5.0, 2.0),
            DMat3::from_diagonal(DVec3::new(0.5, 1.0, 2.0)),
        )
        .unwrap();

        assert_eq!(
            geometry.voxel_center([1, 1, 1]).unwrap().to_array(),
            [10.5, -4.0, 4.0]
        );
        assert_eq!(geometry.voxel_center([2, 1, 1]), None);
    }

    #[test]
    fn coordinate_transforms_reject_non_finite_inputs_and_outputs() {
        let geometry = identity_geometry([2, 2, 2]);
        assert_eq!(geometry.index_to_world([f64::NAN, 0.0, 0.0]), None);
        assert_eq!(
            geometry.world_to_continuous_index(WorldPoint3::new(0.0, f64::INFINITY, 0.0)),
            None
        );
    }
}
