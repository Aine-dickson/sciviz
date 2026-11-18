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


/// A voxel sample in its original numeric representation.
///
/// Integer variants preserve exact source values. Conversion to f64 is
/// available for numerical algorithms, but may lose precision for large
/// 64-bit integers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VoxelValue3 {
    UInt8(u8),
    Int8(i8),
    UInt16(u16),
    Int16(i16),
    UInt32(u32),
    Int32(i32),
    UInt64(u64),
    Int64(i64),
    Float32(f32),
    Float64(f64),
}

impl VoxelValue3 {
    /// Converts this sample to f64 for numerical processing.
    ///
    /// This conversion is exact for 8-, 16-, and 32-bit integers, but not
    /// necessarily for all 64-bit integers or all f64/f32 values.
    #[must_use]
    pub fn as_f64(self) -> f64 {
        match self {
            Self::UInt8(value) => f64::from(value),
            Self::Int8(value) => f64::from(value),
            Self::UInt16(value) => f64::from(value),
            Self::Int16(value) => f64::from(value),
            Self::UInt32(value) => f64::from(value),
            Self::Int32(value) => f64::from(value),
            Self::UInt64(value) => value as f64,
            Self::Int64(value) => value as f64,
            Self::Float32(value) => f64::from(value),
            Self::Float64(value) => value,
        }
    }
}

/// Contiguous, homogeneous voxel storage.
///
/// The vector order must match ImageVolumeGeometry3's linear order:
/// i-fastest, then j, then k. This enum stores native numeric values rather
/// than expanding every sample to f64.
#[derive(Clone, Debug, PartialEq)]
pub enum VoxelBuffer3 {
    UInt8(Vec<u8>),
    Int8(Vec<i8>),
    UInt16(Vec<u16>),
    Int16(Vec<i16>),
    UInt32(Vec<u32>),
    Int32(Vec<i32>),
    UInt64(Vec<u64>),
    Int64(Vec<i64>),
    Float32(Vec<f32>),
    Float64(Vec<f64>),
}

impl VoxelBuffer3 {
    /// Returns the number of stored voxel samples.
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::UInt8(values) => values.len(),
            Self::Int8(values) => values.len(),
            Self::UInt16(values) => values.len(),
            Self::Int16(values) => values.len(),
            Self::UInt32(values) => values.len(),
            Self::Int32(values) => values.len(),
            Self::UInt64(values) => values.len(),
            Self::Int64(values) => values.len(),
            Self::Float32(values) => values.len(),
            Self::Float64(values) => values.len(),
        }
    }

    /// Returns whether the buffer contains no samples.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the sample at a linear index without changing its numeric type.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<VoxelValue3> {
        match self {
            Self::UInt8(values) => values.get(index).copied().map(VoxelValue3::UInt8),
            Self::Int8(values) => values.get(index).copied().map(VoxelValue3::Int8),
            Self::UInt16(values) => values.get(index).copied().map(VoxelValue3::UInt16),
            Self::Int16(values) => values.get(index).copied().map(VoxelValue3::Int16),
            Self::UInt32(values) => values.get(index).copied().map(VoxelValue3::UInt32),
            Self::Int32(values) => values.get(index).copied().map(VoxelValue3::Int32),
            Self::UInt64(values) => values.get(index).copied().map(VoxelValue3::UInt64),
            Self::Int64(values) => values.get(index).copied().map(VoxelValue3::Int64),
            Self::Float32(values) => values.get(index).copied().map(VoxelValue3::Float32),
            Self::Float64(values) => values.get(index).copied().map(VoxelValue3::Float64),
        }
    }
}

/// A regular 3D image volume with geometry and native typed voxel storage.
///
/// This type does not apply modality calibration, interpret segmentation
/// labels, or perform interpolation. Those concerns remain explicit layers
/// above the source samples.
#[derive(Clone, Debug, PartialEq)]
pub struct ImageVolume3 {
    geometry: ImageVolumeGeometry3,
    voxels: VoxelBuffer3,
}

/// Errors produced when constructing an image volume.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageVolumeError3 {
    /// The voxel buffer length does not match the geometry's voxel count.
    VoxelCountMismatch { expected: usize, actual: usize },
}

impl fmt::Display for ImageVolumeError3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VoxelCountMismatch { expected, actual } => write!(
                f,
                "image volume requires {expected} voxel samples; received {actual}"
            ),
        }
    }
}

impl Error for ImageVolumeError3 {}

