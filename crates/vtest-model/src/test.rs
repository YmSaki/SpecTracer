use crate::{AdapterId, ContentHash, SourceLocation, TargetRef, TestId, VoId};
use serde::{Deserialize, Serialize};

/// Adapter-neutral, opaque execution coordinate for a test (詳細設計 v0.1
/// 本冊:644-649、§5.2、逐語):
///
/// ```text
/// pub struct ExecutionDescriptor {
///     pub adapter: AdapterId,
///     pub project: Option<String>,
///     pub suite: Option<TestSuite>,
///     pub selector: String,
/// }
/// ```
///
/// 本冊:688「coreは `project`、`suite.kind`、`suite.name`、`selector` の
/// 文字列を解釈しない」— this struct's fields are opaque strings from
/// `vtest-model`/`vtest-scan`'s point of view. Only the adapter named by
/// `adapter` (via its `TestRunnerAdapter`, 本冊 §9.2) interprets them into
/// an actual execution coordinate; `rust-cargo`'s interpretation is 本冊
/// §9.2's `project` = cargo package名 / `suite.kind` ∈ `lib`/`bin`/
/// `integration` / `suite.name` = bin または integration test target名 /
/// `selector` = module path + function名.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExecutionDescriptor {
    pub adapter: AdapterId,
    pub project: Option<String>,
    pub suite: Option<TestSuite>,
    pub selector: String,
}

/// `ExecutionDescriptor.suite`'s referent (詳細設計 v0.1 本冊:651-654、
/// §5.2、逐語):
///
/// ```text
/// pub struct TestSuite {
///     pub kind: String,
///     pub name: Option<String>,
/// }
/// ```
///
/// `kind` is a plain `String`, not an enum — 本冊:653 defines no domain
/// constraint on it. The three values `rust-cargo` assigns at interpretation
/// time (本冊:1309 "`suite.kind`：`lib` / `bin` / `integration`") are that
/// adapter's own convention, not a type-level enum this crate enforces (see
/// `hash27-model-spec.md` §3, confirmed against 本冊 §9.2). Do not turn this
/// back into an enum on `vtest-model`'s side.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TestSuite {
    pub kind: String,
    pub name: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TestEntity {
    pub id: TestId,
    pub covers: Vec<VoId>,
    /// 基本仕様:146「1 つの Test は 1 件以上の Source Target を持ち、各
    /// target 参照を個別に保持する。代表 1 件へ縮約しない」。本冊:620 も
    /// `pub targets: Vec<TargetRef>` という単一 field を定める。以前の
    /// `target` + `additional_targets` という2 field 形状は代表 1 件を
    /// 前面に押し出す構造であり、この規範と整合しなかった。
    pub targets: Vec<TargetRef>,
    pub intent: String,
    pub input: Option<String>,
    pub expect: Option<String>,
    pub kind: Option<String>,
    pub cases: Vec<String>,
    pub related: Vec<TestId>,
    pub location: SourceLocation,
    pub content_hash: ContentHash,
    /// 本冊:617-630 の逐語形状。以前の `filter` / `package` / `test_target`
    /// （Rust/Cargo 固有 field）と `TestTarget` 型は、本冊:685-703
    /// 「`filter`、`package`、`test_target`および`TestTarget`型を
    /// `vtest-model`へ置かない」により、この crate から除去した。
    /// `TestTarget` は `rust-cargo` adapter 内部の分類 (`vtest-adapter-
    /// rust::TestTarget`) へ移り、`rust-cargo` adapter がそれを
    /// `ExecutionDescriptor` へ解釈してから core（`vtest-scan`）へ渡す
    /// （本冊 §9.2「`rust-cargo` adapterは`TestEntity.execution`を次の
    /// Cargo実行座標として解釈する」）。
    pub execution: ExecutionDescriptor,
}

