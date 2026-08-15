//! Approval upstream dependency closure and the VO status derived from it.
//!
//! The closure is rebuilt from the canonical records on every read: the
//! recursive parent VO chain, every REQ those VOs reference, the recursive
//! parent REQ chain, and every SPEC any of them cite. The approved VO itself is
//! bound through `subject_hash` and never appears in its own closure.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt, fs,
};

use vtest_model::{
    hash_approval_subject, hash_record_subject, hash_spec_subject, hash_specification_source,
    CanonicalProjection, ContentHash,
};

use crate::{
    is_valid_ulid, read_approval, read_req, read_spec, read_text, read_vo, ApprovalDependency,
    ApprovalRecord, Dimension, ReqRecord, SpecRecord, SpecRef, StoreError, VerifyLayout, VoRecord,
};

/// One upstream entity that cannot be resolved completely and currently.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClosureFailure {
    pub kind: &'static str,
    pub id: String,
    pub reason: String,
}

impl fmt::Display for ClosureFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "upstream {} {} is not completely and currently resolvable: {}",
            self.kind, self.id, self.reason
        )
    }
}

/// Why an Approval bound to the current VO content is still not an approval.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ApprovalInvalidity {
    /// Version 1 compatibility Approval recorded without any closure.
    ClosureAbsent,
    /// The recorded closure is no longer the current one.
    ClosureChanged,
    /// The current closure cannot be resolved at all.
    ClosureUnresolvable(ClosureFailure),
}

impl fmt::Display for ApprovalInvalidity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ClosureAbsent => {
                formatter.write_str("compatibility Approval has no upstream dependency closure")
            }
            Self::ClosureChanged => {
                formatter.write_str("recorded upstream dependency closure is out of date")
            }
            Self::ClosureUnresolvable(failure) => write!(formatter, "{failure}"),
        }
    }
}

/// The approval-derived state of one VO.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DerivedVoStatus {
    pub approved: bool,
    /// Approvals that match the current VO content but not its current closure.
    pub invalid: Vec<(String, ApprovalInvalidity)>,
}

impl DerivedVoStatus {
    pub fn status(&self) -> &'static str {
        if self.approved {
            "approved"
        } else {
            "draft"
        }
    }
}

fn failure(kind: &'static str, id: &str, reason: impl Into<String>) -> ClosureFailure {
    ClosureFailure {
        kind,
        id: id.to_owned(),
        reason: reason.into(),
    }
}

/// Resolve the current upstream dependency closure of `vo`.
pub fn resolve_upstream_closure(
    layout: &VerifyLayout,
    vo: &VoRecord,
) -> Result<Vec<ApprovalDependency>, ClosureFailure> {
    let mut dependencies = BTreeMap::<(String, String), ContentHash>::new();
    let mut spec_refs = Vec::<SpecRef>::new();
    let mut requirements = Vec::<String>::new();

    let mut visited_vos = BTreeSet::from([vo.id.as_str().to_owned()]);
    spec_refs.extend(vo.spec_refs.iter().cloned());
    requirements.extend(vo.requirements.iter().map(|id| id.as_str().to_owned()));
    let mut parent = vo.parent.as_ref().map(|id| id.as_str().to_owned());
    while let Some(id) = parent {
        if !visited_vos.insert(id.clone()) {
            return Err(failure("vo", &id, "parent chain contains a cycle"));
        }
        let record =
            read_vo(layout, &id).map_err(|error| failure("vo", &id, unresolved(&error)))?;
        spec_refs.extend(record.spec_refs.iter().cloned());
        requirements.extend(record.requirements.iter().map(|id| id.as_str().to_owned()));
        parent = record.parent.as_ref().map(|id| id.as_str().to_owned());
        dependencies.insert(
            ("vo".to_owned(), id),
            hash_record_subject(&vo_projection(&record)),
        );
    }

    let mut visited_reqs = BTreeSet::new();
    while let Some(id) = requirements.pop() {
        if !visited_reqs.insert(id.clone()) {
            continue;
        }
        let record =
            read_req(layout, &id).map_err(|error| failure("req", &id, unresolved(&error)))?;
        spec_refs.extend(record.spec_refs.iter().cloned());
        if let Some(parent) = &record.parent {
            requirements.push(parent.as_str().to_owned());
        }
        dependencies.insert(
            ("req".to_owned(), id),
            hash_record_subject(&req_projection(&record)),
        );
    }

    let mut visited_specs = BTreeSet::new();
    for spec_ref in spec_refs {
        let id = spec_ref.spec.as_str().to_owned();
        if !visited_specs.insert(id.clone()) {
            continue;
        }
        let record =
            read_spec(layout, &id).map_err(|error| failure("spec", &id, unresolved(&error)))?;
        let source = read_text(&layout.root.join(&record.path))
            .map_err(|error| failure("spec", &id, unresolved(&error)))?;
        if hash_specification_source(&source) != record.sha256 {
            return Err(failure(
                "spec",
                &id,
                "recorded sha256 does not match the current Specification source",
            ));
        }
        dependencies.insert(
            ("spec".to_owned(), id),
            hash_spec_subject(&spec_projection(&record), &source),
        );
    }

    Ok(dependencies
        .into_iter()
        .map(|((kind, id), hash)| ApprovalDependency { kind, id, hash })
        .collect())
}

