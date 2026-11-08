//! Bounded parser for VTK legacy ASCII unstructured-grid datasets.
//!
//! This module intentionally implements only the subset documented in
//! `docs/vtk-legacy-ascii-import.md`. It performs no file I/O.

use std::{error::Error, fmt};

use crate::{
    dataset::{Dataset, DatasetError},
    fields::{
        FieldAssociation, FieldMetadata, ScalarField, VectorBasis3, VectorField3, VectorValue3,
    },
    sampling::SampleValue,
    spatial::WorldPoint3,
    unstructured::{CellType3, UnstructuredGrid3, UnstructuredGridError},
};

/// A categorized failure while parsing a VTK legacy ASCII unstructured grid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VtkLegacyErrorKind {
    InvalidHeader,
    UnsupportedVersion,
    UnsupportedFormat,
    UnsupportedDataset,
    MissingRecord,
    UnexpectedRecord,
    InvalidCount,
    CountOverflow,
    TruncatedData,
    InvalidNumber,
    UnsupportedNumericType,
    UnsupportedCellType,
    UnsupportedAttribute,
    InvalidField,
    InvalidGeometry,
}

/// A parser error with a useful source line and diagnostic message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VtkLegacyError {
    kind: VtkLegacyErrorKind,
    line: Option<usize>,
    message: String,
}

impl VtkLegacyError {
    fn new(kind: VtkLegacyErrorKind, line: Option<usize>, message: impl Into<String>) -> Self {
        Self { kind, line, message: message.into() }
    }

    /// Returns the categorized failure.
    #[must_use]
    pub const fn kind(&self) -> &VtkLegacyErrorKind { &self.kind }

    /// Returns the one-based source line, when available.
    #[must_use]
    pub const fn line(&self) -> Option<usize> { self.line }
}

impl fmt::Display for VtkLegacyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(line) = self.line {
            write!(f, "VTK legacy line {}: {}", line, self.message)
        } else {
            write!(f, "VTK legacy: {}", self.message)
        }
    }
}
impl Error for VtkLegacyError {}

/// Parses a VTK legacy ASCII `UNSTRUCTURED_GRID` into an authoritative SVF dataset.
///
/// Only the supported subset in `docs/vtk-legacy-ascii-import.md` is accepted.
/// Unsupported records fail rather than being silently ignored.
pub fn parse_legacy_ascii_unstructured_grid(input: &str) -> Result<Dataset, VtkLegacyError> {
    Parser::new(input).parse()
}

