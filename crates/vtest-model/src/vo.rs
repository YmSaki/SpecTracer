use crate::{DerivesFrom, VoId};
use serde::de::{MapAccess, Visitor};
use serde::ser::{Error as SerError, SerializeMap};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeSet;
use std::fmt;

/// Defines a verification dimension and its allowed partitions.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Dimension {
    pub name: String,
    pub partitions: Vec<String>,
}

/// One `combinations[]` entry (詳細設計 v0.1 §3.2.1): the declared
/// (dimension name, partition value) pairs, in declaration order, verbatim.
///
/// Deliberately not a `BTreeMap<String, String>` (this type's predecessor
/// here): a plain map cannot represent a YAML mapping that repeats one key
/// — either the entry deserializing into it silently collapses to the last
/// value for that key (a bare `serde` map `Deserialize`, e.g. `BTreeMap`'s,
/// has no duplicate-key check of its own), or, when read via
/// `yaml_serde::Value` first, the read fails outright before any `VoRecord`
/// is built (`yaml_serde::Mapping`'s own `Deserialize` explicitly rejects a
/// repeated key). 詳細設計 v0.1 本冊:283（§3.2.1 の受理条件、条件6）assigns
/// "entry が宣言済み dimension のいずれかを欠く、**または同じ dimension
/// 名を2回以上持つ**" to the *same* `E-SCAN-017` diagnostic
/// (`vtest-scan`'s `invalid_vo_combinations`, VO retained with
/// `chain_integrity = MISMATCH` — 本冊:1625・別紙A:438) — a scan-layer
/// judgment, not a record-layer parse failure. This type exists so the
/// record reader (`vtest-store::canonical::vo_record_from_yaml`) can hand a
/// malformed entry to that scan-layer check losslessly, instead of the
/// record layer pre-empting the judgment by rejecting or silently
/// collapsing the entry first.
///
/// `Deserialize` therefore keeps every declared pair, including a repeated
/// dimension name (see `deserialize` below) — this is the one place in
/// `VoRecord`'s tree that tolerates a duplicate key. Every other field
/// keeps rejecting one, and does not need this type's help to do so: a
/// duplicate top-level key, or a duplicate key inside one `derives_from[]`/
/// `dimensions[]` entry, is already rejected by `serde`'s own generated
/// struct `Deserialize` for `VoRecord`/`DerivesFrom`/`Dimension` — a
/// struct's `Deserialize` visitor tracks each declared field and errors
/// with `duplicate_field` the second time it sees the same one, entirely
/// independent of whichever format's `Deserializer` drives it (confirmed
/// empirically against `yaml_serde` directly, bypassing
/// `yaml_serde::Value`, for both a top-level and a `derives_from[]`/
/// `dimensions[]`-nested duplicate). `combinations`'s element type was the
/// only field in this tree that fell through that protection, because a
/// bare map type (unlike a `#[derive(Deserialize)]` struct) has no
/// equivalent check.
///
/// `Serialize` refuses (returns an error) to emit an entry that still
/// contains a duplicate dimension name — the writer must not be able to
/// persist a YAML mapping with a repeated key merely because the reader can
/// tolerate reading one (`vtest-store::canonical::write_vo_record` checks
/// this explicitly before ever reaching serialization, so that path returns
/// a clean `StoreError` rather than reaching this panic-on-`expect`
/// fallback).
#[derive(Clone, Debug, Default)]
pub struct CombinationEntry(Vec<(String, String)>);

impl CombinationEntry {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The partition value declared for `dimension`, if present. If
    /// `dimension` was declared more than once (see
    /// `duplicate_dimension_names`), this returns the *first* declared
    /// value — callers that need to detect the duplicate itself should call
    /// `duplicate_dimension_names` explicitly rather than infer it from
    /// `get`'s silence.
    pub fn get(&self, dimension: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(name, _)| name == dimension)
            .map(|(_, value)| value.as_str())
    }

    /// The declared (dimension name, partition value) pairs, in declaration
    /// order, including a repeated dimension name if the entry has one.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
    }

    /// Dimension names this entry declares more than once (詳細設計 v0.1
    /// 本冊:283, §3.2.1 condition 6's second half: "同じ dimension 名を2回
    /// 以上持つ"). Empty when every declared name is unique — including
    /// when the entry is empty. Each repeated name appears once, in the
    /// order its *second* occurrence was declared.
    pub fn duplicate_dimension_names(&self) -> Vec<&str> {
        let mut seen = BTreeSet::new();
        let mut duplicates = Vec::new();
        for (name, _) in &self.0 {
            if !seen.insert(name.as_str()) && !duplicates.contains(&name.as_str()) {
                duplicates.push(name.as_str());
            }
        }
        duplicates
    }

    /// A sorted, name-then-value view used for order-independent equality
    /// and ordering (below) — 詳細設計 v0.1 §3.2.1: "記述順・map key 順には
    /// 依存しない". A `BTreeMap`-backed entry got this for free from the
    /// map's own `Eq`/`Ord`; this type recovers the same property
    /// explicitly since it preserves declaration order instead.
    fn canonical_pairs(&self) -> Vec<(&str, &str)> {
        let mut pairs = self.iter().collect::<Vec<_>>();
        pairs.sort_unstable();
        pairs
    }
}

