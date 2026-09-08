//! Storage for the coarse Document registry entity (`.verify/doc/<id>.yaml`)
//! `vtest doc add/list/show` manages — see
//! `vtest_model::doc_registry`'s module doc comment for what this entity is
//! and how it differs from the fine node-tree `DocumentFile`
//! (`.verify/doc/<name>.json`).

use std::{collections::BTreeMap, fs, path::Path};

use vtest_model::{ContentHash, DocRegistryRecord};

use crate::{StoreError, VerifyLayout};

const DOC_REGISTRY_KEYS: &[&str] = &[
    "id",
    "path",
    "title",
    "content_hash",
    "derives_from",
    "root",
    "registered_at",
];
const DOC_REGISTRY_DERIVES_FROM_KEYS: &[&str] = &["doc", "anchor", "note"];

fn to_yaml(record: &DocRegistryRecord) -> Result<String, StoreError> {
    validate(record, None)?;
    yaml_serde::to_string(record).map_err(|error| {
        StoreError::InvalidConfig(format!("could not serialize doc registry record: {error}"))
    })
}

fn from_yaml(text: &str, filename_id: &str) -> Result<DocRegistryRecord, StoreError> {
    let value: yaml_serde::Value = yaml_serde::from_str(text).map_err(|error| {
        StoreError::InvalidConfig(format!("invalid doc registry record: {error}"))
    })?;
    crate::canonical::reject_unknown_fields(&value, DOC_REGISTRY_KEYS, "")?;
    if let Some(entries) = value
        .get("derives_from")
        .and_then(yaml_serde::Value::as_sequence)
    {
        for (index, entry) in entries.iter().enumerate() {
            crate::canonical::reject_unknown_fields(
                entry,
                DOC_REGISTRY_DERIVES_FROM_KEYS,
                &format!("derives_from[{index}]."),
            )?;
        }
    }
    let record: DocRegistryRecord = yaml_serde::from_value(value).map_err(|error| {
        StoreError::InvalidConfig(format!("invalid doc registry record: {error}"))
    })?;
    validate(&record, Some(filename_id))?;
    Ok(record)
}

/// DS-1000〜1008: id/path/registered_at are required non-empty fields;
/// `--derives-from` entries name a non-empty target id (DS-1008's own
/// "`--derives-from` を伴わない `--anchor`" rejection is an argument-parsing
/// concern the CLI layer enforces before a [`DocRegistryDerivesFrom`] is
/// even constructed — every entry that reaches this record already has its
/// `anchor` bound to a present `doc`, by construction of the type itself).
fn validate(record: &DocRegistryRecord, filename_id: Option<&str>) -> Result<(), StoreError> {
    if record.id.trim().is_empty() {
        return Err(StoreError::InvalidConfig(
            "doc registry record is missing id".to_owned(),
        ));
    }
    if let Some(filename_id) = filename_id {
        if record.id != filename_id {
            return Err(StoreError::InvalidConfig(format!(
                "doc registry record id {} does not match file name {filename_id}",
                record.id
            )));
        }
    }
    if record.path.trim().is_empty() {
        return Err(StoreError::InvalidConfig(
            "doc registry record is missing path".to_owned(),
        ));
    }
    if record.registered_at.trim().is_empty() {
        return Err(StoreError::InvalidConfig(
            "doc registry record is missing registered_at".to_owned(),
        ));
    }
    for entry in &record.derives_from {
        if entry.doc.trim().is_empty() {
            return Err(StoreError::InvalidConfig(
                "doc registry record has a derives_from entry with an empty doc id".to_owned(),
            ));
        }
        if entry.doc == record.id {
            return Err(StoreError::InvalidConfig(
                "doc registry record derives_from must not self-reference".to_owned(),
            ));
        }
    }
    Ok(())
}

fn doc_registry_path(layout: &VerifyLayout, id: &str) -> std::path::PathBuf {
    layout.doc_dir().join(format!("{id}.yaml"))
}

pub fn read_doc_registry_record(
    layout: &VerifyLayout,
    id: &str,
) -> Result<DocRegistryRecord, StoreError> {
    let path = doc_registry_path(layout, id);
    let text = crate::records::read_text(&path)?;
    from_yaml(&text, id)
}

/// DS-1012/1013/1014: `--update` recomputes `content_hash` from the current
/// `--path` file and may combine with `--root`/`--no-root`; the caller
/// (`ops::doc`) decides whether this is a fresh `add` or an `--update`
/// re-registration — this function always writes/overwrites in place, per
/// BD-321's "mutable in place" convention already used by
/// `write_vo_record`.
pub fn write_doc_registry_record(
    layout: &VerifyLayout,
    record: &DocRegistryRecord,
) -> Result<(), StoreError> {
    let yaml = to_yaml(record)?;
    let path = doc_registry_path(layout, &record.id);
    crate::records::write_atomic(&path, &yaml)
}

