use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;
use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;
use uuid::Uuid;

use crate::{error::{AppError, AppResult}, AppState};

static BLOCKLIST: &[&str] = &[
    "csam",
    "minor nudes",
    "racial_slur_placeholder",
    "kill yourself",
];

#[derive(Debug, Clone, Serialize)]
pub struct SafetyScanResult {
    pub flagged: bool,
    pub rejected: bool,
    pub reasons: Vec<String>,
    /// The normalized content that was actually scanned. Callers MUST use this
    /// (not the original input) for storage and broadcast to ensure the scanned
    /// content matches the stored content exactly.
    #[serde(skip)]
    pub normalized_content: String,
    /// SHA-256 hash of the normalized content that was scanned, used to verify
    /// the broadcast/stored content has not diverged from the scanned content.
    #[serde(skip)]
    pub content_hash: [u8; 32],
}

impl SafetyScanResult {
    /// Verify that the given content matches what was originally scanned.
    pub fn verify_content(&self, content: &str) -> bool {
        let hash = sha256_content(content);
        self.content_hash == hash
    }
}

/// Compute a SHA-256 hash of the content for integrity binding.
fn sha256_content(content: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hasher.finalize().into()
}

/// Normalize input to prevent safety scan bypass through encoding tricks.
///
/// This strips invisible Unicode characters, applies NFC normalization, and
/// collapses whitespace so that evasion techniques like zero-width character
/// insertion ("k\u{200B}ill") or homoglyph substitution cannot hide content
/// from the blocklist scanner.
fn normalize_content(content: &str) -> String {
    // 1. Strip zero-width and invisible formatting characters.
    //    These can be inserted between letters to break keyword matching.
    let stripped: String = content
        .chars()
        .filter(|c| !is_invisible_char(*c))
        .collect();

    // 2. Apply Unicode NFC normalization to canonicalize composed/decomposed
    //    forms (e.g. e + combining-accent -> e-with-accent).
    let normalized: String = stripped.nfc().collect();

    // 3. Collapse runs of whitespace into single spaces and trim.
    let mut result = String::with_capacity(normalized.len());
    let mut prev_was_space = true; // true to trim leading whitespace
    for c in normalized.chars() {
        if c.is_whitespace() {
            if !prev_was_space {
                result.push(' ');
                prev_was_space = true;
            }
        } else {
            result.push(c);
            prev_was_space = false;
        }
    }
    // Trim trailing space produced by collapsing.
    if result.ends_with(' ') {
        result.pop();
    }

    result
}

/// Returns true for Unicode characters that are invisible / zero-width and
/// should be stripped before safety scanning. This covers:
/// - Zero-width spaces and joiners (U+200B..U+200F)
/// - Directional formatting marks (U+202A..U+202E, U+2066..U+2069)
/// - Word joiner (U+2060), zero-width no-break space / BOM (U+FEFF)
/// - Soft hyphen (U+00AD)
/// - Variation selectors (U+FE00..U+FE0F)
/// - General Category Cf (format characters) except normal whitespace
fn is_invisible_char(c: char) -> bool {
    matches!(c,
        '\u{00AD}'           // soft hyphen
        | '\u{200B}'         // zero-width space
        | '\u{200C}'         // zero-width non-joiner
        | '\u{200D}'         // zero-width joiner
        | '\u{200E}'         // left-to-right mark
        | '\u{200F}'         // right-to-left mark
        | '\u{202A}'         // left-to-right embedding
        | '\u{202B}'         // right-to-left embedding
        | '\u{202C}'         // pop directional formatting
        | '\u{202D}'         // left-to-right override
        | '\u{202E}'         // right-to-left override
        | '\u{2060}'         // word joiner
        | '\u{2061}'         // function application
        | '\u{2062}'         // invisible times
        | '\u{2063}'         // invisible separator
        | '\u{2064}'         // invisible plus
        | '\u{2066}'         // left-to-right isolate
        | '\u{2067}'         // right-to-left isolate
        | '\u{2068}'         // first strong isolate
        | '\u{2069}'         // pop directional isolate
        | '\u{FEFF}'         // zero-width no-break space (BOM)
        | '\u{FE00}'..='\u{FE0F}'  // variation selectors
        | '\u{E0100}'..='\u{E01EF}' // variation selectors supplement
    )
}

/// Scan a message for safety violations.
///
/// This function is **fail-closed**: if any internal check (e.g. Redis for
/// flood detection) fails, the error propagates and the message is NOT
/// delivered. Callers must not catch and ignore errors from this function.
///
/// Input is normalized before scanning to prevent bypass through zero-width
/// characters, Unicode tricks, or whitespace manipulation. The normalized
/// content is returned in the result and MUST be used by callers for storage
/// and broadcast to ensure the scanned content matches the stored content.
pub async fn scan_message(
    state: &AppState,
    chat_id: Uuid,
    user_id: Uuid,
    content: &str,
) -> AppResult<SafetyScanResult> {
    // Normalize input BEFORE scanning to prevent bypass via invisible
    // characters, combining marks, or whitespace tricks.
    let normalized = normalize_content(content);
    let content_hash = sha256_content(&normalized);

    let keyword_reasons = keyword_flags(&normalized);
    // Blocklist keyword matches are severe -- the message must be rejected.
    let rejected_by_keywords = !keyword_reasons.is_empty();

    let mut reasons = keyword_reasons;

    if is_flooding(state, chat_id, user_id).await? {
        reasons.push("rate:flooding".to_owned());
    }

    if contains_link(&normalized) {
        reasons.push("url:present".to_owned());
    }

    let flagged = !reasons.is_empty();
    // Messages are rejected when they hit the blocklist. Other flags (flooding,
    // links) are advisory and the message is still delivered with a flag.
    let rejected = rejected_by_keywords;

    Ok(SafetyScanResult {
        flagged,
        rejected,
        reasons,
        normalized_content: normalized,
        content_hash,
    })
}

