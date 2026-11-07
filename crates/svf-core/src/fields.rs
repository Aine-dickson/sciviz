//! Scientific fields and their association with dataset entities.
//!
//! Fields are independent of geometry storage and rendering. They preserve
//! non-finite measurements explicitly and declare whether each value belongs
//! to a point, cell, or the dataset as a whole.

use std::{error::Error, fmt};

use crate::{
    sampling::SampleValue,
    spatial::WorldVector3,
};

/// Identifies the dataset entities to which field values belong.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FieldAssociation {
    /// One value for each dataset point.
    Point,
    /// One value for each logical dataset cell.
    Cell,
    /// One value describing the dataset as a whole.
    Dataset,
}

/// The coordinate basis used by a three-component vector field.
///
/// M6.1 supports world-coordinate vectors. Other coordinate frames require
/// explicit transform and frame semantics and are intentionally deferred.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VectorBasis3 {
    /// Components are expressed along the SVF world X, Y, and Z axes.
    World,
}

/// Descriptive metadata shared by scientific fields.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldMetadata {
    name: String,
    units: Option<String>,
}

impl FieldMetadata {
    /// Creates metadata with a non-empty name and optional non-empty units.
    pub fn new(name: impl Into<String>, units: Option<String>) -> Result<Self, FieldError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(FieldError::EmptyName);
        }
        if units.as_ref().is_some_and(|value| value.trim().is_empty()) {
            return Err(FieldError::EmptyUnits);
        }
        Ok(Self { name, units })
    }

    /// Returns the field's descriptive name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the optional unit label, without imposing a unit system.
    #[must_use]
    pub fn units(&self) -> Option<&str> {
        self.units.as_deref()
    }
}

/// Errors produced when constructing or validating scientific fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldError {
    /// Field names must contain at least one non-whitespace character.
    EmptyName,
    /// A supplied unit label must contain at least one non-whitespace character.
    EmptyUnits,
    /// A dataset-associated field must contain exactly one value.
    DatasetFieldRequiresOneValue { actual: usize },
    /// A point- or cell-associated field has the wrong number of values.
    CardinalityMismatch {
        association: FieldAssociation,
        expected: usize,
        actual: usize,
    },
}

impl fmt::Display for FieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => write!(f, "field name must not be empty"),
            Self::EmptyUnits => write!(f, "field units must not be empty when supplied"),
            Self::DatasetFieldRequiresOneValue { actual } => write!(
                f,
                "a dataset-associated field requires exactly one value; received {actual}"
            ),
            Self::CardinalityMismatch {
                association,
                expected,
                actual,
            } => write!(
                f,
                "{association:?}-associated field requires {expected} values; received {actual}"
            ),
        }
    }
}

impl Error for FieldError {}

/// A scalar quantity attached to dataset entities.
///
/// Values use [`SampleValue`] so NaN and positive/negative infinity remain
/// explicit entries instead of being dropped or silently converted.
#[derive(Clone, Debug, PartialEq)]
pub struct ScalarField {
    metadata: FieldMetadata,
    association: FieldAssociation,
    values: Vec<SampleValue>,
}

impl ScalarField {
    /// Creates a scalar field. Dataset-associated fields must contain one
    /// value; point/cell cardinality is checked when the field is attached to
    /// a dataset with known topology.
    pub fn new(
        metadata: FieldMetadata,
        association: FieldAssociation,
        values: Vec<SampleValue>,
    ) -> Result<Self, FieldError> {
        validate_dataset_cardinality(association, values.len())?;
        Ok(Self {
            metadata,
            association,
            values,
        })
    }

    /// Returns the field's metadata.
    #[must_use]
    pub const fn metadata(&self) -> &FieldMetadata {
        &self.metadata
    }

    /// Returns the entity association for this field.
    #[must_use]
    pub const fn association(&self) -> FieldAssociation {
        self.association
    }

    /// Returns values in their original entity order.
    #[must_use]
    pub fn values(&self) -> &[SampleValue] {
        &self.values
    }

    /// Returns the number of stored values.
    #[must_use]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Returns whether the field contains no values.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Checks value count against the supplied dataset point and cell counts.
    pub fn validate_cardinality(
        &self,
        point_count: usize,
        cell_count: usize,
    ) -> Result<(), FieldError> {
        validate_cardinality(
            self.association,
            self.values.len(),
            point_count,
            cell_count,
        )
    }
}