/// Every `.verify/doc/<id>.yaml` registry record, keyed by id. The
/// `.verify/doc/<name>.json` node-tree files (a disjoint extension) are not
/// read here — see the module doc comment for why the two coexist.
pub fn read_all_doc_registry(
    layout: &VerifyLayout,
) -> Result<BTreeMap<String, DocRegistryRecord>, StoreError> {
    let dir = layout.doc_dir();
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(source) => return Err(StoreError::Io { path: dir, source }),
    };
    let mut records = BTreeMap::new();
    for entry in entries {
        let entry = entry.map_err(|source| StoreError::Io {
            path: dir.clone(),
            source,
        })?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("yaml") {
            continue;
        }
        let id = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_owned();
        let record = read_doc_registry_record(layout, &id)?;
        records.insert(id, record);
    }
    Ok(records)
}

/// DES-482: sha256 of the `--path` target file, as `doc add`/`--update`
/// bind it to the registry record (a plain content hash, not the §1.3
/// domain-separated subject hash — DES-482's own wording is "sha256 を計算
/// して…束縛した", not a structured multi-field subject).
pub fn hash_doc_registry_path(
    project_root: &Path,
    relative_path: &str,
) -> Result<ContentHash, StoreError> {
    let absolute = project_root.join(relative_path);
    let bytes = fs::read(&absolute).map_err(|source| StoreError::Io {
        path: absolute,
        source,
    })?;
    Ok(ContentHash::from_bytes(&bytes))
}

/// DS-1018: a `derives_from` target that does not resolve to a registered
/// document is a dangling registry-level link (distinct from the fine
/// node-tree's own E-SCAN-012 — see `vtest-verify`'s `evaluate_chain_
/// integrity` for that check; this is the registry-entity analogue this
/// module owns).
pub fn unresolved_derives_from(records: &BTreeMap<String, DocRegistryRecord>) -> Vec<(&str, &str)> {
    let mut unresolved = Vec::new();
    for record in records.values() {
        for entry in &record.derives_from {
            if !records.contains_key(&entry.doc) {
                unresolved.push((record.id.as_str(), entry.doc.as_str()));
            }
        }
    }
    unresolved
}

#[cfg(test)]
mod tests {
    use super::*;
    use vtest_model::DocRegistryDerivesFrom;

    fn temp_layout(name: &str) -> VerifyLayout {
        let root = std::env::temp_dir().join(format!(
            "vtest-store-doc-registry-{name}-{}",
            crate::records::new_record_id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        crate::init_project(&root, "fixture").unwrap();
        VerifyLayout::new(&root)
    }

    fn sample(id: &str) -> DocRegistryRecord {
        DocRegistryRecord {
            id: id.to_owned(),
            path: "docs/basic-spec.md".to_owned(),
            title: Some("基本仕様".to_owned()),
            content_hash: ContentHash::from_text("basic spec\n"),
            derives_from: vec![DocRegistryDerivesFrom {
                doc: "DOC-REQ-001".to_owned(),
                anchor: Some("§3.2".to_owned()),
                note: None,
            }],
            root: false,
            registered_at: "2026-09-09T00:00:00Z".to_owned(),
        }
    }

    #[test]
    fn round_trips_through_yaml() {
        let record = sample("DOC-BASIC-001");
        let yaml = to_yaml(&record).unwrap();
        let parsed = from_yaml(&yaml, "DOC-BASIC-001").unwrap();
        assert_eq!(parsed, record);
    }

    #[test]
    fn write_then_read_round_trips() {
        let layout = temp_layout("write-read");
        let record = sample("DOC-BASIC-001");
        write_doc_registry_record(&layout, &record).unwrap();
        let read = read_doc_registry_record(&layout, "DOC-BASIC-001").unwrap();
        assert_eq!(read, record);
    }

    #[test]
    fn unknown_top_level_field_is_rejected() {
        let record = sample("DOC-BASIC-001");
        let mut yaml = to_yaml(&record).unwrap();
        yaml.push_str("scope: read-only\n");
        let error = from_yaml(&yaml, "DOC-BASIC-001")
            .expect_err("an unrecognized top-level field must fail closed");
        assert!(matches!(error, StoreError::SchemaMismatch { .. }));
    }

    #[test]
    fn self_referencing_derives_from_is_rejected() {
        let mut record = sample("DOC-BASIC-001");
        record.derives_from[0].doc = "DOC-BASIC-001".to_owned();
        assert!(to_yaml(&record).is_err());
    }

    #[test]
    fn unresolved_derives_from_reports_the_dangling_link() {
        let mut records = BTreeMap::new();
        records.insert("DOC-BASIC-001".to_owned(), sample("DOC-BASIC-001"));
        let unresolved = unresolved_derives_from(&records);
        assert_eq!(unresolved, vec![("DOC-BASIC-001", "DOC-REQ-001")]);
    }
}