struct Parser<'a> {
    lines: Vec<&'a str>,
    index: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self { lines: input.lines().collect(), index: 0 }
    }

    fn parse(mut self) -> Result<Dataset, VtkLegacyError> {
        // The legacy envelope is positional: the version must be the first
        // physical line, and the title must occupy the second (possibly blank).
        let version = self.required_physical_line("version header")?;
        let version = version.trim();
        let Some(version) = version.strip_prefix("# vtk DataFile Version ") else {
            return Err(self.error(VtkLegacyErrorKind::InvalidHeader, "expected '# vtk DataFile Version x.x'"));
        };
        if !matches!(version.trim(), "1.0" | "2.0" | "3.0") {
            return Err(self.error(VtkLegacyErrorKind::UnsupportedVersion, format!("unsupported VTK legacy version '{}'", version.trim())));
        }

        let title = self.required_physical_line("title line")?;
        if title.chars().count() > 256 {
            return Err(self.error(VtkLegacyErrorKind::InvalidHeader, "title line exceeds 256 characters"));
        }
        let mode = self.required_physical_line("ASCII declaration")?;
        if !mode.trim().eq_ignore_ascii_case("ASCII") {
            return Err(self.error(VtkLegacyErrorKind::UnsupportedFormat, "only ASCII legacy files are supported"));
        }
        let dataset = self.required_physical_line("DATASET record")?;
        let dataset_tokens: Vec<_> = dataset.split_whitespace().collect();
        if dataset_tokens.len() != 2 || !dataset_tokens[0].eq_ignore_ascii_case("DATASET") {
            return Err(self.error(VtkLegacyErrorKind::InvalidHeader, "expected 'DATASET UNSTRUCTURED_GRID'"));
        }
        if !dataset_tokens[1].eq_ignore_ascii_case("UNSTRUCTURED_GRID") {
            return Err(self.error(VtkLegacyErrorKind::UnsupportedDataset, format!("unsupported dataset '{}'", dataset_tokens[1])));
        }

        let points_header = self.required_record("POINTS")?;
        let (point_count, coordinate_type) = parse_header_count_type(&points_header, "POINTS", self.line_number())?;
        let coordinate_value_count = point_count.checked_mul(3).ok_or_else(|| self.error(VtkLegacyErrorKind::CountOverflow, "POINTS coordinate count overflows usize"))?;
        let coordinates = self.read_numbers(coordinate_value_count, &coordinate_type, true)?;
        let mut points = Vec::new();
        for xyz in coordinates.chunks_exact(3) {
            points.push(WorldPoint3::new(xyz[0], xyz[1], xyz[2]));
        }

        let cells_header = self.required_record("CELLS")?;
        let cells_tokens: Vec<_> = cells_header.split_whitespace().collect();
        if cells_tokens.len() != 3 || !cells_tokens[0].eq_ignore_ascii_case("CELLS") {
            return Err(self.error(VtkLegacyErrorKind::InvalidHeader, "expected 'CELLS n size'"));
        }
        let cell_count = parse_usize(cells_tokens[1], self.line_number(), "CELLS count")?;
        let declared_size = parse_usize(cells_tokens[2], self.line_number(), "CELLS size")?;
        let raw_cells = self.read_integer_tokens(declared_size)?;
        let mut cursor = 0usize;
        let mut cell_records: Vec<Vec<u32>> = Vec::new();
        let mut actual_size = 0usize;
        for cell_index in 0..cell_count {
            let node_count_token = raw_cells.get(cursor).ok_or_else(|| self.error(VtkLegacyErrorKind::TruncatedData, format!("missing node count for cell {cell_index}")))?;
            cursor += 1;
            let node_count = usize::try_from(*node_count_token).map_err(|_| self.error(VtkLegacyErrorKind::InvalidCount, format!("invalid node count for cell {cell_index}")))?;
            actual_size = actual_size.checked_add(1).and_then(|n| n.checked_add(node_count)).ok_or_else(|| self.error(VtkLegacyErrorKind::CountOverflow, "CELLS size overflows usize"))?;
            let end = cursor.checked_add(node_count).ok_or_else(|| self.error(VtkLegacyErrorKind::CountOverflow, "cell connectivity range overflows usize"))?;
            let ids = raw_cells.get(cursor..end).ok_or_else(|| self.error(VtkLegacyErrorKind::TruncatedData, format!("cell {cell_index} declares {node_count} point IDs but the CELLS payload is truncated")))?;
            let mut cell = Vec::new();
            for id in ids {
                cell.push(u32::try_from(*id).map_err(|_| self.error(VtkLegacyErrorKind::InvalidCount, format!("point ID {id} in cell {cell_index} is outside u32")))?);
            }
            cell_records.push(cell);
            cursor = end;
        }
        if cursor != declared_size || actual_size != declared_size {
            return Err(self.error(VtkLegacyErrorKind::InvalidCount, format!("CELLS size declares {declared_size} integers but cell records consume {actual_size}")));
        }

        let types_header = self.required_record("CELL_TYPES")?;
        let type_tokens: Vec<_> = types_header.split_whitespace().collect();
        if type_tokens.len() != 2 || !type_tokens[0].eq_ignore_ascii_case("CELL_TYPES") {
            return Err(self.error(VtkLegacyErrorKind::InvalidHeader, "expected 'CELL_TYPES n'"));
        }
        let type_count = parse_usize(type_tokens[1], self.line_number(), "CELL_TYPES count")?;
        if type_count != cell_count {
            return Err(self.error(VtkLegacyErrorKind::InvalidCount, format!("CELL_TYPES count {type_count} does not match CELLS count {cell_count}")));
        }
        let raw_types = self.read_integer_tokens(type_count)?;
        let mut cell_types = Vec::new();
        let mut connectivity = Vec::new();
        cell_count.checked_add(1).ok_or_else(|| self.error(VtkLegacyErrorKind::CountOverflow, "cell offset count overflows usize"))?;
        let mut offsets = Vec::new();
        offsets.push(0);
        for (cell_index, (raw_type, ids)) in raw_types.into_iter().zip(cell_records).enumerate() {
            let cell_type = match raw_type {
                3 => CellType3::Line2,
                5 => CellType3::Triangle3,
                9 => CellType3::Quad4,
                10 => CellType3::Tetrahedron4,
                12 => CellType3::Hexahedron8,
                other => return Err(self.error(VtkLegacyErrorKind::UnsupportedCellType, format!("unsupported VTK cell type {other} at cell {cell_index}"))),
            };
            if ids.len() != cell_type.node_count() {
                return Err(self.error(VtkLegacyErrorKind::InvalidCount, format!("cell {cell_index} type {cell_type:?} requires {} nodes, received {}", cell_type.node_count(), ids.len())));
            }
            cell_types.push(cell_type);
            connectivity.extend(ids);
            offsets.push(connectivity.len());
        }
        let grid = UnstructuredGrid3::new(points, cell_types, offsets, connectivity)
            .map_err(|error| self.geometry_error(error))?;
        let mut dataset = Dataset::new(grid);

        let mut point_section_seen = false;
        let mut cell_section_seen = false;
        while let Some((line_number, record)) = self.next_nonempty_line() {
            let tokens: Vec<_> = record.split_whitespace().collect();
            let keyword = tokens[0].to_ascii_uppercase();
            match keyword.as_str() {
                "POINT_DATA" | "CELL_DATA" => {
                    if tokens.len() != 2 {
                        return Err(VtkLegacyError::new(VtkLegacyErrorKind::InvalidHeader, Some(line_number), format!("expected '{keyword} n'")));
                    }
                    let count = parse_usize(tokens[1], line_number, "attribute tuple count")?;
                    let association = if keyword == "POINT_DATA" {
                        if point_section_seen { return Err(VtkLegacyError::new(VtkLegacyErrorKind::UnexpectedRecord, Some(line_number), "duplicate POINT_DATA section")); }
                        point_section_seen = true;
                        if count != dataset.point_count() { return Err(VtkLegacyError::new(VtkLegacyErrorKind::InvalidCount, Some(line_number), format!("POINT_DATA count {count} does not match point count {}", dataset.point_count()))); }
                        FieldAssociation::Point
                    } else {
                        if cell_section_seen { return Err(VtkLegacyError::new(VtkLegacyErrorKind::UnexpectedRecord, Some(line_number), "duplicate CELL_DATA section")); }
                        cell_section_seen = true;
                        if count != dataset.cell_count() { return Err(VtkLegacyError::new(VtkLegacyErrorKind::InvalidCount, Some(line_number), format!("CELL_DATA count {count} does not match cell count {}", dataset.cell_count()))); }
                        FieldAssociation::Cell
                    };
                    self.parse_attribute_section(&mut dataset, association, count)?;
                }
                other => return Err(VtkLegacyError::new(VtkLegacyErrorKind::UnexpectedRecord, Some(line_number), format!("unexpected record '{other}' after geometry"))),
            }
        }
        Ok(dataset)
    }

    fn parse_attribute_section(&mut self, dataset: &mut Dataset, association: FieldAssociation, tuple_count: usize) -> Result<(), VtkLegacyError> {
        loop {
            let Some((line_number, record)) = self.peek_nonempty_line() else { return Ok(()); };
            let tokens: Vec<_> = record.split_whitespace().collect();
            let keyword = tokens[0].to_ascii_uppercase();
            if keyword == "POINT_DATA" || keyword == "CELL_DATA" { return Ok(()); }
            self.index += 1;
            match keyword.as_str() {
                "SCALARS" => {
                    if !(tokens.len() == 3 || tokens.len() == 4) {
                        return Err(VtkLegacyError::new(VtkLegacyErrorKind::InvalidHeader, Some(line_number), "expected 'SCALARS name dataType [numComponents]'"));
                    }
                    let name = tokens[1];
                    let data_type = tokens[2];
                    let components = if tokens.len() == 4 { parse_usize(tokens[3], line_number, "SCALARS component count")? } else { 1 };
                    if components != 1 { return Err(VtkLegacyError::new(VtkLegacyErrorKind::UnsupportedAttribute, Some(line_number), "only one-component SCALARS are supported")); }
                    let lookup = self.peek_nonempty_line();
                    if let Some((_, line)) = lookup {
                        let parts: Vec<_> = line.split_whitespace().collect();
                        if parts.first().is_some_and(|part| part.eq_ignore_ascii_case("LOOKUP_TABLE")) {
                            if parts.len() != 2 { return Err(VtkLegacyError::new(VtkLegacyErrorKind::InvalidHeader, Some(lookup.unwrap().0), "expected 'LOOKUP_TABLE tableName'")); }
                            self.index += 1;
                        }
                    }
                    let values = self.read_numbers(tuple_count, data_type, false)?;
                    let values = values.into_iter().map(SampleValue::from_f64).collect();
                    let field = ScalarField::new(FieldMetadata::new(name, None).map_err(|e| self.field_error(line_number, e))?, association, values)
                        .map_err(|e| self.field_error(line_number, e))?;
                    dataset.add_scalar_field(field).map_err(|e| self.dataset_field_error(line_number, e))?;
                }
                "VECTORS" => {
                    if tokens.len() != 3 { return Err(VtkLegacyError::new(VtkLegacyErrorKind::InvalidHeader, Some(line_number), "expected 'VECTORS name dataType'")); }
                    let values_count = tuple_count.checked_mul(3).ok_or_else(|| VtkLegacyError::new(VtkLegacyErrorKind::CountOverflow, Some(line_number), "VECTORS component count overflows usize"))?;
                    let values = self.read_numbers(values_count, tokens[2], false)?;
                    let vectors = values.chunks_exact(3).map(|v| VectorValue3::from_f64(v[0], v[1], v[2])).collect();
                    let field = VectorField3::new(FieldMetadata::new(tokens[1], None).map_err(|e| self.field_error(line_number, e))?, association, VectorBasis3::World, vectors)
                        .map_err(|e| self.field_error(line_number, e))?;
                    dataset.add_vector_field3(field).map_err(|e| self.dataset_field_error(line_number, e))?;
                }
                "LOOKUP_TABLE" => {
                    if tokens.len() != 3 { return Err(VtkLegacyError::new(VtkLegacyErrorKind::InvalidHeader, Some(line_number), "expected 'LOOKUP_TABLE name size'")); }
                    let count = parse_usize(tokens[2], line_number, "lookup table size")?;
                    let values_count = count.checked_mul(4).ok_or_else(|| VtkLegacyError::new(VtkLegacyErrorKind::CountOverflow, Some(line_number), "lookup table value count overflows usize"))?;
                    let _ = self.read_numbers(values_count, "float", false)?;
                }
                unsupported => return Err(VtkLegacyError::new(VtkLegacyErrorKind::UnsupportedAttribute, Some(line_number), format!("unsupported attribute record '{unsupported}'"))),
            }
        }
    }

    fn read_numbers(&mut self, count: usize, data_type: &str, coordinates: bool) -> Result<Vec<f64>, VtkLegacyError> {
        let mut values = Vec::new();
        while values.len() < count {
            let Some((line_number, line)) = self.next_nonempty_line() else {
                return Err(self.error(VtkLegacyErrorKind::TruncatedData, format!("expected {count} numeric values, received {}", values.len())));
            };
            let tokens: Vec<_> = line.split_whitespace().collect();
            if tokens.first().is_some_and(|token| is_record_keyword(token)) {
                return Err(VtkLegacyError::new(VtkLegacyErrorKind::TruncatedData, Some(line_number), format!("numeric payload ended after {} of {count} values", values.len())));
            }
            for token in tokens {
                if values.len() == count { return Err(VtkLegacyError::new(VtkLegacyErrorKind::InvalidCount, Some(line_number), "numeric payload contains more values than declared")); }
                let value = parse_typed_number(token, data_type, line_number)?;
                if coordinates && !value.is_finite() { return Err(VtkLegacyError::new(VtkLegacyErrorKind::InvalidGeometry, Some(line_number), "point coordinates must be finite")); }
                values.push(value);
            }
        }
        Ok(values)
    }

    fn read_integer_tokens(&mut self, count: usize) -> Result<Vec<u64>, VtkLegacyError> {
        let mut values = Vec::new();
        while values.len() < count {
            let Some((line_number, line)) = self.next_nonempty_line() else {
                return Err(self.error(VtkLegacyErrorKind::TruncatedData, format!("expected {count} integer values, received {}", values.len())));
            };
            let tokens: Vec<_> = line.split_whitespace().collect();
            if tokens.first().is_some_and(|token| is_record_keyword(token)) {
                return Err(VtkLegacyError::new(VtkLegacyErrorKind::TruncatedData, Some(line_number), format!("integer payload ended after {} of {count} values", values.len())));
            }
            for token in tokens {
                if values.len() == count { return Err(VtkLegacyError::new(VtkLegacyErrorKind::InvalidCount, Some(line_number), "integer payload contains more values than declared")); }
                let value = token.parse::<u64>().map_err(|_| VtkLegacyError::new(VtkLegacyErrorKind::InvalidNumber, Some(line_number), format!("invalid non-negative integer '{token}'")))?;
                values.push(value);
            }
        }
        Ok(values)
    }

    fn required_record(&mut self, expected: &str) -> Result<String, VtkLegacyError> {
        let line = self.required_line(expected)?;
        if !line.split_whitespace().next().is_some_and(|word| word.eq_ignore_ascii_case(expected)) {
            return Err(self.error(VtkLegacyErrorKind::MissingRecord, format!("expected {expected} record")));
        }
        Ok(line.to_owned())
    }

    fn required_line(&mut self, expected: &str) -> Result<&'a str, VtkLegacyError> {
        let Some((_, line)) = self.next_nonempty_line() else {
            return Err(self.error(VtkLegacyErrorKind::MissingRecord, format!("missing {expected}")));
        };
        Ok(line)
    }

    fn required_physical_line(&mut self, expected: &str) -> Result<&'a str, VtkLegacyError> {
        let Some((_, line)) = self.next_physical_line() else {
            return Err(self.error(VtkLegacyErrorKind::MissingRecord, format!("missing {expected}")));
        };
        Ok(line)
    }

    fn next_physical_line(&mut self) -> Option<(usize, &'a str)> {
        if self.index >= self.lines.len() {
            return None;
        }
        let current = self.index;
        self.index += 1;
        Some((current + 1, self.lines[current].trim()))
    }

    fn next_nonempty_line(&mut self) -> Option<(usize, &'a str)> {
        while self.index < self.lines.len() {
            let current = self.index;
            self.index += 1;
            let line = self.lines[current].trim();
            if !line.is_empty() { return Some((current + 1, line)); }
        }
        None
    }

    fn peek_nonempty_line(&self) -> Option<(usize, &'a str)> {
        let mut index = self.index;
        while index < self.lines.len() {
            let line = self.lines[index].trim();
            if !line.is_empty() { return Some((index + 1, line)); }
            index += 1;
        }
        None
    }

    fn line_number(&self) -> usize { self.index.max(1) }

    fn error(&self, kind: VtkLegacyErrorKind, message: impl Into<String>) -> VtkLegacyError {
        VtkLegacyError::new(kind, Some(self.line_number()), message)
    }

    fn geometry_error(&self, error: UnstructuredGridError) -> VtkLegacyError {
        VtkLegacyError::new(VtkLegacyErrorKind::InvalidGeometry, None, error.to_string())
    }
    fn field_error(&self, line: usize, error: impl fmt::Display) -> VtkLegacyError {
        VtkLegacyError::new(VtkLegacyErrorKind::InvalidField, Some(line), error.to_string())
    }
    fn dataset_field_error(&self, line: usize, error: DatasetError) -> VtkLegacyError {
        VtkLegacyError::new(VtkLegacyErrorKind::InvalidField, Some(line), error.to_string())
    }
}

