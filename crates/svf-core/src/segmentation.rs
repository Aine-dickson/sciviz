//! Independent binary segmentation layers aligned to a regular image volume.
//!
//! Each segment owns its own packed binary mask. Segments share one image
//! geometry but are not forced into a mutually exclusive labelmap, so
//! overlapping segment membership is representable.

use std::{error::Error, fmt};

use crate::image_volume::ImageVolumeGeometry3;

/// Stable identifier for one segment within a segmentation volume.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SegmentId3(u32);

impl SegmentId3 {
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Errors when constructing a packed binary mask.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryMaskError3 {
    /// The supplied word count does not match the voxel count.
    WordCountMismatch { expected: usize, actual: usize },
    /// Unused high bits in the final word must be zero.
    NonZeroPaddingBits,
    /// A foreground voxel index lies outside the mask.
    IndexOutOfBounds { index: usize, voxel_count: usize },
}

impl fmt::Display for BinaryMaskError3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WordCountMismatch { expected, actual } => write!(
                f,
                "binary mask requires {expected} words; received {actual}"
            ),
            Self::NonZeroPaddingBits => {
                write!(f, "binary mask has non-zero bits beyond its voxel count")
            }
            Self::IndexOutOfBounds { index, voxel_count } => write!(
                f,
                "binary mask index {index} is outside voxel count {voxel_count}"
            ),
        }
    }
}

impl Error for BinaryMaskError3 {}

/// A compact binary mask using one bit per voxel.
///
/// Bit index zero represents linear voxel index zero. The bit ordering
/// therefore follows ImageVolumeGeometry3's i-fastest linear storage order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BinaryMask3 {
    voxel_count: usize,
    words: Vec<u64>,
    foreground_count: usize,
}

impl BinaryMask3 {
    /// Creates an all-background mask for the requested voxel count.
    #[must_use]
    pub fn empty(voxel_count: usize) -> Self {
        let word_count = voxel_count / 64 + usize::from(voxel_count % 64 != 0);
        Self {
            voxel_count,
            words: vec![0; word_count],
            foreground_count: 0,
        }
    }

    /// Creates a mask from packed words, validating its exact size and padding.
    pub fn from_words(
        voxel_count: usize,
        words: Vec<u64>,
    ) -> Result<Self, BinaryMaskError3> {
        let expected = voxel_count / 64 + usize::from(voxel_count % 64 != 0);
        if words.len() != expected {
            return Err(BinaryMaskError3::WordCountMismatch {
                expected,
                actual: words.len(),
            });
        }

        if voxel_count % 64 != 0 {
            let valid_bits = voxel_count % 64;
            let valid_mask = (1_u64 << valid_bits) - 1;
            if words.last().is_some_and(|word| word & !valid_mask != 0) {
                return Err(BinaryMaskError3::NonZeroPaddingBits);
            }
        }

        let foreground_count = words.iter().map(|word| word.count_ones() as usize).sum();
        Ok(Self {
            voxel_count,
            words,
            foreground_count,
        })
    }

    /// Creates a mask with foreground membership at the supplied linear indices.
    pub fn from_indices(
        voxel_count: usize,
        indices: impl IntoIterator<Item = usize>,
    ) -> Result<Self, BinaryMaskError3> {
        let mut mask = Self::empty(voxel_count);
        for index in indices {
            if index >= voxel_count {
                return Err(BinaryMaskError3::IndexOutOfBounds {
                    index,
                    voxel_count,
                });
            }
            let word = index / 64;
            let bit = index % 64;
            let flag = 1_u64 << bit;
            if mask.words[word] & flag == 0 {
                mask.words[word] |= flag;
                mask.foreground_count += 1;
            }
        }
        Ok(mask)
    }

    #[must_use]
    pub const fn voxel_count(&self) -> usize {
        self.voxel_count
    }

    #[must_use]
    pub const fn foreground_count(&self) -> usize {
        self.foreground_count
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.foreground_count == 0
    }

    /// Returns foreground membership, or None when the index is out of bounds.
    #[must_use]
    pub fn get(&self, linear_index: usize) -> Option<bool> {
        if linear_index >= self.voxel_count {
            return None;
        }
        let word = self.words[linear_index / 64];
        Some(word & (1_u64 << (linear_index % 64)) != 0)
    }

    /// Returns the packed storage as an immutable slice.
    #[must_use]
    pub fn words(&self) -> &[u64] {
        &self.words
    }
}

/// One independently addressable binary segment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segment3 {
    id: SegmentId3,
    name: String,
    mask: BinaryMask3,
}

impl Segment3 {
    pub fn new(
        id: SegmentId3,
        name: impl Into<String>,
        mask: BinaryMask3,
    ) -> Result<Self, SegmentationError3> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(SegmentationError3::EmptySegmentName { id });
        }
        Ok(Self { id, name, mask })
    }

    #[must_use]
    pub const fn id(&self) -> SegmentId3 {
        self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn mask(&self) -> &BinaryMask3 {
        &self.mask
    }
}

/// Errors produced when constructing a segmentation volume.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SegmentationError3 {
    /// A segment has no usable display name.
    EmptySegmentName { id: SegmentId3 },
    /// Two segments use the same identifier.
    DuplicateSegmentId { id: SegmentId3 },
    /// A segment mask does not match the shared geometry.
    MaskVoxelCountMismatch { id: SegmentId3, expected: usize, actual: usize },
}

impl fmt::Display for SegmentationError3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySegmentName { id } => {
                write!(f, "segment {} must have a non-empty name", id.get())
            }
            Self::DuplicateSegmentId { id } => {
                write!(f, "segment identifier {} is duplicated", id.get())
            }
            Self::MaskVoxelCountMismatch { id, expected, actual } => write!(
                f,
                "segment {} requires {expected} mask voxels; received {actual}",
                id.get()
            ),
        }
    }
}

