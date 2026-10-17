//! Minimal glob matching for user rules: `*` (inside one segment) and `**` (any number
//! of segments). Comparison follows the platform: case-insensitive on Windows.

/// Whether `text` matches `pattern`. Both use `/` or `\` as separators.
pub fn matches(pattern: &str, text: &str) -> bool {
    let pattern = normalize(pattern);
    let text = normalize(text);
    let pattern: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();
    let text: Vec<&str> = text.split('/').filter(|s| !s.is_empty()).collect();
    match_segments(&pattern, &text)
}

fn normalize(value: &str) -> String {
    let value = value.replace('\\', "/");
    if cfg!(windows) {
        value.to_lowercase()
    } else {
        value
    }
}

fn match_segments(pattern: &[&str], text: &[&str]) -> bool {
    match pattern.split_first() {
        None => text.is_empty(),
        Some((&"**", rest)) => (0..=text.len()).any(|skip| match_segments(rest, &text[skip..])),
        Some((first, rest)) => text.split_first().is_some_and(|(segment, others)| {
            match_segment(first.as_bytes(), segment.as_bytes()) && match_segments(rest, others)
        }),
    }
}

/// `*` matches any run of characters within one segment.
fn match_segment(pattern: &[u8], text: &[u8]) -> bool {
    match pattern.split_first() {
        None => text.is_empty(),
        Some((b'*', rest)) => (0..=text.len()).any(|skip| match_segment(rest, &text[skip..])),
        Some((first, rest)) => text
            .split_first()
            .is_some_and(|(c, others)| c == first && match_segment(rest, others)),
    }
}

#[cfg(test)]
mod tests {
    use super::matches;

    #[test]
    fn literal_paths_match_exactly() {
        assert!(matches("C:/Projetos/tcc", "C:\\Projetos\\tcc"));
        assert!(!matches("C:/Projetos/tcc", "C:/Projetos/tcc2"));
        assert!(!matches("C:/Projetos/tcc", "C:/Projetos"));
    }

    #[test]
    fn single_star_stays_inside_one_segment() {
        assert!(matches("release/*", "release/2.0"));
        assert!(!matches("release/*", "release/2.0/hotfix"));
        assert!(!matches("release/*", "releases/2.0"));
        assert!(matches("*-dados", "postgres-dados"));
        assert!(matches("a*b*c", "aXXbYYc"));
        assert!(!matches("a*b*c", "aXXbYY"));
    }

    #[test]
    fn double_star_spans_any_number_of_segments() {
        assert!(matches("**/vendor", "C:/Projetos/app/vendor"));
        assert!(matches("**/vendor", "vendor"));
        assert!(matches("C:/Projetos/**", "C:/Projetos/a/b/c"));
        assert!(matches("C:/Projetos/**/cache", "C:/Projetos/cache"));
        assert!(!matches("**/vendor", "C:/Projetos/app/vendors"));
    }

    #[cfg(windows)]
    #[test]
    fn windows_matching_ignores_case() {
        assert!(matches("C:/PROJETOS/TCC", "c:\\projetos\\tcc"));
    }
}