/// Aggregate an Approval subject: the VO's own record subject plus every
/// upstream dependency subject, in canonical closure order.
pub fn approval_subject_hash(
    subject: &ContentHash,
    dependencies: &[ApprovalDependency],
) -> ContentHash {
    hash_approval_subject(
        subject,
        dependencies.iter().map(|dependency| {
            (
                dependency.kind.as_str(),
                dependency.id.as_str(),
                &dependency.hash,
            )
        }),
    )
}

/// The Approval subject of `vo` as it stands now, or the reason its upstream
/// closure cannot be resolved.
pub fn current_approval_subject(
    layout: &VerifyLayout,
    vo: &VoRecord,
) -> Result<ContentHash, ClosureFailure> {
    let dependencies = resolve_upstream_closure(layout, vo)?;
    Ok(approval_subject_hash(&vo_record_subject(vo), &dependencies))
}

/// Derive the effective status of `vo` from the append-only Approvals.
///
/// `subject` is the current aggregate Approval subject. Approvals whose
/// recorded closure still equals the current one but whose aggregate differs
/// were superseded by an edit to the VO itself; that is the documented way an
/// approval lapses, so it is not reported as an invalid Approval.
pub fn derive_vo_status(
    layout: &VerifyLayout,
    vo: &VoRecord,
    subject: Result<&ContentHash, &ClosureFailure>,
) -> DerivedVoStatus {
    let mut closure = None;
    let mut invalid = Vec::new();
    for approval in read_approvals(layout) {
        if approval.subject != vo.id {
            continue;
        }
        let Some(recorded) = approval.dependencies.as_ref() else {
            invalid.push((approval.id, ApprovalInvalidity::ClosureAbsent));
            continue;
        };
        let subject = match subject {
            Err(error) => {
                invalid.push((
                    approval.id,
                    ApprovalInvalidity::ClosureUnresolvable(error.clone()),
                ));
                continue;
            }
            Ok(subject) => subject,
        };
        if &approval.subject_hash == subject {
            return DerivedVoStatus {
                approved: true,
                invalid: Vec::new(),
            };
        }
        let current = closure.get_or_insert_with(|| resolve_upstream_closure(layout, vo).ok());
        match current {
            Some(current) if recorded == current => {}
            _ => invalid.push((approval.id, ApprovalInvalidity::ClosureChanged)),
        }
    }
    DerivedVoStatus {
        approved: false,
        invalid,
    }
}

/// Read every Approval record in canonical file order.
pub fn read_approvals(layout: &VerifyLayout) -> Vec<ApprovalRecord> {
    let Ok(entries) = fs::read_dir(layout.approvals_dir()) else {
        return Vec::new();
    };
    let mut paths = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().and_then(|value| value.to_str()) == Some("yaml")
                && path
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .is_some_and(is_valid_ulid)
        })
        .collect::<Vec<_>>();
    paths.sort();
    paths
        .iter()
        .filter_map(|path| read_approval(path).ok())
        .collect()
}

/// The canonical VO record subject: the leaf content hash of the VO itself,
/// with the compatibility field `status` excluded.
pub fn vo_record_subject(vo: &VoRecord) -> ContentHash {
    hash_record_subject(&vo_projection(vo))
}