impl Error for SegmentationError3 {}

/// A collection of independent binary segments sharing one regular 3D geometry.
///
/// Segment masks are intentionally independent: a voxel may belong to more
/// than one segment. No implicit priority or labelmap conversion is applied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SegmentationVolume3 {
    geometry: ImageVolumeGeometry3,
    segments: Vec<Segment3>,
}

impl SegmentationVolume3 {
    pub fn new(
        geometry: ImageVolumeGeometry3,
        segments: Vec<Segment3>,
    ) -> Result<Self, SegmentationError3> {
        for (index, segment) in segments.iter().enumerate() {
            if segments[..index].iter().any(|previous| previous.id == segment.id) {
                return Err(SegmentationError3::DuplicateSegmentId { id: segment.id });
            }
            if segment.mask.voxel_count() != geometry.voxel_count() {
                return Err(SegmentationError3::MaskVoxelCountMismatch {
                    id: segment.id,
                    expected: geometry.voxel_count(),
                    actual: segment.mask.voxel_count(),
                });
            }
        }

        Ok(Self { geometry, segments })
    }

    #[must_use]
    pub const fn geometry(&self) -> &ImageVolumeGeometry3 {
        &self.geometry
    }

    #[must_use]
    pub fn segments(&self) -> &[Segment3] {
        &self.segments
    }

    #[must_use]
    pub fn segment(&self, id: SegmentId3) -> Option<&Segment3> {
        self.segments.iter().find(|segment| segment.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spatial::{DMat3, WorldPoint3};

    fn geometry(dimensions: [usize; 3]) -> ImageVolumeGeometry3 {
        ImageVolumeGeometry3::new(dimensions, WorldPoint3::ORIGIN, DMat3::IDENTITY).unwrap()
    }

    #[test]
    fn packed_mask_tracks_membership_and_foreground_count() {
        let mask = BinaryMask3::from_indices(130, [0, 63, 64, 129, 129]).unwrap();
        assert_eq!(mask.foreground_count(), 4);
        assert_eq!(mask.get(0), Some(true));
        assert_eq!(mask.get(1), Some(false));
        assert_eq!(mask.get(63), Some(true));
        assert_eq!(mask.get(64), Some(true));
        assert_eq!(mask.get(129), Some(true));
        assert_eq!(mask.get(130), None);
        assert_eq!(mask.words().len(), 3);
    }

    #[test]
    fn rejects_invalid_packed_word_count_and_padding() {
        assert_eq!(
            BinaryMask3::from_words(65, vec![0]),
            Err(BinaryMaskError3::WordCountMismatch {
                expected: 2,
                actual: 1
            })
        );
        assert_eq!(
            BinaryMask3::from_words(65, vec![0, 2]),
            Err(BinaryMaskError3::NonZeroPaddingBits)
        );
    }

    #[test]
    fn rejects_out_of_bounds_foreground_indices() {
        assert_eq!(
            BinaryMask3::from_indices(4, [4]),
            Err(BinaryMaskError3::IndexOutOfBounds {
                index: 4,
                voxel_count: 4
            })
        );
    }

    #[test]
    fn segmentation_volume_requires_matching_mask_sizes() {
        let segment = Segment3::new(
            SegmentId3::new(1),
            "Kidney",
            BinaryMask3::empty(7),
        )
        .unwrap();
        assert_eq!(
            SegmentationVolume3::new(geometry([2, 2, 2]), vec![segment]).unwrap_err(),
            SegmentationError3::MaskVoxelCountMismatch {
                id: SegmentId3::new(1),
                expected: 8,
                actual: 7
            }
        );
    }

    #[test]
    fn segments_remain_independent_and_can_overlap() {
        let kidney = Segment3::new(
            SegmentId3::new(1),
            "Kidney",
            BinaryMask3::from_indices(8, [1, 2, 3]).unwrap(),
        )
        .unwrap();
        let mass = Segment3::new(
            SegmentId3::new(2),
            "Mass",
            BinaryMask3::from_indices(8, [3, 4]).unwrap(),
        )
        .unwrap();

        let segmentation =
            SegmentationVolume3::new(geometry([2, 2, 2]), vec![kidney, mass]).unwrap();

        assert_eq!(segmentation.segments().len(), 2);
        assert_eq!(
            segmentation.segment(SegmentId3::new(1)).unwrap().mask().get(3),
            Some(true)
        );
        assert_eq!(
            segmentation.segment(SegmentId3::new(2)).unwrap().mask().get(3),
            Some(true)
        );
        assert_eq!(segmentation.segment(SegmentId3::new(99)), None);
    }

    #[test]
    fn rejects_duplicate_segment_identifiers_and_empty_names() {
        let empty_name = Segment3::new(
            SegmentId3::new(1),
            "  ",
            BinaryMask3::empty(1),
        );
        assert_eq!(
            empty_name.unwrap_err(),
            SegmentationError3::EmptySegmentName {
                id: SegmentId3::new(1)
            }
        );

        let first = Segment3::new(
            SegmentId3::new(7),
            "First",
            BinaryMask3::empty(1),
        )
        .unwrap();
        let second = Segment3::new(
            SegmentId3::new(7),
            "Second",
            BinaryMask3::empty(1),
        )
        .unwrap();
        assert_eq!(
            SegmentationVolume3::new(geometry([1, 1, 1]), vec![first, second]).unwrap_err(),
            SegmentationError3::DuplicateSegmentId {
                id: SegmentId3::new(7)
            }
        );
    }
}
