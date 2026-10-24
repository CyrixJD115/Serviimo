//! Read-only Aniimo snapshot inspection.
//!
//! This crate has no dependency on Serviimo's server binary or store. It
//! never contacts a production endpoint and never writes the supplied file.

use std::fs::File;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use rmpv::Value;
use serde::Serialize;
use sha2::{Digest, Sha256};

pub const MAX_SOURCE_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_DECODED_BYTES: u64 = 64 * 1024 * 1024;
const MAX_NODES: usize = 1_000_000;
const MAX_DEPTH: usize = 128;
const MAX_REPORTED_CHANGES: usize = 1_000;
const ZSTD_MAGIC: [u8; 4] = [0x28, 0xb5, 0x2f, 0xfd];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SnapshotReport {
    pub source: PathBuf,
    pub sha256: String,
    pub source_bytes: usize,
    pub compression: &'static str,
    pub decoded_bytes: usize,
    pub message_pack_valid: bool,
    pub root_type: Option<&'static str>,
    pub node_count: Option<usize>,
    pub maximum_depth: Option<usize>,
    pub schema_confidence: &'static str,
    pub homeland_semantics: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SnapshotDiffReport {
    pub before: SnapshotReport,
    pub after: SnapshotReport,
    pub changes: Vec<SnapshotChange>,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SnapshotChange {
    pub path: String,
    pub kind: &'static str,
}

#[derive(Debug)]
pub enum SnapshotError {
    Io(std::io::Error),
    SourceTooLarge(u64),
    DecodedTooLarge,
    Zstandard(String),
    MessagePack(String),
    TrailingBytes(usize),
    StructureLimit(&'static str),
}

impl std::fmt::Display for SnapshotError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "snapshot I/O failed: {error}"),
            Self::SourceTooLarge(bytes) => {
                write!(formatter, "snapshot exceeds source limit: {bytes} bytes")
            }
            Self::DecodedTooLarge => write!(formatter, "snapshot exceeds decoded limit"),
            Self::Zstandard(error) => write!(formatter, "Zstandard decode failed: {error}"),
            Self::MessagePack(error) => write!(formatter, "MessagePack decode failed: {error}"),
            Self::TrailingBytes(bytes) => {
                write!(formatter, "MessagePack has {bytes} trailing bytes")
            }
            Self::StructureLimit(limit) => write!(formatter, "MessagePack exceeds {limit} limit"),
        }
    }
}

impl std::error::Error for SnapshotError {}

impl From<std::io::Error> for SnapshotError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

pub fn inspect(path: &Path) -> Result<SnapshotReport, SnapshotError> {
    Ok(load(path)?.report)
}

/// Compares two local snapshots without exposing their values.
///
/// The result only contains structural paths and whether each path was added,
/// removed, or changed. Both inputs remain untouched.
pub fn compare(before: &Path, after: &Path) -> Result<SnapshotDiffReport, SnapshotError> {
    let before = load(before)?;
    let after = load(after)?;
    let mut changes = Vec::new();
    let mut truncated = false;
    compare_values(
        &before.value,
        &after.value,
        "$",
        &mut changes,
        &mut truncated,
    );
    Ok(SnapshotDiffReport {
        before: before.report,
        after: after.report,
        changes,
        truncated,
    })
}

struct LoadedSnapshot {
    report: SnapshotReport,
    value: Value,
}