fn parse_header_count_type(line: &str, expected: &str, line_number: usize) -> Result<(usize, String), VtkLegacyError> {
    let tokens: Vec<_> = line.split_whitespace().collect();
    if tokens.len() != 3 || !tokens[0].eq_ignore_ascii_case(expected) {
        return Err(VtkLegacyError::new(VtkLegacyErrorKind::InvalidHeader, Some(line_number), format!("expected '{expected} n dataType'")));
    }
    Ok((parse_usize(tokens[1], line_number, "record count")?, tokens[2].to_ascii_lowercase()))
}

fn parse_usize(token: &str, line: usize, label: &str) -> Result<usize, VtkLegacyError> {
    token.parse::<usize>().map_err(|_| VtkLegacyError::new(VtkLegacyErrorKind::InvalidCount, Some(line), format!("invalid {label} '{token}'")))
}

fn parse_typed_number(token: &str, data_type: &str, line: usize) -> Result<f64, VtkLegacyError> {
    let invalid = || VtkLegacyError::new(VtkLegacyErrorKind::InvalidNumber, Some(line), format!("invalid {data_type} value '{token}'"));
    match data_type.to_ascii_lowercase().as_str() {
        "float" => token.parse::<f32>().map(|v| v as f64).map_err(|_| invalid()),
        "double" => token.parse::<f64>().map_err(|_| invalid()),
        "short" => token.parse::<i16>().map(|v| v as f64).map_err(|_| invalid()),
        "unsigned_short" => token.parse::<u16>().map(|v| v as f64).map_err(|_| invalid()),
        "int" => token.parse::<i32>().map(|v| v as f64).map_err(|_| invalid()),
        "unsigned_int" => token.parse::<u32>().map(|v| v as f64).map_err(|_| invalid()),
        "unsigned_char" => token.parse::<u8>().map(|v| v as f64).map_err(|_| invalid()),
        other => Err(VtkLegacyError::new(VtkLegacyErrorKind::UnsupportedNumericType, Some(line), format!("unsupported numeric type '{other}'"))),
    }
}