fn unresolved(error: &StoreError) -> String {
    error.to_string()
}

/// Canonical VO projection. The non-canonical compatibility field `status` is
/// derived from Approvals and is never part of the record subject.
fn vo_projection(record: &VoRecord) -> CanonicalProjection {
    projection_map([
        ("id", text(record.id.as_str())),
        (
            "parent",
            optional_text(record.parent.as_ref().map(|id| id.as_str())),
        ),
        (
            "requirements",
            CanonicalProjection::List(
                record
                    .requirements
                    .iter()
                    .map(|id| text(id.as_str()))
                    .collect(),
            ),
        ),
        ("spec_refs", spec_refs_projection(&record.spec_refs)),
        ("claim", text(&record.claim)),
        ("dimensions", dimensions_projection(&record.dimensions)),
        (
            "coverage_policy",
            optional_text(record.coverage_policy.as_deref()),
        ),
        (
            "combinations",
            CanonicalProjection::List(
                record
                    .combinations
                    .iter()
                    .map(|combination| {
                        CanonicalProjection::List(
                            combination.iter().map(|value| text(value)).collect(),
                        )
                    })
                    .collect(),
            ),
        ),
        (
            "representative_cases",
            CanonicalProjection::List(
                record
                    .representative_cases
                    .iter()
                    .map(|value| text(value))
                    .collect(),
            ),
        ),
        ("created", text(&record.created)),
        ("updated", text(&record.updated)),
    ])
}

fn req_projection(record: &ReqRecord) -> CanonicalProjection {
    projection_map([
        ("id", text(record.id.as_str())),
        (
            "parent",
            optional_text(record.parent.as_ref().map(|id| id.as_str())),
        ),
        ("spec_refs", spec_refs_projection(&record.spec_refs)),
        ("summary", text(&record.summary)),
        ("status", text(&record.status)),
        ("created", text(&record.created)),
        ("updated", text(&record.updated)),
    ])
}

fn spec_projection(record: &SpecRecord) -> CanonicalProjection {
    projection_map([
        ("id", text(record.id.as_str())),
        ("kind", text(&record.kind)),
        ("path", text(&record.path)),
        ("sha256", text(record.sha256.as_str())),
        ("title", optional_text(record.title.as_deref())),
        ("note", optional_text(record.note.as_deref())),
        ("registered_at", text(&record.registered_at)),
    ])
}

fn spec_refs_projection(refs: &[SpecRef]) -> CanonicalProjection {
    CanonicalProjection::List(
        refs.iter()
            .map(|spec_ref| {
                projection_map([
                    ("spec", text(spec_ref.spec.as_str())),
                    ("section", text(&spec_ref.section)),
                ])
            })
            .collect(),
    )
}

fn dimensions_projection(dimensions: &[Dimension]) -> CanonicalProjection {
    CanonicalProjection::List(
        dimensions
            .iter()
            .map(|dimension| {
                projection_map([
                    ("name", text(&dimension.name)),
                    (
                        "partitions",
                        CanonicalProjection::List(
                            dimension
                                .partitions
                                .iter()
                                .map(|value| text(value))
                                .collect(),
                        ),
                    ),
                ])
            })
            .collect(),
    )
}