fn load(path: &Path) -> Result<LoadedSnapshot, SnapshotError> {
    let metadata = path.metadata()?;
    if metadata.len() > MAX_SOURCE_BYTES {
        return Err(SnapshotError::SourceTooLarge(metadata.len()));
    }
    let mut source = Vec::with_capacity(metadata.len() as usize);
    File::open(path)?.read_to_end(&mut source)?;
    let sha256 = format!("{:x}", Sha256::digest(&source));
    let compressed = source.starts_with(&ZSTD_MAGIC);
    let decoded = if compressed {
        let decoder = ruzstd::StreamingDecoder::new(source.as_slice())
            .map_err(|error| SnapshotError::Zstandard(error.to_string()))?;
        let mut limited = decoder.take(MAX_DECODED_BYTES + 1);
        let mut output = Vec::new();
        limited
            .read_to_end(&mut output)
            .map_err(|error| SnapshotError::Zstandard(error.to_string()))?;
        if output.len() as u64 > MAX_DECODED_BYTES {
            return Err(SnapshotError::DecodedTooLarge);
        }
        output
    } else {
        source.clone()
    };

    let mut cursor = Cursor::new(decoded.as_slice());
    let value = rmpv::decode::read_value(&mut cursor)
        .map_err(|error| SnapshotError::MessagePack(error.to_string()))?;
    let consumed = cursor.position() as usize;
    if consumed != decoded.len() {
        return Err(SnapshotError::TrailingBytes(decoded.len() - consumed));
    }
    let mut nodes = 0usize;
    let mut maximum_depth = 0usize;
    inspect_value(&value, 0, &mut nodes, &mut maximum_depth)?;

    Ok(LoadedSnapshot {
        report: SnapshotReport {
            source: path.to_path_buf(),
            sha256,
            source_bytes: source.len(),
            compression: if compressed { "zstandard" } else { "none" },
            decoded_bytes: decoded.len(),
            message_pack_valid: true,
            root_type: Some(value_kind(&value)),
            node_count: Some(nodes),
            maximum_depth: Some(maximum_depth),
            schema_confidence: "structural-only",
            homeland_semantics: "not-yet-mapped",
        },
        value,
    })
}

fn compare_values(
    before: &Value,
    after: &Value,
    path: &str,
    changes: &mut Vec<SnapshotChange>,
    truncated: &mut bool,
) {
    if *truncated || before == after {
        return;
    }
    match (before, after) {
        (Value::Map(before_entries), Value::Map(after_entries)) => {
            for (key, before_value) in before_entries {
                let child_path = map_path(path, key);
                if let Some((_, after_value)) =
                    after_entries.iter().find(|(candidate, _)| candidate == key)
                {
                    compare_values(before_value, after_value, &child_path, changes, truncated);
                } else {
                    push_change(changes, truncated, child_path, "removed");
                }
            }
            for (key, _) in after_entries {
                if !before_entries.iter().any(|(candidate, _)| candidate == key) {
                    push_change(changes, truncated, map_path(path, key), "added");
                }
            }
        }
        (Value::Array(before_values), Value::Array(after_values)) => {
            let shared = before_values.len().min(after_values.len());
            for index in 0..shared {
                compare_values(
                    &before_values[index],
                    &after_values[index],
                    &format!("{path}[{index}]"),
                    changes,
                    truncated,
                );
            }
            for index in shared..before_values.len() {
                push_change(changes, truncated, format!("{path}[{index}]"), "removed");
            }
            for index in shared..after_values.len() {
                push_change(changes, truncated, format!("{path}[{index}]"), "added");
            }
        }
        _ => push_change(changes, truncated, path.to_owned(), "changed"),
    }
}

fn push_change(
    changes: &mut Vec<SnapshotChange>,
    truncated: &mut bool,
    path: String,
    kind: &'static str,
) {
    if changes.len() == MAX_REPORTED_CHANGES {
        *truncated = true;
        return;
    }
    changes.push(SnapshotChange { path, kind });
}

fn map_path(parent: &str, key: &Value) -> String {
    match key {
        Value::String(value) => match value.as_str() {
            Some(value) if is_identifier(value) => format!("{parent}.{value}"),
            Some(value) => format!("{parent}[<string-key:{}>]", short_hash(value.as_bytes())),
            None => format!("{parent}[<invalid-utf8-key>]"),
        },
        Value::Integer(value) => format!("{parent}[{value}]"),
        Value::Boolean(value) => format!("{parent}[{value}]"),
        _ => format!("{parent}[<{}-key>]", value_kind(key)),
    }
}

fn short_hash(bytes: &[u8]) -> String {
    let digest = format!("{:x}", Sha256::digest(bytes));
    digest[..12].to_owned()
}

fn is_identifier(value: &str) -> bool {
    if value.len() > 64 {
        return false;
    }
    let mut characters = value.chars();
    matches!(characters.next(), Some(first) if first == '_' || first.is_ascii_lowercase())
        && characters.all(|character| {
            character == '_' || character.is_ascii_lowercase() || character.is_ascii_digit()
        })
}

