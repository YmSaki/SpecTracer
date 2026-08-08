/// @vtest.id TEST-DUPLICATE
/// @vtest.covers VO-KNOWN
/// @vtest.target src/lib.rs::known
/// @vtest.intent first duplicate ID
#[test]
fn duplicate_first() {}

/// @vtest.id TEST-DUPLICATE
/// @vtest.covers VO-KNOWN
/// @vtest.target src/lib.rs::known
/// @vtest.intent second duplicate ID
#[test]
fn duplicate_second() {}

/// @vtest.id TEST-MISSING-VO
/// @vtest.covers VO-ABSENT
/// @vtest.target src/lib.rs::known
/// @vtest.intent dangling cover
#[test]
fn missing_vo() {}

/// @vtest.id TEST-MISSING-TARGET
/// @vtest.covers VO-KNOWN
/// @vtest.target src/lib.rs::absent
/// @vtest.intent unresolved target
#[test]
fn missing_target() {}

/// @vtest.id TEST-DUPLICATE-KEY
/// @vtest.id TEST-DUPLICATE-KEY-SECOND
/// @vtest.covers VO-KNOWN
/// @vtest.target src/lib.rs::known
/// @vtest.intent duplicate key
#[test]
fn duplicate_key() {}

/// @vtest.id TEST-UNKNOWN-KEY
/// @vtest.covers VO-KNOWN
/// @vtest.target src/lib.rs::known
/// @vtest.intent unknown key
/// @vtest.typo value
#[test]
fn unknown_key() {}

/// @vtest.id TEST-MISSING-INTENT
/// @vtest.covers VO-KNOWN
/// @vtest.target src/lib.rs::known
#[test]
fn missing_intent() {}