/// 1 件の discovered Test construct（本冊:788-801 の `DiscoveredTest`）が、
/// core materialization 後に管理対象 Test へどう対応したかを表す。
///
/// variant は本冊が定める3つのみ（本冊:796-800、`pr3-ruling-spec.md` §1.1）:
/// - `Missing`：管理宣言または必須 metadata の欠落（本冊:805）。
/// - `One(TestId)`：構文上完全な Test Entity へ正規化できた場合に設定する。
///   `covers` の VO 参照が解決できない場合でも、この対応する entity は
///   保持され `One(id)` のままである（本冊:804「解決不能なcoversを持つ
///   draftもcore materialization後のmanaged entity集合に保持され、対応する
///   observationはManagedTestLink::One(id)を持つ」、本冊:569）。
/// - `Multiple(Vec<TestId>)`：**同一 construct から複数 draft が生じた**
///   場合（本冊:805「Multipleは同一Test constructから複数draftが生じる
///   状態を表す」）。
///
/// 注意（読み違えないこと）: この `Multiple` は Test ID の**大域的衝突**
/// （異なる construct が同じ Test ID を宣言する状態）を表す variant では
/// ない。それは基本:412（基本仕様§12）が言う「`M` は…Test ID が衝突する
/// entity も含む」という**別の**整合性条件であり、衝突した各 construct は
/// それぞれ個別に自分自身の `One(自分のid)` を持つ（本冊:804 と同じ理由。
/// `ManagedTestLink` は construct 単位の対応数を表し、Test ID の一意性は
/// 表さない）。この区別の根拠は `chain_integrity` が両者を独立した違反
/// として列挙している点にもある（本冊:898「`ManagedTestLink::Multiple`、
/// E-SCAN-002（Test ID衝突）、E-SCAN-003（解決不能なVO参照） →
/// `chain_integrity = MISMATCH`」— 3つが並列に列挙されており、同一現象なら
/// 並列に書かれない）。`rust-cargo` adapter は 1 関数 item から高々 1 件の
/// draft しか生成しないため、`Multiple` は現状この adapter からは到達
/// 不能である（将来 adapter のための語彙）。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ManagedTestLink {
    Missing,
    One(TestId),
    Multiple(Vec<TestId>),
}

/// core materialization 後の、1 件の discovered Test construct（本冊:
/// 788-801、逐語通りの4 field）。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiscoveredTest {
    pub adapter: AdapterId,
    pub location: SourceLocation,
    pub content_hash: ContentHash,
    pub managed: ManagedTestLink,
}

/// Canonical, adapter-neutral metadata record for a test.
///
/// The fields are the normalized logical fields a source discovery adapter
/// produces for a test. Cardinality requirements on `targets` belong to the
/// adapter, not to this record.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TestRecord {
    pub id: TestId,
    pub covers: Vec<VoId>,
    pub targets: Vec<TargetRef>,
    pub intent: String,
    pub input: Option<String>,
    pub expect: Option<String>,
    pub kind: Option<String>,
    pub cases: Vec<String>,
    pub related: Vec<TestId>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AdapterId, Locator};

    fn populated_record() -> TestRecord {
        TestRecord {
            id: TestId::new("TEST-001"),
            covers: vec![VoId::new("VO-001")],
            targets: vec![TargetRef::Locator(Locator {
                adapter: AdapterId::new("rust-cargo"),
                value: "src/lib.rs::module::function".to_string(),
            })],
            intent: "The parser rejects an unknown key.".to_string(),
            input: Some("an unknown key".to_string()),
            expect: Some("E-SCAN-006".to_string()),
            kind: Some("unit-normal".to_string()),
            cases: vec!["unknown key".to_string()],
            related: vec![TestId::new("TEST-002")],
        }
    }

    #[test]
    fn test_record_carries_the_normalized_logical_fields() {
        let value = serde_json::to_value(populated_record()).unwrap();
        let mut keys: Vec<&String> = value.as_object().unwrap().keys().collect();
        keys.sort();

        let mut expected = vec![
            "id", "covers", "targets", "intent", "input", "expect", "kind", "cases", "related",
        ];
        expected.sort();

        assert_eq!(keys, expected);
    }

    #[test]
    fn test_record_round_trips_without_the_optional_fields() {
        let record = TestRecord {
            input: None,
            expect: None,
            kind: None,
            ..populated_record()
        };

        let json = serde_json::to_string(&record).unwrap();
        assert_eq!(serde_json::from_str::<TestRecord>(&json).unwrap(), record);
    }

    #[test]
    fn test_record_round_trips_with_empty_lists() {
        let record = TestRecord {
            covers: vec![],
            targets: vec![],
            cases: vec![],
            related: vec![],
            ..populated_record()
        };

        let json = serde_json::to_string(&record).unwrap();
        assert_eq!(serde_json::from_str::<TestRecord>(&json).unwrap(), record);
    }
}