fn inspect_value(
    value: &Value,
    depth: usize,
    nodes: &mut usize,
    maximum_depth: &mut usize,
) -> Result<(), SnapshotError> {
    if depth > MAX_DEPTH {
        return Err(SnapshotError::StructureLimit("nesting depth"));
    }
    *nodes = nodes
        .checked_add(1)
        .ok_or(SnapshotError::StructureLimit("node count"))?;
    if *nodes > MAX_NODES {
        return Err(SnapshotError::StructureLimit("node count"));
    }
    *maximum_depth = (*maximum_depth).max(depth);
    match value {
        Value::Array(values) => {
            for child in values {
                inspect_value(child, depth + 1, nodes, maximum_depth)?;
            }
        }
        Value::Map(values) => {
            for (key, child) in values {
                inspect_value(key, depth + 1, nodes, maximum_depth)?;
                inspect_value(child, depth + 1, nodes, maximum_depth)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn value_kind(value: &Value) -> &'static str {
    match value {
        Value::Nil => "null",
        Value::Boolean(_) => "boolean",
        Value::Integer(_) => "integer",
        Value::F32(_) | Value::F64(_) => "number",
        Value::String(_) => "string",
        Value::Binary(_) => "binary",
        Value::Array(_) => "array",
        Value::Map(_) => "map",
        Value::Ext(_, _) => "extension",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inspects_binary_preserving_message_pack_without_writing() {
        let path = std::env::temp_dir().join(format!(
            "serviimo-snapshot-{}-{}.bin",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let bytes = [
            0x82, 0xa7, b'v', b'e', b'r', b's', b'i', b'o', b'n', 0xcd, 0x05, 0xe1, 0xa4, b'b',
            b'l', b'o', b'b', 0xc4, 0x02, 0x00, 0xff,
        ];
        std::fs::write(&path, bytes).unwrap();
        let before = std::fs::read(&path).unwrap();
        let report = inspect(&path).unwrap();
        let after = std::fs::read(&path).unwrap();
        std::fs::remove_file(&path).unwrap();

        assert_eq!(before, after);
        assert_eq!(report.compression, "none");
        assert_eq!(report.root_type, Some("map"));
        assert_eq!(report.node_count, Some(5));
        assert_eq!(report.schema_confidence, "structural-only");
    }

    #[test]
    fn rejects_trailing_message_pack_data() {
        let path = std::env::temp_dir().join(format!(
            "serviimo-snapshot-trailing-{}.bin",
            std::process::id()
        ));
        std::fs::write(&path, [0x80, 0x00]).unwrap();
        let result = inspect(&path);
        std::fs::remove_file(&path).unwrap();
        assert!(matches!(result, Err(SnapshotError::TrailingBytes(1))));
    }

    #[test]
    fn compares_paths_without_exposing_values() {
        let stem = format!("serviimo-snapshot-compare-{}", std::process::id());
        let before_path = std::env::temp_dir().join(format!("{stem}-before.bin"));
        let after_path = std::env::temp_dir().join(format!("{stem}-after.bin"));
        let before = Value::Map(vec![(
            Value::from("records"),
            Value::Map(vec![(
                Value::from("home_camp"),
                Value::Map(vec![(Value::from("level"), Value::from(918_273))]),
            )]),
        )]);
        let after = Value::Map(vec![(
            Value::from("records"),
            Value::Map(vec![(
                Value::from("home_camp"),
                Value::Map(vec![(Value::from("level"), Value::from(192_837))]),
            )]),
        )]);
        let mut before_bytes = Vec::new();
        let mut after_bytes = Vec::new();
        rmpv::encode::write_value(&mut before_bytes, &before).unwrap();
        rmpv::encode::write_value(&mut after_bytes, &after).unwrap();
        std::fs::write(&before_path, before_bytes).unwrap();
        std::fs::write(&after_path, after_bytes).unwrap();

        let report = compare(&before_path, &after_path).unwrap();
        std::fs::remove_file(&before_path).unwrap();
        std::fs::remove_file(&after_path).unwrap();

        assert_eq!(
            report.changes,
            vec![SnapshotChange {
                path: "$.records.home_camp.level".to_owned(),
                kind: "changed",
            }]
        );
        let json = serde_json::to_string(&report).unwrap();
        assert!(!json.contains("918273"));
        assert!(!json.contains("192837"));
    }

    #[test]
    fn hashes_opaque_string_keys_in_paths() {
        let opaque = Value::from("account-like-key-with-dashes");
        let path = map_path("$.records", &opaque);
        assert!(path.starts_with("$.records[<string-key:"));
        assert!(!path.contains("account-like-key"));
    }
}