/// One three-component vector measurement.
///
/// Each component retains its own finite/non-finite classification. This
/// avoids losing valid components when only one component is invalid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VectorValue3 {
    components: [SampleValue; 3],
}

impl VectorValue3 {
    /// Creates a vector value from three already classified components.
    #[must_use]
    pub const fn new(x: SampleValue, y: SampleValue, z: SampleValue) -> Self {
        Self {
            components: [x, y, z],
        }
    }

    /// Classifies three f64 components without discarding non-finite values.
    #[must_use]
    pub fn from_f64(x: f64, y: f64, z: f64) -> Self {
        Self::new(
            SampleValue::from_f64(x),
            SampleValue::from_f64(y),
            SampleValue::from_f64(z),
        )
    }

    /// Returns the X, Y, and Z components in the declared vector basis.
    #[must_use]
    pub const fn components(self) -> [SampleValue; 3] {
        self.components
    }

    /// Returns a world vector only when all three components are finite.
    #[must_use]
    pub fn as_finite_world_vector(self) -> Option<WorldVector3> {
        let [SampleValue::Finite(x), SampleValue::Finite(y), SampleValue::Finite(z)] =
            self.components
        else {
            return None;
        };
        Some(WorldVector3::new(x, y, z))
    }
}

/// A three-component vector quantity attached to dataset entities.
#[derive(Clone, Debug, PartialEq)]
pub struct VectorField3 {
    metadata: FieldMetadata,
    association: FieldAssociation,
    basis: VectorBasis3,
    values: Vec<VectorValue3>,
}

impl VectorField3 {
    /// Creates a vector field. Dataset-associated fields must contain one
    /// value; point/cell cardinality is checked against dataset topology.
    pub fn new(
        metadata: FieldMetadata,
        association: FieldAssociation,
        basis: VectorBasis3,
        values: Vec<VectorValue3>,
    ) -> Result<Self, FieldError> {
        validate_dataset_cardinality(association, values.len())?;
        Ok(Self {
            metadata,
            association,
            basis,
            values,
        })
    }

    /// Returns the field's metadata.
    #[must_use]
    pub const fn metadata(&self) -> &FieldMetadata {
        &self.metadata
    }

    /// Returns the entity association for this field.
    #[must_use]
    pub const fn association(&self) -> FieldAssociation {
        self.association
    }

    /// Returns the coordinate basis of every vector in the field.
    #[must_use]
    pub const fn basis(&self) -> VectorBasis3 {
        self.basis
    }

    /// Returns vector values in their original entity order.
    #[must_use]
    pub fn values(&self) -> &[VectorValue3] {
        &self.values
    }

    /// Returns the number of stored vectors.
    #[must_use]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Returns whether the field contains no vectors.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Checks value count against the supplied dataset point and cell counts.
    pub fn validate_cardinality(
        &self,
        point_count: usize,
        cell_count: usize,
    ) -> Result<(), FieldError> {
        validate_cardinality(
            self.association,
            self.values.len(),
            point_count,
            cell_count,
        )
    }
}

fn validate_dataset_cardinality(
    association: FieldAssociation,
    actual: usize,
) -> Result<(), FieldError> {
    if association == FieldAssociation::Dataset && actual != 1 {
        return Err(FieldError::DatasetFieldRequiresOneValue { actual });
    }
    Ok(())
}

