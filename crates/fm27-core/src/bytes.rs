//! Shared byte-search helpers for save layout scans.

/// Find the first occurrence of `needle` in `haystack`.
///
/// Kept behind one helper so every parser path uses the same optimized search
/// implementation and empty-needle semantics.
pub(crate) fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    memchr::memmem::find(haystack, needle)
}

#[cfg(test)]
mod tests {
    use super::find_bytes;

    #[test]
    fn finds_first_match_and_handles_empty_or_missing_needles() {
        assert_eq!(find_bytes(b"abcabc", b"bc"), Some(1));
        assert_eq!(find_bytes(b"abc", b""), Some(0));
        assert_eq!(find_bytes(b"abc", b"abcd"), None);
    }
}
