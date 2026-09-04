use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fmt, str::FromStr};

/// SHA-256 hash of a canonical source or record representation.
///
/// The serialized form is `sha256:<64 hexadecimal characters>`.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ContentHash(String);

impl ContentHash {
    pub fn from_text(text: &str) -> Self {
        let normalized = normalize_hashed_text(text);
        let digest = Sha256::digest(normalized.as_bytes());
        Self(format!("sha256:{digest:x}"))
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let text = String::from_utf8_lossy(bytes);
        Self::from_text(&text)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Hashes an already-canonical byte sequence directly.
    ///
    /// Unlike [`ContentHash::from_bytes`], this does not lossily reinterpret
    /// the input as UTF-8 text or apply [`normalize_hashed_text`]. It is the
    /// terminal step of [`SubjectHashInput`]: the encoded hash-input bytes
    /// produced by that builder are already the canonical §1.3 encoding, so
    /// no further transformation is applied before SHA-256.
    pub fn from_canonical_bytes(bytes: &[u8]) -> Self {
        let digest = Sha256::digest(bytes);
        Self(format!("sha256:{digest:x}"))
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for ContentHash {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value
            .strip_prefix("sha256:")
            .is_some_and(|hex| hex.len() == 64)
            && value[7..].chars().all(|ch| ch.is_ascii_hexdigit())
        {
            Ok(Self(value.to_owned()))
        } else {
            Err("content hash must be sha256:<64 hexadecimal characters>".to_owned())
        }
    }
}

/// Normalizes text before content hashing according to detailed design §1.3.
///
/// Converts CRLF and CR line endings to LF and removes trailing spaces and
/// tabs from each line. Leading whitespace and the presence of the final
/// newline are preserved.
pub fn normalize_hashed_text(text: &str) -> String {
    let normalized_endings = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut normalized = String::with_capacity(normalized_endings.len());
    for line in normalized_endings.split_inclusive('\n') {
        let has_newline = line.ends_with('\n');
        let body = if has_newline {
            &line[..line.len() - 1]
        } else {
            line
        };
        normalized.push_str(body.trim_end_matches([' ', '\t']));
        if has_newline {
            normalized.push('\n');
        }
    }
    normalized
}

/// The domain separator for one of the five content hash subject kinds
/// defined by 詳細設計 §1.3 (本冊:87-91).
///
/// Values are the literal domain strings from the spec. This is a closed
/// set — the spec defines exactly these five subject hashes and no others
/// (`本冊:87`〜`91`); do not add variants without a corresponding spec entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubjectDomain {
    /// Test subject hash (本冊:87: "domain `vtest:test-subject:v1`").
    TestSubject,
    /// Source Target hash (本冊:88: "domain `vtest:target-subject:v1`").
    TargetSubject,
    /// document subject hash (本冊:89: "domain `vtest:document-subject:v1`").
    DocumentSubject,
    /// VO subject hash (本冊:90: "domain `vtest:record-subject:v1`").
    RecordSubject,
    /// Execution State subject hash (本冊:91: "domain `vtest:execution-state:v1`").
    ExecutionState,
}

impl SubjectDomain {
    /// Returns the literal domain separator string (verbatim from §1.3).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TestSubject => "vtest:test-subject:v1",
            Self::TargetSubject => "vtest:target-subject:v1",
            Self::DocumentSubject => "vtest:document-subject:v1",
            Self::RecordSubject => "vtest:record-subject:v1",
            Self::ExecutionState => "vtest:execution-state:v1",
        }
    }
}

/// Tag byte identifying which encoding a field's payload uses.
///
/// 本冊:85 requires that `null`, an empty string, and an empty list encode
/// to different values ("null、空文字、空listは異なる値としてencodeする").
/// The literal `field-name` / `UTF-8 byte length` / `byte列` triple alone
/// cannot satisfy this: a `null` field and an empty scalar would both encode
/// as a zero-length byte run. A one-byte kind tag ahead of the
/// length-and-bytes payload is the minimal addition that makes the four
/// payload shapes (absent, scalar, sequence, map) mutually distinguishable
/// regardless of length. This tag is an implementation necessity, not a
/// literal spec quotation — see the PR report for this call-out.
mod field_tag {
    pub const NULL: u8 = 0;
    pub const SCALAR: u8 = 1;
    pub const SEQUENCE: u8 = 2;
    pub const MAP: u8 = 3;
}

