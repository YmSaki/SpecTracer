use vtest_model::{
    AdapterId, ContentHash, ExecutionDescriptor, Locator, ProjectPath, SourceLocation, SourceRange,
    TestEntity, TestId,
};

use super::*;

const PATH: &str = "src/lib.rs";

/// DA-002 for the Test at item path `test` of `file` (held at `src/lib.rs`), one
/// verdict per target locator value.
fn verdicts(file: &str, test: &str, targets: &[&str]) -> Vec<(ReachOutcome, String)> {
    let model = FileModel::parse(file).expect("the fixture parses");
    let module = test_module(test);
    let name = test.rsplit("::").next().expect("a name");
    let info = model
        .fns
        .values()
        .find(|info| info.module == module && info.name == name)
        .expect("the fixture holds the Test");
    reach(Some(file), &file[info.start..info.end], test, targets)
}

/// DA-002 for the Test `construct` at item path `test` of a file whose text is
/// `file_text`, one verdict per target locator value.
fn reach(
    file_text: Option<&str>,
    construct: &str,
    test: &str,
    targets: &[&str],
) -> Vec<(ReachOutcome, String)> {
    let hash = ContentHash::from_text(construct);
    let entity = TestEntity {
        id: TestId::new("TEST-X"),
        covers: Vec::new(),
        targets: targets
            .iter()
            .map(|value| {
                TargetRef::Locator(Locator {
                    adapter: AdapterId::new("rust-cargo"),
                    value: (*value).to_owned(),
                })
            })
            .collect(),
        intent: "x".to_owned(),
        input: None,
        expect: None,
        kind: None,
        cases: Vec::new(),
        related: Vec::new(),
        location: SourceLocation {
            adapter: AdapterId::new("rust-cargo"),
            path: ProjectPath(PATH.to_owned()),
            locator: test.to_owned(),
            byte_range: SourceRange { start: 0, end: 1 },
        },
        content_hash: hash.clone(),
        execution: ExecutionDescriptor {
            adapter: AdapterId::new("rust-cargo"),
            project: None,
            suite: None,
            selector: "x".to_owned(),
        },
    };
    target_reach(StaticAnalysisInput {
        test: &entity,
        construct_text: construct,
        file_text,
        content_hash: &hash,
        extra_assertion_macros: &[],
    })
    .into_iter()
    .map(|reach| (reach.outcome, reach.reason))
    .collect()
}

/// The verdict for `src/lib.rs::double` of a file that defines `double` at its root
/// and holds `body` as the Test `tests::t` (with `use super::*;`) after `extra`.
fn double_verdict(extra: &str, body: &str) -> (ReachOutcome, String) {
    let file = format!(
        "pub fn double(x: i32) -> i32 {{ x * 2 }}\n\
         #[cfg(test)]\nmod tests {{\n    use super::*;\n    {extra}\n    #[test]\n    fn t() {{\n        {body}\n    }}\n}}\n"
    );
    verdicts(&file, "tests::t", &["src/lib.rs::double"]).remove(0)
}

fn assert_outcome(expected: ReachOutcome, (outcome, reason): (ReachOutcome, String), case: &str) {
    assert_eq!(outcome, expected, "{case}: {reason}");
}

/// @vtest.id TEST-ORACLE-DA-002-DIRECT-CALL-PASSES
/// @vtest.covers VO-ORACLE-DA-002-ANALYSIS-BOUNDARY, VO-ORACLE-DA-003-COUNTED-POSITIONS
/// @vtest.target crates/vtest-adapter-rust/src/target_reach.rs::target_reach
/// @vtest.intent Test 本体の数える位置にある同一ファイルの target 関数の呼出は、裸の名前、self/super の経路、モジュールの `use` と `as` の別名、本体の `use` のどれで書いても DA-002 を PASS にすることを確認する
#[test]
fn a_direct_call_of_a_same_file_target_passes() {
    for (extra, body) in [
        ("", "assert_eq!(double(2), 4);"),
        ("", "let r = super::double(2);\n        assert_eq!(r, 4);"),
        ("use super::double as twice;", "assert_eq!(twice(2), 4);"),
        (
            "",
            "use super::double as twice;\n        assert_eq!(twice(2), 4);",
        ),
        (
            "",
            "let v = vec![double(2)];\n        assert_eq!(v.len(), 1);",
        ),
        ("", "assert!(matches!(double(2), 4));"),
        ("", "if flag() {\n            double(2);\n        }"),
    ] {
        assert_outcome(ReachOutcome::Pass, double_verdict(extra, body), body);
    }
}

