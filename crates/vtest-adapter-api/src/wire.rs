//! Test JSON wire compatibility layer (BD-193/BD-197, DS-967/968/971/974,
//! DS-1574/1577/1578/1579/1674, DES-018〜022, DES-282〜288).
//!
//! The core `TestEntity` carries only the language-neutral `execution`
//! descriptor (DES-561). Version-1 compatibility fields such as
//! `filter` / `package` / `test_target` are owned by an adapter's
//! [`TestWireCodec`] and exist only on the wire: [`encode_test_json`] adds
//! them for Tests of an adapter that has a codec, and [`decode_test_json`]
//! strips them again, checking that they agree with `execution` or, when
//! `execution` is absent, deriving it from them.

use serde_json::{Map, Value};
use vtest_model::{ExecutionDescriptor, TargetRef, TestEntity};

use crate::AdapterRegistry;

/// A JSON object of wire-only fields.
pub type WireFields = Map<String, Value>;

/// BD-193: an adapter's codec for its own version-1 compatibility fields.
pub trait TestWireCodec {
    fn id(&self) -> &'static str;

    /// Every compatibility field name this codec owns.
    fn field_names(&self) -> &'static [&'static str];

    /// DS-967/1574: the compatibility fields for a Test whose execution
    /// descriptor is `execution`. Returns no field it cannot derive
    /// completely (DS-968: no empty or dummy values).
    fn encode(&self, execution: &ExecutionDescriptor) -> WireFields;

    /// DS-1578: the descriptor that complete, mutually consistent
    /// compatibility fields describe. `fields` holds only this codec's own
    /// field names. Incomplete or inconsistent fields are an error.
    fn decode(&self, fields: &WireFields) -> Result<ExecutionDescriptor, String>;
}

/// A Test JSON input the wire layer rejects.
#[derive(Debug, Eq, PartialEq)]
pub struct WireError(pub String);

impl std::fmt::Display for WireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for WireError {}

/// DES-018/282/288, DS-1574/1674: writes `execution` and `targets` always,
/// the singular `target` only when there is exactly one target, and the
/// compatibility fields only when the Test's adapter has a codec.
pub fn encode_test_json(test: &TestEntity, registry: &AdapterRegistry) -> Value {
    let mut object = match serde_json::to_value(test) {
        Ok(Value::Object(object)) => object,
        _ => unreachable!("TestEntity serializes to a JSON object"),
    };
    if let [single] = test.targets.as_slice() {
        object.insert("target".to_owned(), target_value(single));
    }
    if let Some(codec) = registry.test_wire_codec(test.execution.adapter.as_str()) {
        for (key, value) in codec.encode(&test.execution) {
            object.insert(key, value);
        }
    }
    Value::Object(object)
}

/// DES-284/285, DS-974/1578/1579: reads a Test JSON object.
///
/// - `execution` wins when present, and any compatibility fields must
///   describe the same descriptor; a present but invalid `execution`
///   (`null`, a non-object, a missing or invalid `adapter`) is rejected;
/// - without `execution`, exactly one codec's complete fields derive it;
/// - the singular `target` must equal the only entry of `targets`;
/// - a compatibility field no codec of the Test's adapter owns, or any
///   other undeclared field, is rejected rather than ignored.
pub fn decode_test_json(
    value: &Value,
    registry: &AdapterRegistry,
) -> Result<TestEntity, WireError> {
    let Value::Object(object) = value else {
        return Err(WireError("a Test must be a JSON object".to_owned()));
    };
    let mut object = object.clone();
    let singular = object.remove("target");

    let codecs: Vec<&dyn TestWireCodec> = registry
        .ids()
        .filter_map(|id| registry.test_wire_codec(id))
        .collect();
    // DS-1578 derives `execution` from compatibility fields only when the
    // key is *absent*. A present `execution` is read strictly as an
    // `ExecutionDescriptor` first: `null`, a non-object, or one without a
    // valid `adapter` is rejected, never replaced by a compat-derived
    // descriptor that would name a different execution coordinate.
    let declared: Option<ExecutionDescriptor> = match object.get("execution") {
        None => None,
        Some(execution) => Some(
            serde_json::from_value(execution.clone())
                .map_err(|error| WireError(format!("invalid execution: {error}")))?,
        ),
    };
    let declared_adapter = declared
        .as_ref()
        .map(|execution| execution.adapter.as_str().to_owned());
    let codec = match &declared_adapter {
        Some(adapter) => registry.test_wire_codec(adapter),
        None => {
            let present: Vec<&dyn TestWireCodec> = codecs
                .iter()
                .copied()
                .filter(|codec| {
                    codec
                        .field_names()
                        .iter()
                        .any(|name| object.contains_key(*name))
                })
                .collect();
            match present.as_slice() {
                [] => None,
                [one] => Some(*one),
                _ => {
                    return Err(WireError(
                        "compatibility fields of more than one adapter are present".to_owned(),
                    ))
                }
            }
        }
    };
    let mut compat = WireFields::new();
    if let Some(codec) = codec {
        for name in codec.field_names() {
            if let Some(value) = object.remove(*name) {
                compat.insert((*name).to_owned(), value);
            }
        }
    }

    match (&declared_adapter, codec) {
        (Some(_), Some(codec)) if !compat.is_empty() => {
            let derived = codec.decode(&compat).map_err(WireError)?;
            if declared.as_ref() != Some(&derived) {
                return Err(WireError(
                    "execution and the compatibility fields describe different descriptors"
                        .to_owned(),
                ));
            }
        }
        (None, Some(codec)) => {
            let derived = codec.decode(&compat).map_err(WireError)?;
            object.insert(
                "execution".to_owned(),
                serde_json::to_value(derived).expect("descriptor serializes"),
            );
        }
        (None, None) => {
            return Err(WireError(
                "a Test without execution needs complete compatibility fields".to_owned(),
            ))
        }
        _ => {}
    }

    let test: TestEntity = serde_json::from_value(Value::Object(object.clone()))
        .map_err(|error| WireError(format!("invalid Test: {error}")))?;
    if let Value::Object(canonical) = serde_json::to_value(&test).expect("Test serializes") {
        if let Some(extra) = object.keys().find(|key| !canonical.contains_key(*key)) {
            return Err(WireError(format!("undeclared Test field `{extra}`")));
        }
    }
    if let Some(singular) = singular {
        match test.targets.as_slice() {
            [only] if target_value(only) == singular => {}
            _ => {
                return Err(WireError(
                    "the singular `target` must equal the only entry of `targets`".to_owned(),
                ))
            }
        }
    }
    Ok(test)
}

fn target_value(target: &TargetRef) -> Value {
    serde_json::to_value(target).expect("TargetRef serializes")
}