/// One field's value in a canonical §1.3 hash input.
///
/// Construct scalar text through [`FieldValue::text_fragment`] or
/// [`FieldValue::exact_bytes`] depending on whether the subject-specific
/// rule for that field requires byte-exactness (本冊:83).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FieldValue {
    /// The declaration itself is absent (distinct from a present-but-empty
    /// value; 本冊:85, 87 "宣言の不在と空値の明示は異なる値としてencodeする").
    Null,
    /// A scalar value: text or opaque bytes. May be empty.
    Scalar(Vec<u8>),
    /// An element sequence whose order carries meaning, encoded in the
    /// given order (e.g. `cases`; 本冊:85 "順序に意味があるcasesは宣言順とする").
    /// May be empty.
    Ordered(Vec<Vec<u8>>),
    /// An element sequence treated as a set (e.g. `covers` / `targets` /
    /// `related`). The builder sorts the elements ascending by byte value
    /// before encoding, so the input order never affects the hash (本冊:85
    /// "集合として扱う`covers`・`targets`・`related`は正規化値の昇順").
    /// Each element must already be its subject-specific normalized-value
    /// representation; this type only fixes the *order*, not the value
    /// normalization itself. Duplicates are preserved as given — rejecting
    /// duplicate entries is a record/scan-layer concern, not this encoder's.
    /// May be empty.
    Set(Vec<Vec<u8>>),
    /// A key-sorted map (本冊:85 "mapはkey昇順"). Keys are ordered by
    /// `BTreeMap`'s `Ord for String`, which orders equivalently to raw UTF-8
    /// byte order.
    Map(BTreeMap<String, Vec<u8>>),
}

impl FieldValue {
    /// A scalar text fragment for which the subject-specific rule does not
    /// require byte-exactness: normalizes line endings to LF and strips
    /// trailing whitespace per line before encoding (本冊:83), via
    /// [`normalize_hashed_text`].
    pub fn text_fragment(text: &str) -> Self {
        Self::Scalar(normalize_hashed_text(text).into_bytes())
    }

    /// A scalar value encoded byte-exactly, with no normalization applied.
    /// Use this where the subject-specific rule requires byte-exactness
    /// (本冊:83 "subject固有規則でbyte-exactを要求する" fields).
    pub fn exact_bytes(bytes: impl Into<Vec<u8>>) -> Self {
        Self::Scalar(bytes.into())
    }
}

/// Builds one §1.3 canonical hash input: a domain separator followed by
/// length-prefixed fields, and hashes it with SHA-256 (本冊:83-85).
///
/// Every discrete byte-string component this builder writes — the domain
/// separator, each field name, each scalar/sequence-element/map-key/map-value
/// — is prefixed with its own big-endian `u64` UTF-8 byte length before its
/// bytes. This is a uniform elaboration of the spec's "長さ付きfield"
/// principle (本冊:85) applied to every component, not just the outermost
/// field value; it is what makes "単純な文字列連結を行わない" (本冊:85) hold
/// for field names and map keys as well as scalar values, and is the
/// property the `{a:"xy",b:"z"}` vs `{a:"x",b:"yz"}` non-collision test
/// below exercises.
pub struct SubjectHashInput {
    buf: Vec<u8>,
}

impl SubjectHashInput {
    /// Starts a new hash input for the given subject domain.
    pub fn new(domain: SubjectDomain) -> Self {
        let mut buf = Vec::new();
        push_len_prefixed(&mut buf, domain.as_str().as_bytes());
        Self { buf }
    }

    /// Appends one named field: `field-name`, then its value, in that order
    /// (本冊:85).
    pub fn field(mut self, name: &str, value: FieldValue) -> Self {
        encode_field_into(&mut self.buf, name, value);
        self
    }

    /// Finalizes the accumulated hash input into a [`ContentHash`].
    pub fn finish(self) -> ContentHash {
        ContentHash::from_canonical_bytes(&self.buf)
    }
}

/// Encodes a nested record's fields as an opaque byte string, for use as one
/// element of a [`FieldValue::Ordered`]/[`FieldValue::Set`] list or a
/// [`FieldValue::Map`] value, when a subject's canonical metadata contains a
/// list of small structured entries (e.g. a document subject's
/// `derives_from[]`, each a `{doc, anchor, note}` record).
///
/// Uses the exact same field-tag encoding as [`SubjectHashInput::field`],
/// without a leading domain separator — a nested entry does not repeat the
/// domain, since it is encoded once at the outer [`SubjectHashInput::new`].
pub fn encode_nested_fields<'a>(
    fields: impl IntoIterator<Item = (&'a str, FieldValue)>,
) -> Vec<u8> {
    let mut buf = Vec::new();
    for (name, value) in fields {
        encode_field_into(&mut buf, name, value);
    }
    buf
}

