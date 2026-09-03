use serde::{Deserialize, Serialize};
use std::fmt;

/// Defines a type-safe identifier backed by `String`.
///
/// Use this for canonical identifier types such as `VoId` and `TestId`.
/// Each generated type is distinct at compile time, while serializing
/// transparently as a plain string.
macro_rules! define_string_id_type {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_owned())
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

define_string_id_type!(
    /// Identifier of a specification document.
    SpecId
);

define_string_id_type!(
    /// Identifier of a requirement document.
    ReqId
);

define_string_id_type!(
    /// Identifier of a verification objective.
    VoId
);

define_string_id_type!(
    /// Identifier of a managed test entity.
    TestId
);

define_string_id_type!(
    /// Identifier of a source-level implementation target.
    SrcId
);

define_string_id_type!(
    /// Identifier of a canonical document record.
    DocumentId
);

define_string_id_type!(
    /// Identifier of a registered `SourceDiscoveryAdapter` (詳細設計 v0.1
    /// §5.2「各adapterは一意なID、languages、capabilities、config namespaceを
    /// 宣言し」、§6.1「coreは`TargetRef::Locator.adapter`をregistryで解決
    /// し」)。`config.yaml` の `adapters[].id` および `AdapterRegistry` の
    /// キーと同じ文字列空間を指す。
    AdapterId
);