impl<'a> IntoIterator for &'a CombinationEntry {
    type Item = (&'a str, &'a str);
    type IntoIter = std::iter::Map<
        std::slice::Iter<'a, (String, String)>,
        fn(&'a (String, String)) -> (&'a str, &'a str),
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.0
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
    }
}

impl FromIterator<(String, String)> for CombinationEntry {
    fn from_iter<T: IntoIterator<Item = (String, String)>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl PartialEq for CombinationEntry {
    fn eq(&self, other: &Self) -> bool {
        self.canonical_pairs() == other.canonical_pairs()
    }
}

impl Eq for CombinationEntry {}

impl PartialOrd for CombinationEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CombinationEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.canonical_pairs().cmp(&other.canonical_pairs())
    }
}

impl Serialize for CombinationEntry {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let duplicates = self.duplicate_dimension_names();
        if !duplicates.is_empty() {
            return Err(S::Error::custom(format!(
                "combinations entry declares dimension `{}` more than once; \
                 refusing to serialize a YAML mapping with a duplicate key",
                duplicates.join("`, `")
            )));
        }
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (name, value) in &self.0 {
            map.serialize_entry(name, value)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for CombinationEntry {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct EntryVisitor;

        impl<'de> Visitor<'de> for EntryVisitor {
            type Value = CombinationEntry;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a mapping of dimension name to partition value")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut pairs = Vec::new();
                // Deliberately not `serde::__private::de::Content`/an
                // `IndexMap`/a `BTreeMap` here — any of those would
                // re-introduce the duplicate-collapsing or duplicate-
                // rejecting behavior this type exists to avoid. Every
                // `(key, value)` `next_entry` yields is kept, in order,
                // even if a key repeats.
                while let Some(pair) = map.next_entry::<String, String>()? {
                    pairs.push(pair);
                }
                Ok(CombinationEntry(pairs))
            }
        }

        deserializer.deserialize_map(EntryVisitor)
    }
}

/// `VoRecord.combinations`'s `deserialize_with`: treats an explicit `null`
/// the same as an empty list, matching an omitted key's `#[serde(default)]`
/// (see that field's own doc comment for why all three — missing, `null`,
/// `[]` — must reach `VoRecord` the same way).
fn deserialize_combinations<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<CombinationEntry>, D::Error> {
    Ok(Option::<Vec<CombinationEntry>>::deserialize(deserializer)?.unwrap_or_default())
}

/// Defines how dimension coverage is evaluated for a verification objective.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CoveragePolicy {
    /// Each dimension is covered independently.
    IndependentAxes,

    /// Every possible combination of dimension partitions must be covered.
    FullProduct,

    /// Only explicitly listed combinations are required.
    Explicit,
}