fn encode_field_into(buf: &mut Vec<u8>, name: &str, value: FieldValue) {
    push_len_prefixed(buf, name.as_bytes());
    match value {
        FieldValue::Null => buf.push(field_tag::NULL),
        FieldValue::Scalar(bytes) => {
            buf.push(field_tag::SCALAR);
            push_len_prefixed(buf, &bytes);
        }
        FieldValue::Ordered(items) => {
            buf.push(field_tag::SEQUENCE);
            push_count(buf, items.len());
            for item in items {
                push_len_prefixed(buf, &item);
            }
        }
        FieldValue::Set(mut items) => {
            items.sort();
            buf.push(field_tag::SEQUENCE);
            push_count(buf, items.len());
            for item in items {
                push_len_prefixed(buf, &item);
            }
        }
        FieldValue::Map(map) => {
            buf.push(field_tag::MAP);
            push_count(buf, map.len());
            for (key, value) in map {
                push_len_prefixed(buf, key.as_bytes());
                push_len_prefixed(buf, &value);
            }
        }
    }
}

fn push_len_prefixed(buf: &mut Vec<u8>, bytes: &[u8]) {
    push_count(buf, bytes.len());
    buf.extend_from_slice(bytes);
}

fn push_count(buf: &mut Vec<u8>, count: usize) {
    buf.extend_from_slice(&(count as u64).to_be_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @vtest.id TEST-MODEL-CONTENT-HASH-NORMALIZATION
    /// @vtest.covers VO-MODEL-CONTENT-HASH-NORMALIZATION
    /// @vtest.target crates/vtest-model/src/hash.rs::normalize_hashed_text
    /// @vtest.intent verifies canonical text normalization for content hashing
    #[test]
    fn hash_normalization_only_changes_line_endings_and_trailing_space() {
        assert_eq!(
            ContentHash::from_text("a  \r\nb  \r\n"),
            ContentHash::from_text("a\nb\n")
        );
        assert_ne!(
            ContentHash::from_text("a\n b\n"),
            ContentHash::from_text("a\nb\n")
        );
        assert_ne!(ContentHash::from_text("a"), ContentHash::from_text("a\n"));
    }

    /// @vtest.id TEST-MODEL-CONTENT-HASH-FROM-CANONICAL-BYTES-IS-EXACT
    /// @vtest.covers VO-MODEL-CANONICAL-HASH-ENCODING
    /// @vtest.target crates/vtest-model/src/hash.rs::ContentHash::from_canonical_bytes
    /// @vtest.intent verifies from_canonical_bytes hashes raw bytes directly, unlike from_text/from_bytes which normalize
    #[test]
    fn content_hash_from_canonical_bytes_does_not_normalize() {
        assert_ne!(
            ContentHash::from_canonical_bytes(b"a  \r\n"),
            ContentHash::from_canonical_bytes(b"a\n")
        );
    }

    /// @vtest.id TEST-MODEL-CANONICAL-HASH-DOMAIN-SEPARATION
    /// @vtest.covers VO-MODEL-CANONICAL-HASH-ENCODING
    /// @vtest.target crates/vtest-model/src/hash.rs::SubjectHashInput
    /// @vtest.intent verifies each of the five §1.3 domains yields a distinct hash for identical field content (本冊:87-91)
    #[test]
    fn canonical_hash_domain_separator_distinguishes_all_five_domains() {
        let domains = [
            SubjectDomain::TestSubject,
            SubjectDomain::TargetSubject,
            SubjectDomain::DocumentSubject,
            SubjectDomain::RecordSubject,
            SubjectDomain::ExecutionState,
        ];
        let hashes: Vec<ContentHash> = domains
            .iter()
            .map(|&domain| {
                SubjectHashInput::new(domain)
                    .field(
                        "id",
                        FieldValue::exact_bytes(b"same-field-content".to_vec()),
                    )
                    .finish()
            })
            .collect();
        for i in 0..hashes.len() {
            for j in (i + 1)..hashes.len() {
                assert_ne!(
                    hashes[i], hashes[j],
                    "domains {:?} and {:?} collided",
                    domains[i], domains[j]
                );
            }
        }
    }

    /// @vtest.id TEST-MODEL-CANONICAL-HASH-NULL-EMPTY-DISTINCT
    /// @vtest.covers VO-MODEL-CANONICAL-HASH-ENCODING
    /// @vtest.target crates/vtest-model/src/hash.rs::SubjectHashInput
    /// @vtest.intent verifies null, empty string, and empty list encode to three pairwise distinct hashes (本冊:85 "null、空文字、空listは異なる値としてencodeする")
    #[test]
    fn canonical_hash_null_empty_string_and_empty_list_are_pairwise_distinct() {
        let null_hash = SubjectHashInput::new(SubjectDomain::TestSubject)
            .field("input", FieldValue::Null)
            .finish();
        let empty_string_hash = SubjectHashInput::new(SubjectDomain::TestSubject)
            .field("input", FieldValue::exact_bytes(Vec::<u8>::new()))
            .finish();
        let empty_list_hash = SubjectHashInput::new(SubjectDomain::TestSubject)
            .field("input", FieldValue::Ordered(Vec::new()))
            .finish();
        assert_ne!(null_hash, empty_string_hash);
        assert_ne!(null_hash, empty_list_hash);
        assert_ne!(empty_string_hash, empty_list_hash);
    }

    /// @vtest.id TEST-MODEL-CANONICAL-HASH-LENGTH-PREFIX-PREVENTS-COLLISION
    /// @vtest.covers VO-MODEL-CANONICAL-HASH-ENCODING
    /// @vtest.target crates/vtest-model/src/hash.rs::SubjectHashInput
    /// @vtest.intent verifies length-prefixed field encoding avoids the classic concatenation collision (本冊:85 "単純な文字列連結を行わない")
    #[test]
    fn canonical_hash_length_prefixing_distinguishes_inputs_that_would_naively_collide() {
        // Under naive string concatenation, {a:"xy", b:"z"} and {a:"x", b:"yz"}
        // both produce the literal byte string "xyz" and would collide.
        let split_ab = SubjectHashInput::new(SubjectDomain::TestSubject)
            .field("a", FieldValue::exact_bytes(b"xy".to_vec()))
            .field("b", FieldValue::exact_bytes(b"z".to_vec()))
            .finish();
        let split_a_b2 = SubjectHashInput::new(SubjectDomain::TestSubject)
            .field("a", FieldValue::exact_bytes(b"x".to_vec()))
            .field("b", FieldValue::exact_bytes(b"yz".to_vec()))
            .finish();
        assert_ne!(split_ab, split_a_b2);
    }

    /// @vtest.id TEST-MODEL-CANONICAL-HASH-MAP-KEY-ORDER-INDEPENDENT
    /// @vtest.covers VO-MODEL-CANONICAL-HASH-ENCODING
    /// @vtest.target crates/vtest-model/src/hash.rs::SubjectHashInput
    /// @vtest.intent verifies map fields hash the same regardless of construction/insertion order (本冊:85 "mapはkey昇順")
    #[test]
    fn canonical_hash_map_field_is_independent_of_insertion_order() {
        let mut map_ab = BTreeMap::new();
        map_ab.insert("a".to_string(), b"1".to_vec());
        map_ab.insert("b".to_string(), b"2".to_vec());

        let mut map_ba = BTreeMap::new();
        map_ba.insert("b".to_string(), b"2".to_vec());
        map_ba.insert("a".to_string(), b"1".to_vec());

        let hash_ab = SubjectHashInput::new(SubjectDomain::ExecutionState)
            .field("config", FieldValue::Map(map_ab))
            .finish();
        let hash_ba = SubjectHashInput::new(SubjectDomain::ExecutionState)
            .field("config", FieldValue::Map(map_ba))
            .finish();
        assert_eq!(hash_ab, hash_ba);
    }

    /// @vtest.id TEST-MODEL-CANONICAL-HASH-SET-FIELD-ORDER-INDEPENDENT
    /// @vtest.covers VO-MODEL-CANONICAL-HASH-ENCODING
    /// @vtest.target crates/vtest-model/src/hash.rs::SubjectHashInput
    /// @vtest.intent verifies set-typed fields (covers/targets/related) hash the same regardless of declared order (本冊:85)
    #[test]
    fn canonical_hash_set_field_is_independent_of_declared_order() {
        let forward = vec![b"VO-A".to_vec(), b"VO-B".to_vec(), b"VO-C".to_vec()];
        let mut shuffled = forward.clone();
        shuffled.reverse();
        assert_ne!(
            forward, shuffled,
            "precondition: the two orders must actually differ"
        );

        let hash_forward = SubjectHashInput::new(SubjectDomain::TestSubject)
            .field("covers", FieldValue::Set(forward))
            .finish();
        let hash_shuffled = SubjectHashInput::new(SubjectDomain::TestSubject)
            .field("covers", FieldValue::Set(shuffled))
            .finish();
        assert_eq!(hash_forward, hash_shuffled);
    }

    /// @vtest.id TEST-MODEL-CANONICAL-HASH-ORDERED-FIELD-ORDER-DEPENDENT
    /// @vtest.covers VO-MODEL-CANONICAL-HASH-ENCODING
    /// @vtest.target crates/vtest-model/src/hash.rs::SubjectHashInput
    /// @vtest.intent verifies order-significant fields (cases) change hash when declaration order changes (本冊:85 "順序に意味があるcasesは宣言順とする")
    #[test]
    fn canonical_hash_ordered_field_changes_when_declaration_order_changes() {
        let forward = vec![b"case-one".to_vec(), b"case-two".to_vec()];
        let mut reversed = forward.clone();
        reversed.reverse();

        let hash_forward = SubjectHashInput::new(SubjectDomain::TestSubject)
            .field("cases", FieldValue::Ordered(forward))
            .finish();
        let hash_reversed = SubjectHashInput::new(SubjectDomain::TestSubject)
            .field("cases", FieldValue::Ordered(reversed))
            .finish();
        assert_ne!(hash_forward, hash_reversed);
    }

    /// @vtest.id TEST-MODEL-CANONICAL-HASH-TEXT-FRAGMENT-NORMALIZES-LINE-ENDINGS
    /// @vtest.covers VO-MODEL-CANONICAL-HASH-ENCODING
    /// @vtest.target crates/vtest-model/src/hash.rs::FieldValue::text_fragment
    /// @vtest.intent verifies text_fragment fields unify CRLF/CR to LF and strip trailing per-line whitespace (本冊:83)
    #[test]
    fn canonical_hash_text_fragment_field_normalizes_line_endings_and_trailing_space() {
        let crlf = SubjectHashInput::new(SubjectDomain::TestSubject)
            .field("intent", FieldValue::text_fragment("a  \r\nb  \r\n"))
            .finish();
        let lf = SubjectHashInput::new(SubjectDomain::TestSubject)
            .field("intent", FieldValue::text_fragment("a\nb\n"))
            .finish();
        assert_eq!(crlf, lf);
    }

    /// @vtest.id TEST-MODEL-CANONICAL-HASH-TEXT-FRAGMENT-PRESERVES-LEADING-SPACE-AND-TRAILING-NEWLINE
    /// @vtest.covers VO-MODEL-CANONICAL-HASH-ENCODING
    /// @vtest.target crates/vtest-model/src/hash.rs::FieldValue::text_fragment
    /// @vtest.intent verifies text_fragment fields preserve leading whitespace and trailing-newline presence (本冊:83 "これ以外の空白は正規化しない")
    #[test]
    fn canonical_hash_text_fragment_field_preserves_leading_space_and_trailing_newline_presence() {
        let leading_space = SubjectHashInput::new(SubjectDomain::TestSubject)
            .field("intent", FieldValue::text_fragment("a\n b\n"))
            .finish();
        let no_leading_space = SubjectHashInput::new(SubjectDomain::TestSubject)
            .field("intent", FieldValue::text_fragment("a\nb\n"))
            .finish();
        assert_ne!(leading_space, no_leading_space);

        let with_trailing_newline = SubjectHashInput::new(SubjectDomain::TestSubject)
            .field("intent", FieldValue::text_fragment("a"))
            .finish();
        let without_trailing_newline = SubjectHashInput::new(SubjectDomain::TestSubject)
            .field("intent", FieldValue::text_fragment("a\n"))
            .finish();
        assert_ne!(with_trailing_newline, without_trailing_newline);
    }

    /// @vtest.id TEST-MODEL-CANONICAL-HASH-EXACT-BYTES-SKIPS-NORMALIZATION
    /// @vtest.covers VO-MODEL-CANONICAL-HASH-ENCODING
    /// @vtest.target crates/vtest-model/src/hash.rs::FieldValue::exact_bytes
    /// @vtest.intent verifies exact_bytes fields are byte-exact and do not apply line-ending/trailing-space normalization
    #[test]
    fn canonical_hash_exact_bytes_field_does_not_normalize() {
        let crlf = SubjectHashInput::new(SubjectDomain::ExecutionState)
            .field(
                "manifest_entry",
                FieldValue::exact_bytes(b"a  \r\n".to_vec()),
            )
            .finish();
        let lf = SubjectHashInput::new(SubjectDomain::ExecutionState)
            .field("manifest_entry", FieldValue::exact_bytes(b"a\n".to_vec()))
            .finish();
        assert_ne!(crlf, lf);
    }
}