/// @vtest.id TEST-ORACLE-DA-002-HELPERS-FOLLOWED
/// @vtest.covers VO-ORACLE-DA-002-TRACKING-RANGE
/// @vtest.target crates/vtest-adapter-rust/src/target_reach.rs::target_reach
/// @vtest.intent 数える位置で呼ぶ同一ファイルの helper の本体を 1 段でも 2 段でも追跡して target 呼出を見つけ PASS にし、helper を辿っても呼出が無ければ FAIL にすることを確認する
#[test]
fn helpers_of_the_same_file_are_followed_at_any_depth() {
    let one = "fn helper() -> i32 { double(2) }";
    assert_outcome(
        ReachOutcome::Pass,
        double_verdict(one, "assert_eq!(helper(), 4);"),
        "one level",
    );
    let two = "fn outer() -> i32 { inner() }\n    fn inner() -> i32 { super::double(2) }";
    assert_outcome(
        ReachOutcome::Pass,
        double_verdict(two, "assert_eq!(outer(), 4);"),
        "two levels",
    );
    let none = "fn outer() -> i32 { inner() + 1 }\n    fn inner() -> i32 { 3 }";
    assert_outcome(
        ReachOutcome::Fail,
        double_verdict(none, "std::assert_eq!(outer(), 4);"),
        "no call",
    );
    let cycle = "fn ping(n: i32) -> i32 { if n == 0 { 0 } else { pong(n - 1) } }\n    fn pong(n: i32) -> i32 { ping(n) }";
    assert_outcome(
        ReachOutcome::Fail,
        double_verdict(cycle, "std::assert_eq!(ping(3), 0);"),
        "a cycle of helpers",
    );
}

/// @vtest.id TEST-ORACLE-DA-002-FAIL-ONLY-WITHOUT-UNFOLLOWED-CALLS
/// @vtest.covers VO-ORACLE-DA-002-FAIL-CONDITION
/// @vtest.target crates/vtest-adapter-rust/src/target_reach.rs::target_reach
/// @vtest.intent 探索範囲に target 呼出が無く追跡しない呼出も無い本体（標準マクロは std:: / core:: の経路で呼ぶ）だけを DA-002 FAIL にし、追跡しない関数呼出・メソッド呼出・値としての経路・構造体リテラル・Test の引数・書き換えうる属性・他ファイルの macro_rules! が取りうる裸の名前のマクロ呼出のどれかがあれば UNKNOWN にすることを確認する
#[test]
fn fail_needs_no_call_left_unfollowed() {
    for body in [
        "std::assert_eq!(1 + 1, 2);",
        "let x = 2;\n        std::assert_eq!(x * 2, 4, \"{}\", x);",
        "for i in 0..3 {\n            core::assert!(i < 3);\n        }",
    ] {
        assert_outcome(ReachOutcome::Fail, double_verdict("", body), body);
    }
    for body in [
        // A `macro_rules!` of another file may take a bare standard name.
        "assert_eq!(1 + 1, 2);",
        "assert_eq!(other(2), 4);",
        "assert_eq!(std::cmp::max(1, 2), 2);",
        "let v = 2;\n        assert_eq!(v.pow(2), 4);",
        "let f = double;\n        assert_eq!(f(2), 4);",
        "let p = P { x: 1 };\n        assert_eq!(p.x, 1);",
        "println!(\"{}\", 1);",
    ] {
        assert_outcome(ReachOutcome::Unknown, double_verdict("", body), body);
    }
    let file = "pub fn double(x: i32) -> i32 { x * 2 }\n\
                #[cfg(test)]\nmod tests {\n    #[tokio::test]\n    async fn t() { assert_eq!(1, 1); }\n    #[test]\n    fn p(x: i32) { assert_eq!(x, 1); }\n}\n";
    for test in ["tests::t", "tests::p"] {
        let (outcome, reason) = verdicts(file, test, &["src/lib.rs::double"]).remove(0);
        assert_eq!(outcome, ReachOutcome::Unknown, "{test}: {reason}");
    }
}