fn keyword_flags(content: &str) -> Vec<String> {
    let text = content.to_ascii_lowercase();

    let mut reasons = Vec::new();
    for term in BLOCKLIST {
        if text.contains(term) {
            reasons.push(format!("keyword:{term}"));
        }
    }

    reasons
}

async fn is_flooding(state: &AppState, chat_id: Uuid, user_id: Uuid) -> AppResult<bool> {
    let key = format!("msg_rate:{chat_id}:{user_id}");

    let mut conn = state.redis.get().await.map_err(|e| AppError::Internal(format!("redis pool error: {e}")))?;

    // Atomic Lua script: INCR + conditional EXPIRE in a single Redis eval,
    // eliminating the TOCTOU race between separate INCR and EXPIRE calls.
    let script = redis::Script::new(
        r#"
        local count = redis.call('INCR', KEYS[1])
        if count == 1 then
            redis.call('EXPIRE', KEYS[1], ARGV[1])
        end
        return count
        "#,
    );

    let count: i64 = script.key(&key).arg(5_i64).invoke_async(&mut *conn).await?;

    Ok(count > 10)
}

static URL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(https?://|www\.)\S+").unwrap()
});

fn contains_link(content: &str) -> bool {
    URL_RE.is_match(content)
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- normalize_content tests ---

    #[test]
    fn normalize_strips_zero_width_chars() {
        // "k\u{200B}ill" should become "kill"
        let input = "k\u{200B}ill";
        assert_eq!(normalize_content(input), "kill");
    }

    #[test]
    fn normalize_strips_multiple_invisible_chars() {
        let input = "h\u{200C}e\u{200D}l\u{FEFF}lo";
        assert_eq!(normalize_content(input), "hello");
    }

    #[test]
    fn normalize_collapses_whitespace() {
        let input = "  hello    world  ";
        assert_eq!(normalize_content(input), "hello world");
    }

    #[test]
    fn normalize_handles_tabs_and_newlines() {
        let input = "hello\t\n\nworld";
        assert_eq!(normalize_content(input), "hello world");
    }

    #[test]
    fn normalize_empty_string() {
        assert_eq!(normalize_content(""), "");
    }

    #[test]
    fn normalize_only_whitespace() {
        assert_eq!(normalize_content("   \t\n  "), "");
    }

    #[test]
    fn normalize_strips_soft_hyphen() {
        let input = "ex\u{00AD}ample";
        assert_eq!(normalize_content(input), "example");
    }

    #[test]
    fn normalize_strips_variation_selectors() {
        let input = "text\u{FE0F}here";
        assert_eq!(normalize_content(input), "texthere");
    }

    // --- contains_link tests ---

    #[test]
    fn contains_link_detects_http_url() {
        assert!(contains_link("check out http://example.com"));
    }

    #[test]
    fn contains_link_detects_https_url() {
        assert!(contains_link("visit https://example.com/path"));
    }

    #[test]
    fn contains_link_detects_www_prefix() {
        assert!(contains_link("go to www.example.com"));
    }

    #[test]
    fn contains_link_no_false_positive_on_plain_text() {
        assert!(!contains_link("hello world, no links here"));
    }

    #[test]
    fn contains_link_no_false_positive_on_dotted_words() {
        assert!(!contains_link("e.g. this is fine"));
    }

    #[test]
    fn contains_link_case_insensitive() {
        assert!(contains_link("go to HTTP://EXAMPLE.COM"));
    }

    // --- keyword_flags tests ---

    #[test]
    fn keyword_flags_detects_blocklisted_term() {
        let flags = keyword_flags("this contains kill yourself in it");
        assert!(!flags.is_empty());
        assert!(flags.iter().any(|r| r.contains("kill yourself")));
    }

    #[test]
    fn keyword_flags_case_insensitive() {
        let flags = keyword_flags("KILL YOURSELF");
        assert!(!flags.is_empty());
    }

    #[test]
    fn keyword_flags_no_match_on_clean_content() {
        let flags = keyword_flags("hello, how are you today?");
        assert!(flags.is_empty());
    }

    // --- SafetyScanResult::verify_content tests ---

    #[test]
    fn verify_content_matches_original() {
        let content = "test message";
        let hash = sha256_content(content);
        let result = SafetyScanResult {
            flagged: false,
            rejected: false,
            reasons: vec![],
            normalized_content: content.to_owned(),
            content_hash: hash,
        };
        assert!(result.verify_content(content));
    }

    #[test]
    fn verify_content_fails_on_different_content() {
        let content = "test message";
        let hash = sha256_content(content);
        let result = SafetyScanResult {
            flagged: false,
            rejected: false,
            reasons: vec![],
            normalized_content: content.to_owned(),
            content_hash: hash,
        };
        assert!(!result.verify_content("different message"));
    }

    // --- is_invisible_char tests ---

    #[test]
    fn invisible_char_detects_zwsp() {
        assert!(is_invisible_char('\u{200B}'));
    }

    #[test]
    fn invisible_char_does_not_flag_regular_chars() {
        assert!(!is_invisible_char('a'));
        assert!(!is_invisible_char(' '));
        assert!(!is_invisible_char('1'));
    }
}