/// Canonical metadata record for a verification objective.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct VoRecord {
    pub id: VoId,
    pub parent: Option<VoId>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub derives_from: Vec<DerivesFrom>,

    pub claim: String,
    pub dimensions: Vec<Dimension>,
    pub coverage_policy: Option<CoveragePolicy>,

    /// Each entry maps every declared `dimensions[].name` to one of its
    /// partitions (詳細設計 v0.1 §3.2.1: "各 entry は dimension 名 →
    /// partition 値の map とし...記述順・map key 順には依存しない"). See
    /// `CombinationEntry`'s own doc comment for why the element type is not
    /// a plain map.
    ///
    /// `#[serde(default, deserialize_with = "deserialize_combinations")]`
    /// (unlike `dimensions`/`representative_cases`, which stay plain
    /// required fields): 別紙C:97-104 lists `explicit` かつ `combinations`
    /// **欠落**（missing key）、**null**、and an explicit **empty list** as
    /// three distinct inputs that must all reach the *scan* layer as the
    /// same E-SCAN-017 condition, not fail here as a record-layer schema
    /// rejection (E-SCAN-010) before `vtest-scan`'s `invalid_vo_
    /// combinations` (詳細設計 v0.1 §17.1) ever sees the record. `#[serde(
    /// default)]` alone only covers a missing key — a *present* `null`
    /// still fails a plain `Vec<T>` deserialize outright ("invalid type:
    /// unit value, expected a sequence"; confirmed empirically once
    /// `vo_record_from_yaml` started building `VoRecord` directly from text
    /// instead of via `yaml_serde::Value` first — BLOCKER 1, PR #26 review
    /// round 2 — `Value`'s own `null` → `Vec::default()` coercion no longer
    /// applies). `deserialize_combinations` restores that behavior
    /// explicitly, independent of whichever `Value`-based path was doing it
    /// as an unlabeled side effect before.
    #[serde(default, deserialize_with = "deserialize_combinations")]
    pub combinations: Vec<CombinationEntry>,

    pub representative_cases: Vec<String>,
    pub created: String,
    pub updated: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimension_serializes_correctly() {
        let dimension = Dimension {
            name: "dimension1".to_string(),
            partitions: vec!["partitionA".to_string(), "partitionB".to_string()],
        };

        assert_eq!(
            serde_json::to_string(&dimension).unwrap(),
            r#"{"name":"dimension1","partitions":["partitionA","partitionB"]}"#
        );
    }

    #[test]
    fn coverage_policy_serializes_correctly() {
        assert_eq!(
            serde_json::to_string(&CoveragePolicy::IndependentAxes).unwrap(),
            "\"independent-axes\""
        );
        assert_eq!(
            serde_json::to_string(&CoveragePolicy::FullProduct).unwrap(),
            "\"full-product\""
        );
        assert_eq!(
            serde_json::to_string(&CoveragePolicy::Explicit).unwrap(),
            "\"explicit\""
        );
    }

    #[test]
    fn vo_record_serializes_correctly() {
        let vo_record = VoRecord {
            id: VoId::new("vo123"),
            parent: Some(VoId::new("parent456")),
            derives_from: vec![],
            claim: "This is a claim.".to_string(),
            dimensions: vec![],
            coverage_policy: Some(CoveragePolicy::IndependentAxes),
            combinations: vec![],
            representative_cases: vec![],
            created: "2026-08-08".to_string(),
            updated: "2026-08-08".to_string(),
        };

        assert_eq!(
            serde_json::to_string(&vo_record).unwrap(),
            r#"{"id":"vo123","parent":"parent456","claim":"This is a claim.","dimensions":[],"coverage_policy":"independent-axes","combinations":[],"representative_cases":[],"created":"2026-08-08","updated":"2026-08-08"}"#
        );
    }

    #[test]
    fn vo_record_serializes_without_parent() {
        let vo_record = VoRecord {
            id: VoId::new("vo789"),
            parent: None,
            derives_from: vec![],
            claim: "Another claim.".to_string(),
            dimensions: vec![],
            coverage_policy: None,
            combinations: vec![],
            representative_cases: vec![],
            created: "2026-08-08".to_string(),
            updated: "2026-08-08".to_string(),
        };

        assert_eq!(
            serde_json::to_string(&vo_record).unwrap(),
            r#"{"id":"vo789","parent":null,"claim":"Another claim.","dimensions":[],"coverage_policy":null,"combinations":[],"representative_cases":[],"created":"2026-08-08","updated":"2026-08-08"}"#
        );
    }

    #[test]
    fn vo_record_carries_representative_cases() {
        let vo_record = VoRecord {
            id: VoId::new("vo321"),
            parent: None,
            derives_from: vec![],
            claim: "A claim with representative cases.".to_string(),
            dimensions: vec![],
            coverage_policy: None,
            combinations: vec![],
            representative_cases: vec!["empty input".to_string(), "max length input".to_string()],
            created: "2026-08-08".to_string(),
            updated: "2026-08-09".to_string(),
        };

        let json = serde_json::to_string(&vo_record).unwrap();
        assert_eq!(serde_json::from_str::<VoRecord>(&json).unwrap(), vo_record);
    }

    /// 詳細設計 v0.1 §3.2.1's own example: `combinations` entries are maps
    /// from dimension name to partition value, not positional value lists.
    #[test]
    fn vo_record_combinations_are_dimension_keyed_maps() {
        let vo_record = VoRecord {
            id: VoId::new("VO-PARSER-UTF8-003"),
            parent: None,
            derives_from: vec![],
            claim: "claim".to_string(),
            dimensions: vec![
                Dimension {
                    name: "operand-sign".to_string(),
                    partitions: vec!["positive".to_string(), "negative".to_string()],
                },
                Dimension {
                    name: "operator".to_string(),
                    partitions: vec![
                        "add".to_string(),
                        "sub".to_string(),
                        "mul".to_string(),
                        "div".to_string(),
                    ],
                },
            ],
            coverage_policy: Some(CoveragePolicy::Explicit),
            combinations: vec![
                CombinationEntry::from_iter([
                    ("operand-sign".to_string(), "positive".to_string()),
                    ("operator".to_string(), "div".to_string()),
                ]),
                CombinationEntry::from_iter([
                    ("operand-sign".to_string(), "negative".to_string()),
                    ("operator".to_string(), "div".to_string()),
                ]),
            ],
            representative_cases: vec![],
            created: "2026-08-08".to_string(),
            updated: "2026-08-08".to_string(),
        };

        let json = serde_json::to_string(&vo_record).unwrap();
        assert_eq!(serde_json::from_str::<VoRecord>(&json).unwrap(), vo_record);
        assert!(json.contains(r#""combinations":[{"operand-sign":"positive","operator":"div"},"#));
    }

    /// `CombinationEntry`'s whole reason to exist: preserve a repeated
    /// dimension name losslessly, in declaration order, rather than
    /// collapsing or rejecting it (see the type's own doc comment).
    #[test]
    fn combination_entry_deserialize_preserves_a_duplicate_dimension_name() {
        let entry: CombinationEntry =
            serde_json::from_str(r#"{"d1":"a","d1":"b"}"#).expect("duplicate keys are tolerated");
        assert_eq!(
            entry.iter().collect::<Vec<_>>(),
            vec![("d1", "a"), ("d1", "b")],
            "both declared pairs must survive, in declaration order"
        );
        assert_eq!(entry.duplicate_dimension_names(), vec!["d1"]);
        assert_eq!(entry.len(), 2);
    }

    /// A well-formed entry (no repeated dimension name) has no duplicates
    /// to report and round-trips normally.
    #[test]
    fn combination_entry_without_a_duplicate_reports_none() {
        let entry = CombinationEntry::from_iter([
            ("d1".to_string(), "a".to_string()),
            ("d2".to_string(), "x".to_string()),
        ]);
        assert!(entry.duplicate_dimension_names().is_empty());
        assert_eq!(entry.get("d1"), Some("a"));
        assert_eq!(entry.get("d2"), Some("x"));
        assert_eq!(entry.get("d3"), None);
    }

    /// 詳細設計 v0.1 §3.2.1: "記述順・map key 順には依存しない" — two entries
    /// with the same pairs in a different declaration order must still
    /// compare equal, matching the order-independence a `BTreeMap`-backed
    /// entry got for free from the map's own `Eq`/`Ord`.
    #[test]
    fn combination_entry_equality_is_order_independent() {
        let first = CombinationEntry::from_iter([
            ("d1".to_string(), "a".to_string()),
            ("d2".to_string(), "x".to_string()),
        ]);
        let second = CombinationEntry::from_iter([
            ("d2".to_string(), "x".to_string()),
            ("d1".to_string(), "a".to_string()),
        ]);
        assert_eq!(first, second);
    }

    /// The writer-side counterpart to `combination_entry_deserialize_
    /// preserves_a_duplicate_dimension_name`: an entry that still has a
    /// duplicate dimension name must not be serializable — the type can
    /// hold this state (so the reader can hand it to the scan layer
    /// losslessly), but must refuse to re-emit it as YAML/JSON with a
    /// repeated mapping key.
    #[test]
    fn combination_entry_with_a_duplicate_dimension_name_refuses_to_serialize() {
        let entry = CombinationEntry::from_iter([
            ("d1".to_string(), "a".to_string()),
            ("d1".to_string(), "b".to_string()),
        ]);
        assert!(
            serde_json::to_string(&entry).is_err(),
            "serializing a combinations entry with a duplicate dimension name must fail closed"
        );
    }
}