/// @vtest.id TEST-ORACLE-DA-002-BOUNDARY-IDENTIFIES-THE-CALL
/// @vtest.covers VO-ORACLE-DA-002-ANALYSIS-BOUNDARY
/// @vtest.target crates/vtest-adapter-rust/src/target_reach.rs::target_reach
/// @vtest.intent 同じ名前の別関数の呼出、他ファイルの target、受け手の型を解決しないメソッド呼出、trait メソッド、解決しない経路では PASS にせず、同一ファイルの固有メソッドを型の経路で呼ぶ形と、別関数の中で target を呼ぶ形だけを PASS にすることを確認する
#[test]
fn only_a_call_resolved_to_the_target_passes() {
    let same_name = "mod other {\n        pub fn double(x: i32) -> i32 { x + x }\n    }";
    assert_outcome(
        ReachOutcome::Fail,
        double_verdict(same_name, "std::assert_eq!(other::double(2), 4);"),
        "another function with the same name",
    );
    let calls_target =
        "mod other {\n        pub fn double(x: i32) -> i32 { super::super::double(x) }\n    }";
    assert_outcome(
        ReachOutcome::Pass,
        double_verdict(calls_target, "assert_eq!(other::double(2), 4);"),
        "another function that calls the target",
    );
    let shadow = "fn double(x: i32) -> i32 { x + x }";
    assert_outcome(
        ReachOutcome::Fail,
        double_verdict(shadow, "std::assert_eq!(double(2), 4);"),
        "a same-named function of the Test's module shadows the glob",
    );
    for (extra, body) in [
        ("", "assert_eq!(crate::double(2), 4);"),
        ("use crate::double as twice;", "assert_eq!(twice(2), 4);"),
        (
            "",
            "let double = |x: i32| x;\n        let g = double;\n        assert_eq!(g(2), 2);",
        ),
    ] {
        assert_outcome(ReachOutcome::Unknown, double_verdict(extra, body), body);
    }

    let file = "pub fn double(x: i32) -> i32 { x * 2 }\n\
                pub struct Calc;\n\
                impl Calc {\n    pub fn triple(&self, x: i32) -> i32 { x * 3 }\n}\n\
                pub trait Half { fn half(&self, x: i32) -> i32; }\n\
                impl Half for Calc {\n    fn half(&self, x: i32) -> i32 { x / 2 }\n}\n\
                #[cfg(test)]\nmod tests {\n    use super::*;\n\
                #[test]\n    fn path_call() { assert_eq!(Calc::triple(&Calc, 2), 6); }\n\
                #[test]\n    fn method_call() { let c = Calc; assert_eq!(c.triple(2), 6); }\n\
                #[test]\n    fn trait_call() { assert_eq!(Calc::half(&Calc, 4), 2); }\n\
                #[test]\n    fn direct() { assert_eq!(double(2), 4); }\n}\n";
    let check = |test: &str, target: &str, expected: ReachOutcome| {
        let (outcome, reason) = verdicts(file, test, &[target]).remove(0);
        assert_eq!(outcome, expected, "{test} / {target}: {reason}");
    };
    check(
        "tests::path_call",
        "src/lib.rs::Calc::triple",
        ReachOutcome::Pass,
    );
    check(
        "tests::method_call",
        "src/lib.rs::Calc::triple",
        ReachOutcome::Unknown,
    );
    check(
        "tests::trait_call",
        "src/lib.rs::Calc::half",
        ReachOutcome::Unknown,
    );
    check(
        "tests::direct",
        "src/other.rs::double",
        ReachOutcome::Unknown,
    );
}

/// @vtest.id TEST-ORACLE-DA-002-COUNTED-POSITIONS
/// @vtest.covers VO-ORACLE-DA-003-COUNTED-POSITIONS
/// @vtest.target crates/vtest-adapter-rust/src/target_reach.rs::target_reach
/// @vtest.intent 数える位置に無い target 呼出（assert のメッセージ引数、async ブロック、入れ子の項目、数えない位置から呼ぶ helper の本体、async 関数の本体、cfg 属性の下）だけがある場合に、DA-002 が PASS でも FAIL でもなく UNKNOWN になることを確認する
#[test]
fn calls_only_at_uncounted_positions_are_unknown() {
    let helper = "fn helper() -> i32 { double(2) }";
    for (extra, body, why) in [
        ("", "assert!(true, \"{}\", double(2));", "DS-1962"),
        ("", "let _f = async { double(2) };", "DS-1962"),
        (
            "",
            "fn inner() -> i32 { super::double(2) }\n        let _ = 1;",
            "DS-1962",
        ),
        (helper, "let _f = async { helper() };", "DS-1962"),
        (
            "async fn later() -> i32 { double(2) }",
            "let _f = later();",
            "DS-1962",
        ),
        (
            "",
            "#[cfg(feature = \"x\")]\n        let _ = double(2);",
            "DS-1962",
        ),
    ] {
        let (outcome, reason) = double_verdict(extra, body);
        assert_eq!(outcome, ReachOutcome::Unknown, "{body}: {reason}");
        assert!(reason.contains(why), "{body}: {reason}");
    }
}

