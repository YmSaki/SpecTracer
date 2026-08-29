use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fmt, str::FromStr};

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
}
