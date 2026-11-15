//! Test specification data: the `data` of `vtest export` (DS-1701), built
//! from a finished scan.
//!
//! Boundary: this module is a pure function of a [`ScanResult`]. It reads no
//! file, clock, environment variable, process or adapter (BD-335), takes no
//! Evidence, approval, judgment record or verification result (BD-336), emits
//! no diagnostic and cannot fail (DES-607). Rendering the data as Markdown
//! belongs to `vtest-cli` (BD-342); deciding `ok` and the exit code belongs
//! to the operation (DES-611).
//!
//! Clauses: BD-332, BD-334 to BD-336, DES-600 to DES-607, DES-609, DS-1707
//! to DS-1742, DS-1765.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use vtest_model::{
    AdapterId, ManagedTestLink, SourceLocation, TargetRef, TestEntity, TestId, VoId,
};

use crate::ScanResult;

/// What a scan hands to [`test_specification`] besides `tests` and
/// `discovered` (DES-600): the diagnostic codes of each unmanaged construct,
/// the result of resolving every `covers` and target reference, and the
/// claim of every VO whose record could be read.
///
/// These are values, so the test specification never has to recover them
/// from a diagnostic's `message` (DES-602).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TestSpecBasis {
    pub construct_codes: Vec<ConstructCodes>,
    pub references: Vec<ReferenceResolution>,
    pub vo_claims: BTreeMap<String, String>,
}

/// The codes of the diagnostics attached to one Discovered Test that has no
/// managed Test Entity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConstructCodes {
    pub adapter: AdapterId,
    pub location: SourceLocation,
    pub codes: Vec<String>,
}

/// A `covers` or target reference as a Test declared it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeclaredReference {
    Covers(VoId),
    Target(TargetRef),
}

/// The result of resolving one declared reference of one Test (DES-601).
/// `code` is `None` when the reference resolved, and the diagnostic code of
/// the failure otherwise.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferenceResolution {
    pub test: TestId,
    pub location: SourceLocation,
    pub reference: DeclaredReference,
    pub code: Option<String>,
}

/// The test specification (DES-603). Field order is the serialized key order
/// (DES-609, DS-1739).
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TestSpecification {
    pub tests: Vec<SpecTest>,
    pub unmanaged: Vec<SpecUnmanaged>,
    pub unresolved: Vec<SpecUnresolved>,
    pub by_vo: Vec<SpecVoIndex>,
    pub by_path: Vec<SpecPathIndex>,
}

/// One Test item. This is not a `TestEntity`: it has no `execution`, content
/// hash, line number or byte position (DES-604, DS-1718, DS-1741).
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SpecTest {
    pub key: String,
    pub id: String,
    pub intent: String,
    pub covers: Vec<SpecCover>,
    pub targets: Vec<SpecTarget>,
    pub input: Option<String>,
    pub expect: Option<String>,
    pub kind: Option<String>,
    pub cases: Vec<String>,
    pub related: Vec<SpecRelated>,
    pub location: SpecLocation,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SpecCover {
    pub vo: String,
    pub claim: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SpecTarget {
    pub reference: SpecTargetRef,
    pub resolved: bool,
}

/// A Target Reference as declared (DS-1713). It is never replaced by the
/// canonical Locator it resolved to.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SpecTargetRef {
    Locator { adapter: String, value: String },
    SrcId { id: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SpecRelated {
    pub id: String,
    pub test: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SpecLocation {
    pub adapter: String,
    pub path: String,
    pub locator: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SpecUnmanaged {
    pub location: SpecLocation,
    pub codes: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SpecReferenceField {
    Covers,
    Related,
    Target,
}

impl SpecReferenceField {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Covers => "covers",
            Self::Related => "related",
            Self::Target => "target",
        }
    }
}

/// The declared reference of an unresolved entry: a VO ID or a Test ID as a
/// string, a Target Reference as an object (DS-1724).
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum SpecReference {
    Id(String),
    Target(SpecTargetRef),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SpecUnresolved {
    pub test: String,
    pub field: SpecReferenceField,
    pub reference: SpecReference,
    pub code: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SpecVoIndex {
    pub vo: String,
    pub claim: Option<String>,
    pub tests: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SpecPathIndex {
    pub adapter: String,
    pub path: String,
    pub tests: Vec<String>,
}

/// DS-1709: bytes in `[A-Za-z0-9-]` stay, every other byte becomes `_` and
/// two upper-case hex digits.
pub fn identifier_encoding(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || byte == b'-' {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("_{byte:02X}"));
        }
    }
    encoded
}

/// DS-1765: a string of the comparison JSON.
fn comparison_string(value: &str, out: &mut String) {
    out.push('"');
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{0C}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            control if control < '\u{20}' => {
                out.push_str(&format!("\\u{:04x}", u32::from(control)));
            }
            other => out.push(other),
        }
    }
    out.push('"');
}

fn comparison_strings<'a>(values: impl IntoIterator<Item = &'a str>, out: &mut String) {
    out.push('[');
    for (index, value) in values.into_iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        comparison_string(value, out);
    }
    out.push(']');
}

fn comparison_optional(value: Option<&str>, out: &mut String) {
    match value {
        Some(value) => comparison_string(value, out),
        None => out.push_str("null"),
    }
}

/// DS-1731 / DS-1732: the declared content of a Test as comparison JSON.
/// Object keys are written in byte order.
fn declaration_comparison_json(test: &TestEntity) -> String {
    let mut out = String::new();
    out.push_str("{\"cases\":");
    comparison_strings(test.cases.iter().map(String::as_str), &mut out);
    out.push_str(",\"covers\":");
    comparison_strings(test.covers.iter().map(VoId::as_str), &mut out);
    out.push_str(",\"expect\":");
    comparison_optional(test.expect.as_deref(), &mut out);
    out.push_str(",\"input\":");
    comparison_optional(test.input.as_deref(), &mut out);
    out.push_str(",\"intent\":");
    comparison_string(&test.intent, &mut out);
    out.push_str(",\"kind\":");
    comparison_optional(test.kind.as_deref(), &mut out);
    out.push_str(",\"related\":");
    comparison_strings(test.related.iter().map(TestId::as_str), &mut out);
    out.push_str(",\"targets\":[");
    for (index, target) in test.targets.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        match target {
            TargetRef::Locator(locator) => {
                out.push_str("{\"adapter\":");
                comparison_string(locator.adapter.as_str(), &mut out);
                out.push_str(",\"kind\":\"locator\",\"value\":");
                comparison_string(&locator.value, &mut out);
                out.push('}');
            }
            TargetRef::SrcId(src_id) => {
                out.push_str("{\"id\":");
                comparison_string(src_id.as_str(), &mut out);
                out.push_str(",\"kind\":\"src_id\"}");
            }
        }
    }
    out.push_str("]}");
    out
}