/// @vtest.id TEST-ORACLE-DA-002-NEVER-RUN-CALL-UNKNOWN
/// @vtest.covers VO-ORACLE-DA-002-ANALYSIS-BOUNDARY
/// @vtest.target crates/vtest-adapter-rust/src/target_reach.rs::target_reach
/// @vtest.intent 構文上実行されないと分かる位置（定数で偽の if、return の後、定数で決まる && / || の評価されない右辺、必ず失敗する定数の assert_eq! / assert_ne! の後。それを含むブロックの後も含む）にだけ target 呼出があるとき、その呼出は静的な到達の証明にならないので PASS にせず、呼出があるので FAIL にもせず、DA-002 を UNKNOWN にすることを確認する
#[test]
fn a_call_syntax_shows_never_runs_is_unknown() {
    let helper = "fn helper() -> i32 { double(2) }";
    for (extra, body) in [
        ("", "if false {\n            double(2);\n        }"),
        (
            "",
            "return;\n        #[allow(unreachable_code)]\n        double(2);",
        ),
        (helper, "if 1 > 2 {\n            helper();\n        }"),
        ("", "let _ = false && double(2) == 4;"),
        ("", "let _ = true || double(2) == 4;"),
        ("", "assert_eq!(1, 2, \"boom\");\n        double(2);"),
        ("", "std::assert_ne!(1, 1);\n        double(2);"),
        (
            "",
            "{\n            assert_eq!(1, 2);\n        }\n        double(2);",
        ),
        (
            "",
            "if true {\n            assert_eq!(1, 2);\n        }\n        double(2);",
        ),
        (
            "",
            "let _ = {\n            assert_ne!(1, 1);\n        };\n        double(2);",
        ),
    ] {
        let (outcome, reason) = double_verdict(extra, body);
        assert_eq!(outcome, ReachOutcome::Unknown, "{body}: {reason}");
        assert!(reason.contains("never to run"), "{body}: {reason}");
    }
    // An operand or a comparison syntax does not fix still runs.
    for body in [
        "let _ = true && double(2) == 4;",
        "let _ = false || double(2) == 4;",
        "std::assert_eq!(1, 1);\n        double(2);",
        "let x = 1;\n        std::assert_eq!(x, 2);\n        double(2);",
    ] {
        assert_outcome(ReachOutcome::Pass, double_verdict("", body), body);
    }
}

/// @vtest.id TEST-ORACLE-DA-002-EXPANDED-NAMES-UNRESOLVED
/// @vtest.covers VO-ORACLE-DA-002-ANALYSIS-BOUNDARY
/// @vtest.target crates/vtest-adapter-rust/src/target_reach.rs::target_reach
/// @vtest.intent モジュールや Test 本体に、展開で項目を足しうる属性（属性マクロ、標準以外の derive）の付いた項目があるとき、glob import で解決する名前を解決せず、その呼出で PASS にも FAIL にもしないこと、標準の derive と明示の項目はこれまでどおり解決することを確認する
#[test]
fn names_an_expansion_may_bind_are_not_resolved() {
    for (extra, body) in [
        ("#[pm::emit_double]\n    struct Marker;", "double(2);"),
        (
            "#[derive(pm::EmitDouble)]\n    struct Marker;",
            "double(2);",
        ),
        (
            "",
            "#[pm::emit_double]\n        struct Marker;\n        double(2);",
        ),
        (
            "#[pm::emit_helper]\n    struct Marker;",
            "let _ = helper();",
        ),
    ] {
        assert_outcome(ReachOutcome::Unknown, double_verdict(extra, body), extra);
    }
    for (extra, body) in [
        (
            "#[derive(Debug, Clone)]\n    #[repr(C)]\n    struct Marker;",
            "double(2);",
        ),
        (
            "#[pm::emit_double]\n    struct Marker;",
            "super::double(2);",
        ),
    ] {
        assert_outcome(ReachOutcome::Pass, double_verdict(extra, body), body);
    }
}