fn is_record_keyword(token: &str) -> bool {
    matches!(token.to_ascii_uppercase().as_str(),
        "POINTS" | "CELLS" | "CELL_TYPES" | "POINT_DATA" | "CELL_DATA" |
        "SCALARS" | "VECTORS" | "LOOKUP_TABLE" | "COLOR_SCALARS" | "NORMALS" |
        "TEXTURE_COORDINATES" | "TENSORS" | "FIELD" | "METADATA" | "DATASET")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{dataset::{DatasetField, DatasetGeometry}, unstructured::CellType3};

    const MIXED: &str = "# vtk DataFile Version 3.0
SVF fixture
ASCII
DATASET UNSTRUCTURED_GRID
POINTS 8 double
0 0 0  1 0 0  1 1 0  0 1 0
0 0 1  1 0 1  1 1 1  0 1 1
CELLS 2 12
2 0 1
8 0 1 2 3 4 5 6 7
CELL_TYPES 2
3 12
POINT_DATA 8
SCALARS temperature float 1
LOOKUP_TABLE default
0 1 2 3 4 5 6 7
VECTORS velocity double
1 0 0  0 1 0  0 0 1  1 1 0
0 0 1  1 0 1  0 1 1  1 1 1
CELL_DATA 2
SCALARS pressure int
LOOKUP_TABLE default
100 200
VECTORS flux float
1 2 3 4 5 6
";

    #[test]
    fn parses_mixed_grid_and_point_cell_fields() {
        let dataset = parse_legacy_ascii_unstructured_grid(MIXED).unwrap();
        assert_eq!((dataset.point_count(), dataset.cell_count()), (8, 2));
        let DatasetGeometry::UnstructuredGrid3(grid) = dataset.geometry() else { panic!("wrong geometry"); };
        assert_eq!(grid.cell_types(), &[CellType3::Line2, CellType3::Hexahedron8]);
        assert_eq!(grid.cell_offsets(), &[0, 2, 10]);
        assert_eq!(grid.connectivity(), &[0, 1, 0, 1, 2, 3, 4, 5, 6, 7]);
        assert!(matches!(dataset.fields()[0], DatasetField::Scalar(_)));
        assert_eq!(dataset.scalar_field("temperature").unwrap().association(), FieldAssociation::Point);
        assert_eq!(dataset.vector_field3("velocity").unwrap().basis(), VectorBasis3::World);
        assert_eq!(dataset.scalar_field("pressure").unwrap().association(), FieldAssociation::Cell);
        assert_eq!(dataset.vector_field3("flux").unwrap().values()[1].components(), [
            SampleValue::Finite(4.0), SampleValue::Finite(5.0), SampleValue::Finite(6.0)
        ]);
    }

    #[test]
    fn accepts_crlf_and_split_payloads() {
        let text = "# vtk DataFile Version 1.0\r\nfixture\r\nASCII\r\nDATASET UNSTRUCTURED_GRID\r\nPOINTS 3 float\r\n0\r\n0 0\r\n1 0 0\r\n0 1 0\r\nCELLS 1 4\r\n3 0\r\n1 2\r\nCELL_TYPES 1\r\n5\r\n";
        let dataset = parse_legacy_ascii_unstructured_grid(text).unwrap();
        assert_eq!((dataset.point_count(), dataset.cell_count()), (3, 1));
    }

    #[test]
    fn rejects_binary_and_unsupported_cell_types() {
        let binary = MIXED.replace("ASCII", "BINARY");
        assert_eq!(parse_legacy_ascii_unstructured_grid(&binary).unwrap_err().kind(), &VtkLegacyErrorKind::UnsupportedFormat);
        let voxel = MIXED.replace("3 12", "3 11");
        assert_eq!(parse_legacy_ascii_unstructured_grid(&voxel).unwrap_err().kind(), &VtkLegacyErrorKind::UnsupportedCellType);
    }

    #[test]
    fn rejects_non_finite_coordinates_and_duplicate_field_names() {
        let nonfinite = MIXED.replace("0 0 0  1", "NaN 0 0  1");
        assert_eq!(parse_legacy_ascii_unstructured_grid(&nonfinite).unwrap_err().kind(), &VtkLegacyErrorKind::InvalidGeometry);
        let duplicate = MIXED.replace("CELL_DATA 2", "CELL_DATA 2\nSCALARS temperature float\n1 2");
        assert_eq!(parse_legacy_ascii_unstructured_grid(&duplicate).unwrap_err().kind(), &VtkLegacyErrorKind::InvalidField);
    }

    #[test]
    fn preserves_non_finite_field_values_and_float32_semantics() {
        let text = MIXED.replace("SCALARS temperature float 1\nLOOKUP_TABLE default\n0 1 2 3 4 5 6 7\nVECTORS velocity", "SCALARS temperature float 1\nLOOKUP_TABLE default\nNaN Inf -Inf 0.1 4 5 6 7\nVECTORS velocity");
        let dataset = parse_legacy_ascii_unstructured_grid(&text).unwrap();
        let values = dataset.scalar_field("temperature").unwrap().values();
        assert_eq!(values[0], SampleValue::from_f64(f64::NAN));
        assert_eq!(values[1], SampleValue::from_f64(f64::INFINITY));
        assert_eq!(values[2], SampleValue::from_f64(f64::NEG_INFINITY));
        assert_eq!(values[3], SampleValue::from_f64(0.1_f32 as f64));
    }
    #[test]
    fn enforces_positional_file_envelope_and_accepts_blank_title() {
        let blank_title = MIXED.replacen("SVF fixture", "", 1);
        assert!(parse_legacy_ascii_unstructured_grid(&blank_title).is_ok());

        let leading_blank = format!("\n{MIXED}");
        assert_eq!(
            parse_legacy_ascii_unstructured_grid(&leading_blank).unwrap_err().kind(),
            &VtkLegacyErrorKind::InvalidHeader
        );

        let extra_blank_before_ascii = MIXED.replacen("SVF fixture\nASCII", "SVF fixture\n\nASCII", 1);
        assert_eq!(
            parse_legacy_ascii_unstructured_grid(&extra_blank_before_ascii).unwrap_err().kind(),
            &VtkLegacyErrorKind::UnsupportedFormat
        );
    }

    #[test]
    fn accepts_cell_data_before_point_data_and_optional_scalar_metadata() {
        let point_start = MIXED.find("POINT_DATA 8\n").unwrap();
        let cell_start = MIXED.find("CELL_DATA 2\n").unwrap();
        let point_section = &MIXED[point_start..cell_start];
        let cell_section = &MIXED[cell_start..];
        let geometry = &MIXED[..point_start];
        let reordered = format!(
            "{geometry}{}{}",
            cell_section.replace("LOOKUP_TABLE default\n", ""),
            point_section
                .replace("SCALARS temperature float 1", "SCALARS temperature float")
                .replace("LOOKUP_TABLE default\n", "")
        );
        let dataset = parse_legacy_ascii_unstructured_grid(&reordered).unwrap();
        assert_eq!(dataset.scalar_field("pressure").unwrap().association(), FieldAssociation::Cell);
        assert_eq!(dataset.scalar_field("temperature").unwrap().association(), FieldAssociation::Point);
    }

    #[test]
    fn converts_supported_integer_field_types_at_declared_widths() {
        let text = MIXED
            .replace("SCALARS temperature float 1", "SCALARS temperature unsigned_char 1")
            .replace("SCALARS pressure int", "SCALARS pressure short")
            .replace("VECTORS flux float", "VECTORS flux unsigned_short");
        let dataset = parse_legacy_ascii_unstructured_grid(&text).unwrap();
        assert_eq!(dataset.scalar_field("temperature").unwrap().values()[7], SampleValue::Finite(7.0));
        assert_eq!(dataset.scalar_field("pressure").unwrap().values()[1], SampleValue::Finite(200.0));
        assert_eq!(
            dataset.vector_field3("flux").unwrap().values()[1].components(),
            [SampleValue::Finite(4.0), SampleValue::Finite(5.0), SampleValue::Finite(6.0)]
        );
    }

    #[test]
    fn rejects_malformed_topology_sections_and_trailing_records() {
        let bad_cell_count = MIXED.replace("CELLS 2 12", "CELLS 3 12");
        assert_eq!(
            parse_legacy_ascii_unstructured_grid(&bad_cell_count).unwrap_err().kind(),
            &VtkLegacyErrorKind::TruncatedData
        );

        let bad_index = MIXED.replace("2 0 1\n", "2 0 99\n");
        assert_eq!(
            parse_legacy_ascii_unstructured_grid(&bad_index).unwrap_err().kind(),
            &VtkLegacyErrorKind::InvalidGeometry
        );

        let repeated_index = MIXED.replace("2 0 1\n", "2 0 0\n");
        assert_eq!(
            parse_legacy_ascii_unstructured_grid(&repeated_index).unwrap_err().kind(),
            &VtkLegacyErrorKind::InvalidGeometry
        );

        let duplicate_section = format!("{MIXED}POINT_DATA 8\n");
        assert_eq!(
            parse_legacy_ascii_unstructured_grid(&duplicate_section).unwrap_err().kind(),
            &VtkLegacyErrorKind::UnexpectedRecord
        );

        let trailing = format!("{MIXED}GARBAGE\n");
        assert_eq!(
            parse_legacy_ascii_unstructured_grid(&trailing).unwrap_err().kind(),
            &VtkLegacyErrorKind::UnexpectedRecord
        );
    }

    #[test]
    fn rejects_unsupported_attribute_and_numeric_records() {
        let unsupported_attribute = MIXED.replace(
            "SCALARS temperature float 1",
            "NORMALS surface_normal float"
        );
        assert_eq!(
            parse_legacy_ascii_unstructured_grid(&unsupported_attribute).unwrap_err().kind(),
            &VtkLegacyErrorKind::UnsupportedAttribute
        );

        let unsupported_type = MIXED.replace(
            "SCALARS temperature float 1",
            "SCALARS temperature long 1"
        );
        assert_eq!(
            parse_legacy_ascii_unstructured_grid(&unsupported_type).unwrap_err().kind(),
            &VtkLegacyErrorKind::UnsupportedNumericType
        );

        let multicomponent = MIXED.replace(
            "SCALARS temperature float 1",
            "SCALARS temperature float 3"
        );
        assert_eq!(
            parse_legacy_ascii_unstructured_grid(&multicomponent).unwrap_err().kind(),
            &VtkLegacyErrorKind::UnsupportedAttribute
        );
    }

}