impl ImageVolume3 {
    /// Creates a volume after checking that storage matches its geometry.
    pub fn new(
        geometry: ImageVolumeGeometry3,
        voxels: VoxelBuffer3,
    ) -> Result<Self, ImageVolumeError3> {
        if voxels.len() != geometry.voxel_count() {
            return Err(ImageVolumeError3::VoxelCountMismatch {
                expected: geometry.voxel_count(),
                actual: voxels.len(),
            });
        }
        Ok(Self { geometry, voxels })
    }

    /// Returns the volume's immutable geometry.
    #[must_use]
    pub const fn geometry(&self) -> &ImageVolumeGeometry3 {
        &self.geometry
    }

    /// Returns the original typed voxel buffer.
    #[must_use]
    pub const fn voxels(&self) -> &VoxelBuffer3 {
        &self.voxels
    }

    /// Returns the stored sample at an in-bounds integer voxel index.
    #[must_use]
    pub fn voxel(&self, index: [usize; 3]) -> Option<VoxelValue3> {
        let linear = self.geometry.linear_index(index)?;
        self.voxels.get(linear)
    }

    /// Returns the stored sample at a linear storage index.
    #[must_use]
    pub fn voxel_linear(&self, linear_index: usize) -> Option<VoxelValue3> {
        self.voxels.get(linear_index)
    }
}

#[cfg(test)]
mod storage_tests {
    use super::*;

    fn geometry(dimensions: [usize; 3]) -> ImageVolumeGeometry3 {
        ImageVolumeGeometry3::new(
            dimensions,
            WorldPoint3::ORIGIN,
            DMat3::IDENTITY,
        )
        .unwrap()
    }

    #[test]
    fn preserves_native_sample_types_and_values() {
        let cases = [
            (VoxelBuffer3::UInt8(vec![255]), VoxelValue3::UInt8(255)),
            (VoxelBuffer3::Int8(vec![-8]), VoxelValue3::Int8(-8)),
            (VoxelBuffer3::UInt16(vec![4095]), VoxelValue3::UInt16(4095)),
            (VoxelBuffer3::Int16(vec![-1024]), VoxelValue3::Int16(-1024)),
            (VoxelBuffer3::UInt32(vec![70000]), VoxelValue3::UInt32(70000)),
            (VoxelBuffer3::Int32(vec![-70000]), VoxelValue3::Int32(-70000)),
            (VoxelBuffer3::UInt64(vec![u64::MAX]), VoxelValue3::UInt64(u64::MAX)),
            (VoxelBuffer3::Int64(vec![i64::MIN]), VoxelValue3::Int64(i64::MIN)),
            (VoxelBuffer3::Float32(vec![1.25]), VoxelValue3::Float32(1.25)),
            (VoxelBuffer3::Float64(vec![1.25]), VoxelValue3::Float64(1.25)),
        ];

        for (buffer, expected) in cases {
            assert_eq!(buffer.get(0), Some(expected));
            assert_eq!(buffer.len(), 1);
        }
    }

    #[test]
    fn converts_common_integer_samples_to_f64() {
        assert_eq!(VoxelValue3::UInt16(2957).as_f64(), 2957.0);
        assert_eq!(VoxelValue3::Int16(-1024).as_f64(), -1024.0);
        assert_eq!(VoxelValue3::Float32(0.5).as_f64(), 0.5);
    }

    #[test]
    fn image_volume_requires_exactly_one_sample_per_voxel() {
        assert_eq!(
            ImageVolume3::new(
                geometry([2, 2, 2]),
                VoxelBuffer3::UInt16(vec![0; 7]),
            )
            .unwrap_err(),
            ImageVolumeError3::VoxelCountMismatch {
                expected: 8,
                actual: 7,
            }
        );

        let volume = ImageVolume3::new(
            geometry([2, 2, 2]),
            VoxelBuffer3::UInt16(vec![0; 8]),
        )
        .unwrap();
        assert_eq!(volume.geometry().voxel_count(), 8);
        assert_eq!(volume.voxels().len(), 8);
    }

    #[test]
    fn samples_volume_by_voxel_and_linear_index() {
        let volume = ImageVolume3::new(
            geometry([2, 2, 2]),
            VoxelBuffer3::UInt16(vec![10, 11, 12, 13, 14, 15, 16, 17]),
        )
        .unwrap();

        assert_eq!(volume.voxel([1, 0, 0]), Some(VoxelValue3::UInt16(11)));
        assert_eq!(volume.voxel([0, 1, 1]), Some(VoxelValue3::UInt16(16)));
        assert_eq!(volume.voxel([2, 0, 0]), None);
        assert_eq!(volume.voxel_linear(7), Some(VoxelValue3::UInt16(17)));
        assert_eq!(volume.voxel_linear(8), None);
    }
}