/// @vtest.id TEST-ORACLE-DA-002-TRACKED-CLOSURE-PASSES
/// @vtest.covers VO-ORACLE-DA-003-CLOSURE-CALL-TRACKED
/// @vtest.target crates/vtest-adapter-rust/src/target_reach.rs::target_reach
/// @vtest.intent let で束縛したクロージャを数える位置で直接呼ぶとき、その本体の target 呼出を DA-002 の呼出として数えて PASS にすることを確認する
#[test]
fn a_tracked_closure_call_counts_its_body() {
    assert_outcome(
        ReachOutcome::Pass,
        double_verdict(
            "",
            "let f = |x: i32| double(x);\n        assert_eq!(f(2), 4);",
        ),
        "tracked closure",
    );
}

/// @vtest.id TEST-ORACLE-DA-002-UNTRACKED-CLOSURE-UNKNOWN
/// @vtest.covers VO-ORACLE-DA-003-UNTRACKED-CLOSURE-UNKNOWN
/// @vtest.target crates/vtest-adapter-rust/src/target_reach.rs::target_reach
/// @vtest.intent 追跡しないクロージャ（関数へ渡すもの、一度も呼ばないもの）の本体に target 呼出があるとき、DA-002 を FAIL ではなく UNKNOWN にすることを確認する
#[test]
fn a_target_call_in_an_untracked_closure_is_unknown() {
    for body in [
        "let v: Vec<i32> = [1].iter().map(|x| double(*x)).collect();\n        assert_eq!(v, [2]);",
        "let _f = |x: i32| double(x);",
    ] {
        let (outcome, reason) = double_verdict("", body);
        assert_eq!(outcome, ReachOutcome::Unknown, "{body}: {reason}");
        assert!(reason.contains("DS-1944"), "{body}: {reason}");
    }
}

/// @vtest.id TEST-ORACLE-DA-002-UNREAD-MACRO-UNKNOWN
/// @vtest.covers VO-ORACLE-DA-002-UNTRACKED-MACRO-UNKNOWN
/// @vtest.target crates/vtest-adapter-rust/src/target_reach.rs::target_reach
/// @vtest.intent 展開を読まないマクロの中の target 呼出は DA-002 を UNKNOWN にし、展開を読むマクロ（assert_eq! の比較引数）の中の呼出は PASS にすることを確認する
#[test]
fn a_call_in_a_macro_not_read_is_unknown() {
    assert_outcome(
        ReachOutcome::Unknown,
        double_verdict("", "println!(\"{}\", double(2));"),
        "println!",
    );
    assert_outcome(
        ReachOutcome::Unknown,
        double_verdict("", "debug_assert_eq!(double(2), 4);"),
        "debug_assert_eq!",
    );
    assert_outcome(
        ReachOutcome::Pass,
        double_verdict("", "assert_eq!(double(2), 4);"),
        "assert_eq!",
    );
}