fn validate_cardinality(
    association: FieldAssociation,
    actual: usize,
    point_count: usize,
    cell_count: usize,
) -> Result<(), FieldError> {
    let expected = match association {
        FieldAssociation::Point => point_count,
        FieldAssociation::Cell => cell_count,
        FieldAssociation::Dataset => 1,
    };
    if actual == expected {
        Ok(())
    } else if association == FieldAssociation::Dataset {
        Err(FieldError::DatasetFieldRequiresOneValue { actual })
    } else {
        Err(FieldError::CardinalityMismatch {
            association,
            expected,
            actual,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sampling::NonFiniteValue;

    fn metadata(name: &str, units: Option<&str>) -> FieldMetadata {
        FieldMetadata::new(name, units.map(str::to_owned)).unwrap()
    }

    #[test]
    fn metadata_preserves_name_and_optional_units() {
        let field = metadata("temperature", Some("K"));
        assert_eq!(field.name(), "temperature");
        assert_eq!(field.units(), Some("K"));
        assert_eq!(metadata("pressure", None).units(), None);
    }

    #[test]
    fn metadata_rejects_empty_name_and_units() {
        assert_eq!(
            FieldMetadata::new("  ", None),
            Err(FieldError::EmptyName)
        );
        assert_eq!(
            FieldMetadata::new("temperature", Some("  ".to_owned())),
            Err(FieldError::EmptyUnits)
        );
    }

    #[test]
    fn scalar_field_preserves_order_and_non_finite_values() {
        let values = vec![
            SampleValue::Finite(273.15),
            SampleValue::NonFinite(NonFiniteValue::NaN),
            SampleValue::NonFinite(NonFiniteValue::PositiveInfinity),
            SampleValue::NonFinite(NonFiniteValue::NegativeInfinity),
        ];
        let field = ScalarField::new(
            metadata("temperature", Some("K")),
            FieldAssociation::Point,
            values.clone(),
        )
        .unwrap();
        assert_eq!(field.values(), values);
        assert_eq!(field.len(), 4);
        assert_eq!(field.association(), FieldAssociation::Point);
    }

    #[test]
    fn scalar_field_checks_point_and_cell_cardinality() {
        let point = ScalarField::new(
            metadata("temperature", None),
            FieldAssociation::Point,
            vec![SampleValue::Finite(1.0); 3],
        )
        .unwrap();
        assert_eq!(point.validate_cardinality(3, 2), Ok(()));
        assert_eq!(
            point.validate_cardinality(4, 2),
            Err(FieldError::CardinalityMismatch {
                association: FieldAssociation::Point,
                expected: 4,
                actual: 3,
            })
        );

        let cell = ScalarField::new(
            metadata("pressure", None),
            FieldAssociation::Cell,
            vec![SampleValue::Finite(1.0); 2],
        )
        .unwrap();
        assert_eq!(cell.validate_cardinality(3, 2), Ok(()));
    }

    #[test]
    fn dataset_scalar_field_requires_exactly_one_value() {
        assert_eq!(
            ScalarField::new(
                metadata("ambient_temperature", None),
                FieldAssociation::Dataset,
                vec![],
            ),
            Err(FieldError::DatasetFieldRequiresOneValue { actual: 0 })
        );
        let field = ScalarField::new(
            metadata("ambient_temperature", None),
            FieldAssociation::Dataset,
            vec![SampleValue::Finite(293.15)],
        )
        .unwrap();
        assert_eq!(field.validate_cardinality(100, 160), Ok(()));
    }

    #[test]
    fn vector_value_classifies_components_independently() {
        let value = VectorValue3::from_f64(1.0, f64::NAN, 3.0);
        assert_eq!(
            value.components(),
            [
                SampleValue::Finite(1.0),
                SampleValue::NonFinite(NonFiniteValue::NaN),
                SampleValue::Finite(3.0),
            ]
        );
        assert_eq!(value.as_finite_world_vector(), None);
    }

    #[test]
    fn finite_vector_value_converts_to_world_vector() {
        let value = VectorValue3::from_f64(1.0, 2.0, 3.0);
        assert_eq!(
            value.as_finite_world_vector(),
            Some(WorldVector3::new(1.0, 2.0, 3.0))
        );
    }

    #[test]
    fn vector_field_preserves_basis_association_and_cardinality() {
        let field = VectorField3::new(
            metadata("velocity", Some("m/s")),
            FieldAssociation::Point,
            VectorBasis3::World,
            vec![VectorValue3::from_f64(1.0, 0.0, 0.0); 3],
        )
        .unwrap();
        assert_eq!(field.basis(), VectorBasis3::World);
        assert_eq!(field.association(), FieldAssociation::Point);
        assert_eq!(field.validate_cardinality(3, 1), Ok(()));
        assert_eq!(
            field.validate_cardinality(2, 1),
            Err(FieldError::CardinalityMismatch {
                association: FieldAssociation::Point,
                expected: 2,
                actual: 3,
            })
        );
    }

    #[test]
    fn dataset_vector_field_requires_one_value() {
        assert_eq!(
            VectorField3::new(
                metadata("bulk_velocity", None),
                FieldAssociation::Dataset,
                VectorBasis3::World,
                vec![VectorValue3::from_f64(1.0, 2.0, 3.0); 2],
            ),
            Err(FieldError::DatasetFieldRequiresOneValue { actual: 2 })
        );
    }

    #[test]
    fn empty_point_and_cell_fields_are_representable_before_attachment() {
        let point = ScalarField::new(
            metadata("temperature", None),
            FieldAssociation::Point,
            Vec::new(),
        )
        .unwrap();
        assert!(point.is_empty());
        assert_eq!(
            point.validate_cardinality(0, 0),
            Ok(())
        );
    }
}