fn projection_map<const N: usize>(
    entries: [(&'static str, CanonicalProjection); N],
) -> CanonicalProjection {
    CanonicalProjection::Map(
        entries
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    )
}

fn text(value: &str) -> CanonicalProjection {
    CanonicalProjection::String(value.to_owned())
}

fn optional_text(value: Option<&str>) -> CanonicalProjection {
    value.map_or(CanonicalProjection::Null, text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{new_record_id, Approver};
    use std::path::PathBuf;
    use vtest_model::VoId;

    fn bound_project(name: &str) -> (VerifyLayout, VoRecord) {
        let root = std::env::temp_dir().join(format!("vtest-approval-{name}-{}", new_record_id()));
        for directory in ["spec", "req", "vo", "approvals"] {
            fs::create_dir_all(root.join(".verify").join(directory)).unwrap();
        }
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::write(root.join("docs/contract.md"), "# Contract\n\nStable.\n").unwrap();
        let layout = VerifyLayout::new(&root);
        fs::write(
            layout.spec_dir().join("SPEC-ONE.yaml"),
            format!(
                "id: SPEC-ONE\nkind: design\npath: docs/contract.md\nsha256: {}\nregistered_at: '2026-01-01'\n",
                ContentHash::from_text("# Contract\n\nStable.\n")
            ),
        )
        .unwrap();
        fs::write(
            layout.req_dir().join("REQ-ONE.yaml"),
            "id: REQ-ONE\nparent: null\nspec_refs:\n  - spec: SPEC-ONE\n    section: contract\nsummary: contract\nstatus: active\ncreated: '2026-01-01'\nupdated: '2026-01-01'\n",
        )
        .unwrap();
        fs::write(
            layout.vo_dir().join("VO-ONE.yaml"),
            "id: VO-ONE\nparent: null\nrequirements: ['REQ-ONE']\nspec_refs:\n  - spec: SPEC-ONE\n    section: contract\nclaim: bound\ndimensions:\ncoverage_policy: null\ncombinations: []\nrepresentative_cases: []\ncreated: '2026-01-01'\nupdated: '2026-01-01'\n",
        )
        .unwrap();
        let vo = read_vo(&layout, "VO-ONE").unwrap();
        (layout, vo)
    }

    fn approve(
        layout: &VerifyLayout,
        vo: &VoRecord,
        dependencies: Option<Vec<ApprovalDependency>>,
    ) {
        let id = new_record_id();
        let record = ApprovalRecord {
            id: id.clone(),
            subject: VoId::new(vo.id.as_str()),
            subject_hash: approval_subject_hash(
                &vo_record_subject(vo),
                dependencies.as_deref().unwrap_or_default(),
            ),
            dependencies,
            approver: Approver {
                kind: "human".to_owned(),
                id: "reviewer".to_owned(),
                model: None,
            },
            basis: Vec::new(),
            approved_at: "2026-08-08T00:00:00Z".to_owned(),
        };
        fs::write(
            layout.approvals_dir().join(format!("{id}.yaml")),
            record.to_yaml(),
        )
        .unwrap();
    }

    fn cleanup(layout: &VerifyLayout) {
        let root: PathBuf = layout.root.clone();
        let _ = fs::remove_dir_all(root);
    }

    /// @vtest.id TEST-STORE-001
    /// @vtest.covers VO-STORE-001
    /// @vtest.target crates/vtest-store/src/approval.rs::resolve_upstream_closure
    /// @vtest.intent Closure spans parent REQs and their SPECs, excluding the subject VO itself
    #[test]
    fn closure_spans_requirements_and_their_specifications() {
        let (layout, vo) = bound_project("closure");
        let closure = resolve_upstream_closure(&layout, &vo).unwrap();
        let identities = closure
            .iter()
            .map(|dependency| (dependency.kind.as_str(), dependency.id.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(
            identities,
            vec![("req", "REQ-ONE"), ("spec", "SPEC-ONE")],
            "the subject VO is bound by subject_hash and never appears in its own closure"
        );
        cleanup(&layout);
    }

    /// @vtest.id TEST-STORE-002
    /// @vtest.covers VO-STORE-002
    /// @vtest.target crates/vtest-store/src/approval.rs::derive_vo_status
    /// @vtest.intent A changed Specification source makes the closure unresolvable and un-approves the VO
    #[test]
    fn specification_source_change_invalidates_the_recorded_closure() {
        let (layout, vo) = bound_project("spec-change");
        let closure = resolve_upstream_closure(&layout, &vo).unwrap();
        approve(&layout, &vo, Some(closure));
        let subject = current_approval_subject(&layout, &vo).unwrap();
        assert!(derive_vo_status(&layout, &vo, Ok(&subject)).approved);

        fs::write(
            layout.root.join("docs/contract.md"),
            "# Contract\n\nChanged.\n",
        )
        .unwrap();
        let stale = current_approval_subject(&layout, &vo)
            .expect_err("a moved Specification source is not resolvable");
        let derived = derive_vo_status(&layout, &vo, Err(&stale));
        assert!(!derived.approved);
        assert!(matches!(
            derived.invalid.first(),
            Some((_, ApprovalInvalidity::ClosureUnresolvable(_))),
        ));
        cleanup(&layout);
    }

    /// @vtest.id TEST-STORE-003
    /// @vtest.covers VO-STORE-002
    /// @vtest.target crates/vtest-store/src/approval.rs::derive_vo_status
    /// @vtest.intent Editing the subject VO lapses the approval without flagging it invalid
    #[test]
    fn editing_the_subject_vo_supersedes_the_approval_without_reporting_it() {
        let (layout, vo) = bound_project("subject-edit");
        let closure = resolve_upstream_closure(&layout, &vo).unwrap();
        approve(&layout, &vo, Some(closure.clone()));

        let mut edited = vo.clone();
        edited.claim = "the claim moved".to_owned();
        fs::write(layout.vo_dir().join("VO-ONE.yaml"), edited.to_yaml()).unwrap();
        let edited = read_vo(&layout, "VO-ONE").unwrap();
        assert_eq!(
            resolve_upstream_closure(&layout, &edited).unwrap(),
            closure,
            "editing the subject changes no upstream dependency"
        );
        let subject = current_approval_subject(&layout, &edited).unwrap();
        let derived = derive_vo_status(&layout, &edited, Ok(&subject));
        assert!(
            !derived.approved,
            "the aggregate subject binds the VO's own record subject"
        );
        assert!(
            derived.invalid.is_empty(),
            "a superseded approval is the documented lapse, not an invalid record: {:?}",
            derived.invalid
        );
        cleanup(&layout);
    }

    /// @vtest.id TEST-STORE-004
    /// @vtest.covers VO-STORE-002
    /// @vtest.target crates/vtest-store/src/approval.rs::derive_vo_status
    /// @vtest.intent A compatibility Approval with no closure is never approved (ClosureAbsent)
    #[test]
    fn compatibility_approval_without_a_closure_is_never_approved() {
        let (layout, vo) = bound_project("compatibility");
        approve(&layout, &vo, None);
        let subject = current_approval_subject(&layout, &vo).unwrap();
        let derived = derive_vo_status(&layout, &vo, Ok(&subject));
        assert!(!derived.approved);
        assert!(matches!(
            derived.invalid.first(),
            Some((_, ApprovalInvalidity::ClosureAbsent)),
        ));
        cleanup(&layout);
    }

    /// @vtest.id TEST-STORE-005
    /// @vtest.covers VO-STORE-002
    /// @vtest.target crates/vtest-store/src/approval.rs::derive_vo_status
    /// @vtest.intent A stale recorded closure is reported as ClosureChanged, not an approval
    #[test]
    fn a_stale_closure_is_reported_as_changed_not_as_an_approval() {
        let (layout, vo) = bound_project("stale-closure");
        approve(
            &layout,
            &vo,
            Some(vec![ApprovalDependency {
                kind: "req".to_owned(),
                id: "REQ-ONE".to_owned(),
                hash: ContentHash::from_text("outdated\n"),
            }]),
        );
        let subject = current_approval_subject(&layout, &vo).unwrap();
        let derived = derive_vo_status(&layout, &vo, Ok(&subject));
        assert!(!derived.approved);
        assert!(matches!(
            derived.invalid.first(),
            Some((_, ApprovalInvalidity::ClosureChanged)),
        ));
        cleanup(&layout);
    }

    /// @vtest.id TEST-STORE-006
    /// @vtest.covers VO-STORE-001
    /// @vtest.target crates/vtest-store/src/approval.rs::resolve_upstream_closure
    /// @vtest.intent The compatibility status field never alters any upstream closure subject
    #[test]
    fn the_compatibility_status_field_never_reaches_the_record_subject() {
        let (layout, vo) = bound_project("status-field");
        let canonical = resolve_upstream_closure(&layout, &vo).unwrap();
        let path = layout.vo_dir().join("VO-ONE.yaml");
        let text = read_text(&path).unwrap();
        assert!(!vo.to_yaml().contains("status:"));
        fs::write(
            &path,
            text.replace("claim: bound\n", "claim: bound\nstatus: approved\n"),
        )
        .unwrap();
        let compatibility = read_vo(&layout, "VO-ONE").unwrap();
        assert_eq!(compatibility.status.as_deref(), Some("approved"));
        assert_eq!(
            resolve_upstream_closure(&layout, &compatibility).unwrap(),
            canonical,
            "an ignored compatibility field cannot change any upstream subject"
        );
        cleanup(&layout);
    }
}
