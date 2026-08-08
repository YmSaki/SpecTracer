use calc_fixture::{add, evaluate};

/// @vtest.id TEST-CALC-ADD
/// @vtest.covers VO-CALC-ADD
/// @vtest.target src/lib.rs::add
/// @vtest.intent adds two integers
/// @vtest.input 2 and 3
/// @vtest.expect 5
#[test]
fn adds_two_integers() {
    assert_eq!(add(2, 3), 5);
}

/// @vtest.id TEST-CALC-TABLE
/// @vtest.covers VO-CALC-ADD
/// @vtest.target src/lib.rs::add
/// @vtest.intent evaluates a table of additions
/// @vtest.case 1+1=2
/// @vtest.case 2+3=5
#[test]
fn table_driven_additions() {
    for (left, right, expected) in [(1, 1, 2), (2, 3, 5)] {
        assert_eq!(add(left, right), expected);
    }
}

/// @vtest.id TEST-CALC-ASSERT-TRUE
/// @vtest.covers VO-CALC-ADD
/// @vtest.target src/lib.rs::add
/// @vtest.intent intentionally weak assertion for DA-001
#[test]
fn assert_true_only() {
    let _ = add(1, 1);
    assert!(true);
}

/// @vtest.id TEST-CALC-NO-CALL
/// @vtest.covers VO-CALC-ADD
/// @vtest.target src/lib.rs::add
/// @vtest.intent intentionally does not call target for DA-002
#[test]
fn target_not_called() {
    assert_eq!(1, 1);
}

/// @vtest.id TEST-CALC-NO-ASSERT
/// @vtest.covers VO-CALC-ADD
/// @vtest.target src/lib.rs::add
/// @vtest.intent intentionally has no assertion for DA-003
#[test]
fn no_result_assertion() {
    let _ = add(1, 2);
}

/// @vtest.id TEST-CALC-SELF-COMPARE
/// @vtest.covers VO-CALC-ADD
/// @vtest.target src/lib.rs::add
/// @vtest.intent intentionally self-compares for DA-004
#[test]
fn self_compare() {
    let actual = add(1, 2);
    assert_eq!(actual, actual);
}

/// @vtest.id TEST-CALC-DANGLING
/// @vtest.covers VO-CALC-MISSING
/// @vtest.target src/lib.rs::evaluate
/// @vtest.intent references a missing VO for E-SCAN-003
#[test]
fn dangling_vo_reference() {
    assert!(evaluate(1, 1, '+').is_ok());
}

#[test]
fn unregistered_test() {
    assert_eq!(add(0, 0), 0);
}