/// DS-1714: the three strings a Target Reference is ordered by.
fn target_order(reference: &SpecTargetRef) -> [&str; 3] {
    match reference {
        SpecTargetRef::Locator { adapter, value } => ["locator", adapter, value],
        SpecTargetRef::SrcId { id } => ["src_id", id, ""],
    }
}

fn reference_order(reference: &SpecReference) -> [&str; 3] {
    match reference {
        SpecReference::Id(id) => [id, "", ""],
        SpecReference::Target(target) => target_order(target),
    }
}

fn spec_target(target: &TargetRef) -> SpecTargetRef {
    match target {
        TargetRef::Locator(locator) => SpecTargetRef::Locator {
            adapter: locator.adapter.as_str().to_owned(),
            value: locator.value.clone(),
        },
        TargetRef::SrcId(src_id) => SpecTargetRef::SrcId {
            id: src_id.as_str().to_owned(),
        },
    }
}

fn spec_location(location: &SourceLocation) -> SpecLocation {
    SpecLocation {
        adapter: location.adapter.as_str().to_owned(),
        path: location.path.as_str().to_owned(),
        locator: location.locator.clone(),
    }
}

/// Identity of a construct inside one scan: adapter, path, locator and byte
/// range. Used only to join the basis to `tests` / `discovered`; none of it
/// except adapter, path and locator reaches the output.
type ConstructKey = (String, String, String, u64, u64);

fn construct_key(location: &SourceLocation) -> ConstructKey {
    (
        location.adapter.as_str().to_owned(),
        location.path.as_str().to_owned(),
        location.locator.clone(),
        location.byte_range.start,
        location.byte_range.end,
    )
}

type ResolutionKey = (String, ConstructKey, SpecReferenceField, [String; 3]);

fn resolution_key(
    test: &TestId,
    location: &SourceLocation,
    field: SpecReferenceField,
    order: [&str; 3],
) -> ResolutionKey {
    (
        test.as_str().to_owned(),
        construct_key(location),
        field,
        order.map(str::to_owned),
    )
}