/// @vtest.id TEST-ORACLE-DA-002-SHADOWED-STANDARD-MACRO-UNKNOWN
/// @vtest.covers VO-ORACLE-DA-002-UNTRACKED-MACRO-UNKNOWN
/// @vtest.target crates/vtest-adapter-rust/src/target_reach.rs::target_reach
/// @vtest.intent ファイルの macro_rules!、名前の use、ファイル外からの glob import、項目を足しうる属性（属性マクロ、標準以外の derive）の付いた項目が assert_eq! などの標準の名前を取りうるとき、およびファイルが構文解析できないときは、そのマクロの展開を読まず、DA-002 を PASS にも FAIL にもせず UNKNOWN にすること、裸の名前の標準マクロは他ファイルの macro_rules! でありうるので呼出のない本体でも FAIL にしないことを確認する
#[test]
fn a_macro_that_may_take_a_standard_name_is_not_read() {
    for (extra, body) in [
        (
            "macro_rules! assert_eq { ($a:expr, $b:expr) => {}; }",
            "assert_eq!(double(2), 4);",
        ),
        (
            "macro_rules! assert { ($e:expr) => { super::double(1); }; }",
            "assert!(true);",
        ),
        ("use other::assert_eq;", "assert_eq!(double(2), 4);"),
        ("use other::*;", "assert_eq!(double(2), 4);"),
        (
            "#[macro_use]\n    extern crate other;",
            "assert_eq!(double(2), 4);",
        ),
        ("#[macro_use]\n    mod macros;", "assert_eq!(double(2), 4);"),
        (
            "",
            "macro_rules! assert_eq { ($a:expr, $b:expr) => {}; }\n        assert_eq!(double(2), 4);",
        ),
        ("use other::test;", "double(2);"),
    ] {
        assert_outcome(ReachOutcome::Unknown, double_verdict(extra, body), extra);
    }
    assert_outcome(
        ReachOutcome::Fail,
        double_verdict("", "std::assert!(true);"),
        "the standard assert!",
    );
    // A `macro_rules!` of another file may take a bare standard name and call the
    // target, so a bare one is not grounds for `Fail`; its arguments still give `Pass`.
    assert_outcome(
        ReachOutcome::Unknown,
        double_verdict("", "assert_eq!(1, 1);"),
        "a bare assert_eq! without a call",
    );
    // An item an attribute macro or a non-standard derive adds may be a macro.
    for extra in [
        "#[pm::emit]\n    struct Marker;",
        "#[derive(pm::Emit)]\n    struct Marker;",
    ] {
        assert_outcome(
            ReachOutcome::Unknown,
            double_verdict(extra, "std::assert_eq!(double(2), 4);"),
            extra,
        );
    }
    // A file that does not parse may define any macro.
    let construct = "fn t() { std::assert!(true); }";
    let (outcome, reason) =
        reach(Some("fn t() {"), construct, "t", &["src/lib.rs::double"]).remove(0);
    assert_eq!(outcome, ReachOutcome::Unknown, "an unparsed file: {reason}");
    let (outcome, reason) = reach(None, construct, "t", &["src/lib.rs::double"]).remove(0);
    assert_eq!(outcome, ReachOutcome::Unknown, "no file text: {reason}");
    // A glob of a module of the file imports nothing this scan does not see.
    assert_outcome(
        ReachOutcome::Pass,
        double_verdict(
            "mod inner { pub fn unrelated() {} }\n    use inner::*;",
            "assert_eq!(double(2), 4);",
        ),
        "a glob of an inline module",
    );
}