/// Builds the test specification from a finished scan (BD-332).
///
/// The result does not depend on the order of `scan.tests`,
/// `scan.discovered` or the lists of `scan.test_spec_basis` (DES-606).
///
/// A reference the basis does not mention is treated as resolved: the scan
/// reports every `covers` and target reference of every managed Test, so
/// this only concerns a `ScanResult` built by hand.
pub fn test_specification(scan: &ScanResult) -> TestSpecification {
    let basis = &scan.test_spec_basis;

    // Unresolved wins if the basis names the same reference twice with
    // different results; the scan never does.
    let mut resolutions = BTreeMap::<ResolutionKey, Option<String>>::new();
    for resolution in &basis.references {
        let key = match &resolution.reference {
            DeclaredReference::Covers(vo) => resolution_key(
                &resolution.test,
                &resolution.location,
                SpecReferenceField::Covers,
                [vo.as_str(), "", ""],
            ),
            DeclaredReference::Target(target) => resolution_key(
                &resolution.test,
                &resolution.location,
                SpecReferenceField::Target,
                target_order(&spec_target(target)),
            ),
        };
        let entry = resolutions.entry(key).or_insert(None);
        *entry = match (entry.take(), resolution.code.clone()) {
            (Some(left), Some(right)) => Some(left.min(right)),
            (Some(code), None) | (None, Some(code)) => Some(code),
            (None, None) => None,
        };
    }

    // DS-1730: adapter ID, path, Test ID, locator, declared content.
    let mut ordered = scan
        .tests
        .iter()
        .map(|test| {
            (
                (
                    test.location.adapter.as_str(),
                    test.location.path.as_str(),
                    test.id.as_str(),
                    test.location.locator.as_str(),
                    declaration_comparison_json(test),
                ),
                test,
            )
        })
        .collect::<Vec<_>>();
    ordered.sort_by(|left, right| left.0.cmp(&right.0));

    // DS-1708: the key is the encoded Test ID, plus `~<rank>` when the ID is
    // shared.
    let mut id_counts = BTreeMap::<&str, usize>::new();
    for test in &scan.tests {
        *id_counts.entry(test.id.as_str()).or_default() += 1;
    }
    let mut ranks = BTreeMap::<&str, usize>::new();
    let mut keys = Vec::with_capacity(ordered.len());
    let mut keys_by_id = BTreeMap::<&str, Vec<String>>::new();
    for (_, test) in &ordered {
        let id = test.id.as_str();
        let key = if id_counts[id] == 1 {
            identifier_encoding(id)
        } else {
            let rank = ranks.entry(id).or_default();
            *rank += 1;
            format!("{}~{rank}", identifier_encoding(id))
        };
        keys_by_id.entry(id).or_default().push(key.clone());
        keys.push(key);
    }

    let mut tests = Vec::with_capacity(ordered.len());
    let mut unresolved = Vec::new();
    let mut unresolved_vos = BTreeSet::<&str>::new();
    for ((_, test), key) in ordered.iter().zip(&keys) {
        let covers = test
            .covers
            .iter()
            .map(|vo| {
                let code = resolutions
                    .get(&resolution_key(
                        &test.id,
                        &test.location,
                        SpecReferenceField::Covers,
                        [vo.as_str(), "", ""],
                    ))
                    .cloned()
                    .flatten();
                let claim = match code {
                    Some(code) => {
                        unresolved_vos.insert(vo.as_str());
                        unresolved.push(SpecUnresolved {
                            test: key.clone(),
                            field: SpecReferenceField::Covers,
                            reference: SpecReference::Id(vo.as_str().to_owned()),
                            code: Some(code),
                        });
                        None
                    }
                    None => basis.vo_claims.get(vo.as_str()).cloned(),
                };
                SpecCover {
                    vo: vo.as_str().to_owned(),
                    claim,
                }
            })
            .collect();
        let targets = test
            .targets
            .iter()
            .map(|target| {
                let reference = spec_target(target);
                let code = resolutions
                    .get(&resolution_key(
                        &test.id,
                        &test.location,
                        SpecReferenceField::Target,
                        target_order(&reference),
                    ))
                    .cloned()
                    .flatten();
                let resolved = code.is_none();
                if let Some(code) = code {
                    unresolved.push(SpecUnresolved {
                        test: key.clone(),
                        field: SpecReferenceField::Target,
                        reference: SpecReference::Target(reference.clone()),
                        code: Some(code),
                    });
                }
                SpecTarget {
                    reference,
                    resolved,
                }
            })
            .collect();
        let related = test
            .related
            .iter()
            .map(|related| {
                // DS-1722: resolved only when exactly one managed Test has
                // this ID.
                let candidates = keys_by_id
                    .get(related.as_str())
                    .map(Vec::as_slice)
                    .unwrap_or_default();
                let resolved = match candidates {
                    [single] => Some(single.clone()),
                    _ => None,
                };
                if resolved.is_none() {
                    unresolved.push(SpecUnresolved {
                        test: key.clone(),
                        field: SpecReferenceField::Related,
                        reference: SpecReference::Id(related.as_str().to_owned()),
                        // DS-1725: two or more is the Test ID collision; none
                        // has no diagnostic.
                        code: (!candidates.is_empty()).then(|| "E-SCAN-002".to_owned()),
                    });
                }
                SpecRelated {
                    id: related.as_str().to_owned(),
                    test: resolved,
                }
            })
            .collect();
        tests.push(SpecTest {
            key: key.clone(),
            id: test.id.as_str().to_owned(),
            intent: test.intent.clone(),
            covers,
            targets,
            input: test.input.clone(),
            expect: test.expect.clone(),
            kind: test.kind.clone(),
            cases: test.cases.clone(),
            related,
            location: spec_location(&test.location),
        });
    }

    // DS-1734.
    unresolved.sort_by(|left, right| {
        (
            left.test.as_str(),
            left.field.as_str(),
            reference_order(&left.reference),
            left.code.as_deref().unwrap_or(""),
        )
            .cmp(&(
                right.test.as_str(),
                right.field.as_str(),
                reference_order(&right.reference),
                right.code.as_deref().unwrap_or(""),
            ))
    });

    // DS-1719 / DS-1720 / DS-1733.
    let mut codes_by_construct = BTreeMap::<ConstructKey, BTreeSet<&str>>::new();
    for construct in &basis.construct_codes {
        let mut key = construct_key(&construct.location);
        key.0 = construct.adapter.as_str().to_owned();
        codes_by_construct
            .entry(key)
            .or_default()
            .extend(construct.codes.iter().map(String::as_str));
    }
    let mut unmanaged = scan
        .discovered
        .iter()
        .filter(|discovered| discovered.managed == ManagedTestLink::Missing)
        .map(|discovered| {
            let mut key = construct_key(&discovered.location);
            key.0 = discovered.adapter.as_str().to_owned();
            let codes = codes_by_construct
                .get(&key)
                .map(|codes| codes.iter().map(|code| (*code).to_owned()).collect())
                .unwrap_or_default();
            SpecUnmanaged {
                location: SpecLocation {
                    adapter: discovered.adapter.as_str().to_owned(),
                    path: discovered.location.path.as_str().to_owned(),
                    locator: discovered.location.locator.clone(),
                },
                codes,
            }
        })
        .collect::<Vec<_>>();
    unmanaged.sort_by_cached_key(|entry| {
        let mut codes = String::new();
        comparison_strings(entry.codes.iter().map(String::as_str), &mut codes);
        (
            entry.location.adapter.clone(),
            entry.location.path.clone(),
            entry.location.locator.clone(),
            codes,
        )
    });

    // DS-1728 / DS-1729 / DS-1735. `tests` is already in output order, so
    // pushing while skipping repeats keeps that order.
    let mut vo_tests = BTreeMap::<&str, Vec<String>>::new();
    let mut path_tests = BTreeMap::<(&str, &str), Vec<String>>::new();
    for test in &tests {
        for cover in &test.covers {
            let keys = vo_tests.entry(cover.vo.as_str()).or_default();
            if keys.last() != Some(&test.key) {
                keys.push(test.key.clone());
            }
        }
        path_tests
            .entry((test.location.adapter.as_str(), test.location.path.as_str()))
            .or_default()
            .push(test.key.clone());
    }
    let by_vo = vo_tests
        .into_iter()
        .map(|(vo, tests)| SpecVoIndex {
            vo: vo.to_owned(),
            claim: if unresolved_vos.contains(vo) {
                None
            } else {
                basis.vo_claims.get(vo).cloned()
            },
            tests,
        })
        .collect();
    let by_path = path_tests
        .into_iter()
        .map(|((adapter, path), tests)| SpecPathIndex {
            adapter: adapter.to_owned(),
            path: path.to_owned(),
            tests,
        })
        .collect();

    TestSpecification {
        tests,
        unmanaged,
        unresolved,
        by_vo,
        by_path,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ScanResult;
    use vtest_model::{
        ContentHash, DiscoveredTest, ExecutionDescriptor, Locator, ProjectPath, ScanSummary,
        SourceRange, SrcId,
    };

    fn location(adapter: &str, path: &str, locator: &str, start: u64) -> SourceLocation {
        SourceLocation {
            adapter: AdapterId::new(adapter),
            path: ProjectPath::new(path),
            locator: locator.to_owned(),
            byte_range: SourceRange {
                start,
                end: start + 10,
            },
        }
    }

    fn test(id: &str, location: SourceLocation) -> TestEntity {
        TestEntity {
            id: TestId::new(id),
            covers: vec![VoId::new("VO-A")],
            targets: Vec::new(),
            intent: format!("intent of {id}"),
            input: None,
            expect: None,
            kind: None,
            cases: Vec::new(),
            related: Vec::new(),
            execution: ExecutionDescriptor {
                adapter: location.adapter.clone(),
                project: Some("package".to_owned()),
                suite: None,
                selector: "selector".to_owned(),
            },
            content_hash: ContentHash::from_text(id),
            location,
        }
    }

    fn locator_target(value: &str) -> TargetRef {
        TargetRef::Locator(Locator {
            adapter: AdapterId::new("rust-cargo"),
            value: value.to_owned(),
        })
    }

    /// A scan whose basis reports every `covers` and target reference of
    /// every Test, the way `scan_project` does. `unresolved` names the
    /// references that failed and their codes.
    fn scan(
        tests: Vec<TestEntity>,
        missing: Vec<(SourceLocation, Vec<&str>)>,
        unresolved: &[(&str, &str)],
        claims: &[(&str, &str)],
    ) -> ScanResult {
        let code_of = |spelling: &str| {
            unresolved
                .iter()
                .find(|(reference, _)| *reference == spelling)
                .map(|(_, code)| (*code).to_owned())
        };
        let mut references = Vec::new();
        for test in &tests {
            for vo in &test.covers {
                references.push(ReferenceResolution {
                    test: test.id.clone(),
                    location: test.location.clone(),
                    reference: DeclaredReference::Covers(vo.clone()),
                    code: code_of(vo.as_str()),
                });
            }
            for target in &test.targets {
                let spelling = match target {
                    TargetRef::Locator(locator) => locator.value.clone(),
                    TargetRef::SrcId(src_id) => src_id.as_str().to_owned(),
                };
                references.push(ReferenceResolution {
                    test: test.id.clone(),
                    location: test.location.clone(),
                    reference: DeclaredReference::Target(target.clone()),
                    code: code_of(&spelling),
                });
            }
        }
        let mut discovered = tests
            .iter()
            .map(|test| DiscoveredTest {
                adapter: test.location.adapter.clone(),
                location: test.location.clone(),
                content_hash: ContentHash::from_text("construct"),
                managed: ManagedTestLink::One(test.id.clone()),
            })
            .collect::<Vec<_>>();
        let mut construct_codes = Vec::new();
        for (location, codes) in missing {
            construct_codes.push(ConstructCodes {
                adapter: location.adapter.clone(),
                location: location.clone(),
                codes: codes.into_iter().map(str::to_owned).collect(),
            });
            discovered.push(DiscoveredTest {
                adapter: location.adapter.clone(),
                location,
                content_hash: ContentHash::from_text("construct"),
                managed: ManagedTestLink::Missing,
            });
        }
        ScanResult {
            summary: ScanSummary {
                files: 0,
                tests: tests.len() as u64,
                sources: 0,
            },
            tests,
            discovered,
            sources: Vec::new(),
            diagnostics: Vec::new(),
            test_spec_basis: TestSpecBasis {
                construct_codes,
                references,
                vo_claims: claims
                    .iter()
                    .map(|(vo, claim)| ((*vo).to_owned(), (*claim).to_owned()))
                    .collect(),
            },
        }
    }

    fn keys(specification: &TestSpecification) -> Vec<&str> {
        specification
            .tests
            .iter()
            .map(|test| test.key.as_str())
            .collect()
    }

    /// @vtest.id TEST-SCAN-TEST-SPEC-ITEM-HAS-ELEVEN-KEYS
    /// @vtest.covers VO-SCAN-TEST-SPEC-TESTS-ITEM-KEYS
    /// @vtest.target crates/vtest-scan/src/test_spec.rs::test_specification
    /// @vtest.intent a Test item serializes exactly key, id, intent, covers, targets, input, expect, kind, cases, related, location in that order, with null for an undeclared scalar, an empty list for an undeclared list, and a location of adapter, path, locator only (DS-1707, DS-1716, DS-1717, DS-1718)
    #[test]
    fn a_test_item_serializes_exactly_the_eleven_keys_in_a_fixed_order() {
        let scan = scan(
            vec![test(
                "TEST-A",
                location("rust-cargo", "src/a.rs", "tests::a", 0),
            )],
            Vec::new(),
            &[],
            &[("VO-A", "claim A")],
        );

        let json = serde_json::to_string(&test_specification(&scan)).unwrap();

        assert_eq!(
            json,
            concat!(
                r#"{"tests":[{"key":"TEST-A","id":"TEST-A","intent":"intent of TEST-A","#,
                r#""covers":[{"vo":"VO-A","claim":"claim A"}],"targets":[],"input":null,"#,
                r#""expect":null,"kind":null,"cases":[],"related":[],"#,
                r#""location":{"adapter":"rust-cargo","path":"src/a.rs","locator":"tests::a"}}],"#,
                r#""unmanaged":[],"unresolved":[],"#,
                r#""by_vo":[{"vo":"VO-A","claim":"claim A","tests":["TEST-A"]}],"#,
                r#""by_path":[{"adapter":"rust-cargo","path":"src/a.rs","tests":["TEST-A"]}]}"#
            )
        );
    }

    /// @vtest.id TEST-SCAN-TEST-SPEC-KEY-RANKS-SHARED-TEST-IDS
    /// @vtest.covers VO-SCAN-TEST-SPEC-KEY-FROM-TEST-ID
    /// @vtest.target crates/vtest-scan/src/test_spec.rs::test_specification
    /// @vtest.intent a unique Test ID becomes its identifier encoding, and two Tests sharing an ID are both kept with ~1 and ~2 appended in output order (DS-1708, DS-1709)
    #[test]
    fn a_shared_test_id_gets_a_rank_and_a_unique_one_is_only_encoded() {
        let scan = scan(
            vec![
                test(
                    "TEST-DUP",
                    location("rust-cargo", "src/b.rs", "tests::b", 0),
                ),
                test(
                    "TEST-DUP",
                    location("rust-cargo", "src/a.rs", "tests::a", 0),
                ),
                test("T 1/é", location("rust-cargo", "src/c.rs", "tests::c", 0)),
            ],
            Vec::new(),
            &[],
            &[],
        );

        let specification = test_specification(&scan);

        assert_eq!(
            keys(&specification),
            ["TEST-DUP~1", "TEST-DUP~2", "T_201_2F_C3_A9"]
        );
        assert_eq!(specification.tests[0].location.path, "src/a.rs");
        assert_eq!(specification.tests[1].location.path, "src/b.rs");
    }

    /// @vtest.id TEST-SCAN-TEST-SPEC-TESTS-SORTED-BY-ADAPTER-PATH-ID-LOCATOR-CONTENT
    /// @vtest.covers VO-SCAN-TEST-SPEC-TESTS-ORDER
    /// @vtest.target crates/vtest-scan/src/test_spec.rs::test_specification
    /// @vtest.intent data.tests is ordered by adapter ID, then path, then Test ID, then locator, then the declared content, each compared as bytes (DS-1730, DS-1737)
    #[test]
    fn tests_are_ordered_by_adapter_path_id_locator_and_declared_content() {
        let mut same_place_later = test("TEST-B", location("rust-cargo", "src/a.rs", "x", 0));
        same_place_later.intent = "z".to_owned();
        let mut same_place_earlier = test("TEST-B", location("rust-cargo", "src/a.rs", "x", 0));
        same_place_earlier.intent = "a".to_owned();
        let scan = scan(
            vec![
                test("TEST-A", location("synthetic", "a.txt", "a", 0)),
                same_place_later,
                test("TEST-B", location("rust-cargo", "src/a.rs", "w", 0)),
                test("TEST-A", location("rust-cargo", "src/b.rs", "a", 0)),
                same_place_earlier,
                test("TEST-Z", location("rust-cargo", "src/Z.rs", "a", 0)),
            ],
            Vec::new(),
            &[],
            &[],
        );

        let specification = test_specification(&scan);

        let order = specification
            .tests
            .iter()
            .map(|test| {
                format!(
                    "{} {} {} {} {}",
                    test.location.adapter,
                    test.location.path,
                    test.id,
                    test.location.locator,
                    test.intent
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            order,
            [
                "rust-cargo src/Z.rs TEST-Z a intent of TEST-Z",
                "rust-cargo src/a.rs TEST-B w intent of TEST-B",
                "rust-cargo src/a.rs TEST-B x a",
                "rust-cargo src/a.rs TEST-B x z",
                "rust-cargo src/b.rs TEST-A a intent of TEST-A",
                "synthetic a.txt TEST-A a intent of TEST-A",
            ]
        );
    }

    fn rich_scan() -> ScanResult {
        let mut first = test("TEST-A", location("rust-cargo", "src/a.rs", "tests::a", 0));
        first.covers = vec![VoId::new("VO-MISSING"), VoId::new("VO-A")];
        first.targets = vec![
            TargetRef::SrcId(SrcId::new("SRC-TWICE")),
            locator_target("src/a.rs::gone"),
            locator_target("src/a.rs::here"),
        ];
        first.related = vec![
            TestId::new("TEST-NOWHERE"),
            TestId::new("TEST-DUP"),
            TestId::new("TEST-B"),
        ];
        first.input = Some("input".to_owned());
        first.cases = vec!["second".to_owned(), "first".to_owned()];
        let second = test("TEST-B", location("rust-cargo", "src/a.rs", "tests::b", 20));
        scan(
            vec![
                first,
                second,
                test("TEST-DUP", location("synthetic", "x.txt", "one", 0)),
                test("TEST-DUP", location("synthetic", "x.txt", "two", 20)),
            ],
            vec![
                (
                    location("rust-cargo", "src/a.rs", "tests::plain", 40),
                    vec!["W-SCAN-101"],
                ),
                (
                    location("rust-cargo", "src/a.rs", "tests::broken", 60),
                    vec!["E-SCAN-007", "E-SCAN-006", "E-SCAN-007"],
                ),
            ],
            &[
                ("VO-MISSING", "E-SCAN-003"),
                ("SRC-TWICE", "E-SCAN-011"),
                ("src/a.rs::gone", "E-SCAN-004"),
            ],
            &[("VO-A", "claim A")],
        )
    }

    /// @vtest.id TEST-SCAN-TEST-SPEC-UNRESOLVED-LISTS-EVERY-FAILED-REFERENCE
    /// @vtest.covers VO-SCAN-TEST-SPEC-UNRESOLVED-REFERENCES
    /// @vtest.target crates/vtest-scan/src/test_spec.rs::test_specification
    /// @vtest.intent data.unresolved lists the failed covers, target and related references with the code the scan gave (E-SCAN-003, E-SCAN-004 or E-SCAN-011, E-SCAN-002 for a related ID held by two Tests, null for one held by none), ordered by test, field, reference, code, while the Test item keeps all of them (DS-1721, DS-1722, DS-1725, DS-1727, DS-1734)
    #[test]
    fn unresolved_lists_every_failed_reference_with_the_code_the_scan_gave() {
        let specification = test_specification(&rich_scan());

        let unresolved = serde_json::to_value(&specification.unresolved).unwrap();
        assert_eq!(
            unresolved,
            serde_json::json!([
                {"test": "TEST-A", "field": "covers", "reference": "VO-MISSING", "code": "E-SCAN-003"},
                {"test": "TEST-A", "field": "related", "reference": "TEST-DUP", "code": "E-SCAN-002"},
                {"test": "TEST-A", "field": "related", "reference": "TEST-NOWHERE", "code": null},
                {"test": "TEST-A", "field": "target",
                 "reference": {"kind": "locator", "adapter": "rust-cargo", "value": "src/a.rs::gone"},
                 "code": "E-SCAN-004"},
                {"test": "TEST-A", "field": "target",
                 "reference": {"kind": "src_id", "id": "SRC-TWICE"}, "code": "E-SCAN-011"},
            ])
        );

        let item = serde_json::to_value(&specification.tests[0]).unwrap();
        assert_eq!(
            item["covers"],
            serde_json::json!([
                {"vo": "VO-MISSING", "claim": null},
                {"vo": "VO-A", "claim": "claim A"},
            ])
        );
        assert_eq!(
            item["targets"],
            serde_json::json!([
                {"reference": {"kind": "src_id", "id": "SRC-TWICE"}, "resolved": false},
                {"reference": {"kind": "locator", "adapter": "rust-cargo", "value": "src/a.rs::gone"},
                 "resolved": false},
                {"reference": {"kind": "locator", "adapter": "rust-cargo", "value": "src/a.rs::here"},
                 "resolved": true},
            ])
        );
        assert_eq!(
            item["related"],
            serde_json::json!([
                {"id": "TEST-NOWHERE", "test": null},
                {"id": "TEST-DUP", "test": null},
                {"id": "TEST-B", "test": "TEST-B"},
            ])
        );
        assert_eq!(item["cases"], serde_json::json!(["second", "first"]));
    }

    /// @vtest.id TEST-SCAN-TEST-SPEC-UNMANAGED-CARRIES-CONSTRUCT-CODES
    /// @vtest.covers VO-SCAN-TEST-SPEC-UNMANAGED-CODES
    /// @vtest.target crates/vtest-scan/src/test_spec.rs::test_specification
    /// @vtest.intent data.unmanaged holds every Discovered Test without a managed Test Entity and none of the managed ones, each with the codes the scan attached to that construct, without repeats and in ascending order, ordered by adapter, path, locator (DS-1719, DS-1720, DS-1733)
    #[test]
    fn unmanaged_holds_each_missing_construct_with_its_sorted_distinct_codes() {
        let specification = test_specification(&rich_scan());

        assert_eq!(
            serde_json::to_value(&specification.unmanaged).unwrap(),
            serde_json::json!([
                {"location": {"adapter": "rust-cargo", "path": "src/a.rs", "locator": "tests::broken"},
                 "codes": ["E-SCAN-006", "E-SCAN-007"]},
                {"location": {"adapter": "rust-cargo", "path": "src/a.rs", "locator": "tests::plain"},
                 "codes": ["W-SCAN-101"]},
            ])
        );
    }

    /// @vtest.id TEST-SCAN-TEST-SPEC-INDEXES-BY-VO-AND-PATH
    /// @vtest.covers VO-SCAN-TEST-SPEC-INDEX-ORDER
    /// @vtest.target crates/vtest-scan/src/test_spec.rs::test_specification
    /// @vtest.intent data.by_vo has one element per declared VO ID in ascending order with a null claim for an unresolved VO, data.by_path one per adapter and path in ascending order, and each lists its Test keys once in data.tests order (DS-1728, DS-1729, DS-1735)
    #[test]
    fn indexes_list_test_keys_once_in_tests_order() {
        let mut twice = test("TEST-C", location("rust-cargo", "src/a.rs", "tests::c", 80));
        twice.covers = vec![VoId::new("VO-A"), VoId::new("VO-A")];
        let mut scan = rich_scan();
        scan.test_spec_basis.references.push(ReferenceResolution {
            test: twice.id.clone(),
            location: twice.location.clone(),
            reference: DeclaredReference::Covers(VoId::new("VO-A")),
            code: None,
        });
        scan.tests.push(twice);

        let specification = test_specification(&scan);

        assert_eq!(
            serde_json::to_value(&specification.by_vo).unwrap(),
            serde_json::json!([
                {"vo": "VO-A", "claim": "claim A",
                 "tests": ["TEST-A", "TEST-B", "TEST-C", "TEST-DUP~1", "TEST-DUP~2"]},
                {"vo": "VO-MISSING", "claim": null, "tests": ["TEST-A"]},
            ])
        );
        assert_eq!(
            serde_json::to_value(&specification.by_path).unwrap(),
            serde_json::json!([
                {"adapter": "rust-cargo", "path": "src/a.rs", "tests": ["TEST-A", "TEST-B", "TEST-C"]},
                {"adapter": "synthetic", "path": "x.txt", "tests": ["TEST-DUP~1", "TEST-DUP~2"]},
            ])
        );
    }

    /// @vtest.id TEST-SCAN-TEST-SPEC-SAME-DATA-FOR-ANY-INPUT-ORDER
    /// @vtest.covers VO-SCAN-TEST-SPEC-INPUT-ORDER-INDEPENDENT
    /// @vtest.target crates/vtest-scan/src/test_spec.rs::test_specification
    /// @vtest.intent reversing the Tests, the Discovered Tests, the construct codes and the reference resolutions of a scan yields the same test specification, and so does building it twice (DES-606)
    #[test]
    fn reversing_every_input_list_yields_the_same_data() {
        let scan = rich_scan();
        let mut reversed = scan.clone();
        reversed.tests.reverse();
        reversed.discovered.reverse();
        reversed.test_spec_basis.construct_codes.reverse();
        reversed.test_spec_basis.references.reverse();

        let expected = serde_json::to_string(&test_specification(&scan)).unwrap();

        assert_eq!(
            serde_json::to_string(&test_specification(&scan)).unwrap(),
            expected
        );
        assert_eq!(
            serde_json::to_string(&test_specification(&reversed)).unwrap(),
            expected
        );
    }

    /// @vtest.id TEST-SCAN-TEST-SPEC-COMPARISON-JSON-ESCAPES
    /// @vtest.covers VO-SCAN-TEST-SPEC-COMPARISON-JSON-STRING
    /// @vtest.target crates/vtest-scan/src/test_spec.rs::declaration_comparison_json
    /// @vtest.intent the comparison JSON of a declaration writes its eight keys in byte order without whitespace, escapes the quote, the backslash and U+0000 to U+001F with the short forms for b, t, n, f, r and lower-case \u otherwise, and leaves other characters as UTF-8 (DS-1731, DS-1732, DS-1765)
    #[test]
    fn comparison_json_has_sorted_keys_and_the_fixed_string_escapes() {
        let mut entity = test("TEST-A", location("rust-cargo", "src/a.rs", "a", 0));
        entity.intent = "\"\\\u{08}\t\n\u{0C}\r\u{1F}\u{00}é/\u{7F}".to_owned();
        entity.kind = Some("unit".to_owned());
        entity.targets = vec![
            locator_target("src/a.rs::f"),
            TargetRef::SrcId(SrcId::new("SRC-1")),
        ];
        entity.related = vec![TestId::new("TEST-B")];

        assert_eq!(
            declaration_comparison_json(&entity),
            concat!(
                r#"{"cases":[],"covers":["VO-A"],"expect":null,"input":null,"#,
                r#""intent":"\"\\\b\t\n\f\r\u001f\u0000é/"#,
                "\u{7F}",
                r#"","kind":"unit","related":["TEST-B"],"#,
                r#""targets":[{"adapter":"rust-cargo","kind":"locator","value":"src/a.rs::f"},"#,
                r#"{"id":"SRC-1","kind":"src_id"}]}"#
            )
        );
    }
}