/// @vtest.id TEST-ORACLE-DA-002-UNFIXED-CODE-NOT-COUNTED
/// @vtest.covers VO-ORACLE-DA-002-ANALYSIS-BOUNDARY
/// @vtest.target crates/vtest-adapter-rust/src/target_reach.rs::target_reach
/// @vtest.intent 属性マクロが書き換えうる関数、型引数が隠す型名、コンパイル時に評価する式の中の target 呼出は PASS にせず、format 文字列が捕捉するローカル以外の名前は呼出を残したものとして FAIL にしないことを確認する
#[test]
fn code_the_boundary_does_not_fix_is_not_counted() {
    for (extra, body) in [
        (
            "",
            "let a = [0u8; double(2) as usize];\n        assert_eq!(a.len(), 4);",
        ),
        (
            "",
            "let x = const { double(2) };\n        assert_eq!(x, 4);",
        ),
        ("", "assert!(1 == 1, \"{w}\");"),
        // A static of a nested block, captured where a closure parameter elsewhere
        // shares its name.
        (
            "",
            "let _f = |g: i32| g;\n        {\n            static g: W = W;\n            std::panic!(\"{g}\");\n        }",
        ),
        ("", "let _a: [u8; double(2) as usize] = [0; 4];"),
        ("fn h<const N: i32>() {}", "h::<{ double(2) }>();"),
        (
            "#[my::strip]\n    fn helper() -> i32 { double(2) }",
            "assert_eq!(helper(), 4);",
        ),
        (
            "struct S;\n    #[my::strip]\n    impl S { fn helper() -> i32 { double(2) } }",
            "assert_eq!(S::helper(), 4);",
        ),
    ] {
        assert_outcome(ReachOutcome::Unknown, double_verdict(extra, body), body);
    }
    for (extra, body) in [
        ("", "let a = [double(2); 3];\n        assert_eq!(a[0], 4);"),
        (
            "#[inline]\n    fn helper() -> i32 { double(2) }",
            "assert_eq!(helper(), 4);",
        ),
        (
            "#[rustfmt::skip]\n    fn helper() -> i32 { double(2) }",
            "assert_eq!(helper(), 4);",
        ),
    ] {
        assert_outcome(ReachOutcome::Pass, double_verdict(extra, body), extra);
    }
    for (extra, body) in [
        ("", "let w = 1;\n        std::assert!(1 == 1, \"{w}\");"),
        ("", "std::assert!(1 == 1, \"{{w}}\");"),
        // A const parameter shadows the constant `D`, and its value runs nothing.
        (
            "const D: S = S;\n    struct S;\n    fn helper<const D: i32>() -> i32 { D }",
            "std::assert_eq!(helper::<2>(), 2);",
        ),
    ] {
        assert_outcome(ReachOutcome::Fail, double_verdict(extra, body), body);
    }
    let attributed = "pub fn double(x: i32) -> i32 { x * 2 }\n\
                      #[cfg(test)]\nmod tests {\n    use super::*;\n    #[my::strip]\n    #[test]\n    fn t() { double(2); }\n}\n";
    let module = "pub fn double(x: i32) -> i32 { x * 2 }\n\
                  #[my::strip]\nmod tests {\n    use super::*;\n    #[test]\n    fn t() { double(2); }\n}\n";
    for (file, case) in [(attributed, "on the Test"), (module, "on its module")] {
        let (outcome, reason) = verdicts(file, "tests::t", &["src/lib.rs::double"]).remove(0);
        assert_eq!(outcome, ReachOutcome::Unknown, "{case}: {reason}");
    }
    let plain_cfg_attr = "pub fn double(x: i32) -> i32 { x * 2 }\n\
                          #[cfg(test)]\nmod tests {\n    use super::*;\n    #[cfg_attr(miri, ignore)]\n    #[test]\n    fn t() { double(2); }\n}\n";
    assert_outcome(
        ReachOutcome::Pass,
        verdicts(plain_cfg_attr, "tests::t", &["src/lib.rs::double"]).remove(0),
        "a cfg_attr that applies a plain attribute",
    );
    let generic = |helper: &str| {
        format!(
            "pub struct Calc;\nimpl Calc {{ pub fn make() -> i32 {{ 1 }} }}\n\
             pub trait Mk {{ fn make() -> i32; }}\npub struct O;\n\
             impl Mk for O {{ fn make() -> i32 {{ 2 }} }}\n{helper}\n\
             #[test]\nfn t() {{ assert_eq!(helper::<O>(), 2); }}\n"
        )
    };
    let shadowed = generic("fn helper<Calc: Mk>() -> i32 { Calc::make() }");
    let plain = generic("fn helper<T: Mk>() -> i32 { Calc::make() }");
    let outcome = |file: &str| verdicts(file, "t", &["src/lib.rs::Calc::make"]).remove(0);
    assert_outcome(
        ReachOutcome::Unknown,
        outcome(&shadowed),
        "a type parameter `Calc`",
    );
    assert_outcome(ReachOutcome::Pass, outcome(&plain), "the type `Calc`");
}

/// @vtest.id TEST-ORACLE-DA-002-EACH-TARGET-ON-ITS-OWN
/// @vtest.covers VO-ORACLE-DA-002-PER-TARGET
/// @vtest.target crates/vtest-adapter-rust/src/target_reach.rs::target_reach
/// @vtest.intent 複数 target の Test で、呼んでいる target は PASS、呼んでいない target は FAIL と、target ごとに宣言順で別々に判定することを確認する
#[test]
fn each_target_is_decided_on_its_own() {
    let file = "pub fn double(x: i32) -> i32 { x * 2 }\n\
                pub fn triple(x: i32) -> i32 { x * 3 }\n\
                #[cfg(test)]\nmod tests {\n    use super::*;\n    #[test]\n    fn t() { std::assert_eq!(double(2), 4); }\n}\n";
    let outcomes: Vec<ReachOutcome> = verdicts(
        file,
        "tests::t",
        &["src/lib.rs::triple", "src/lib.rs::double"],
    )
    .into_iter()
    .map(|(outcome, _)| outcome)
    .collect();
    assert_eq!(outcomes, [ReachOutcome::Fail, ReachOutcome::Pass]);
}
