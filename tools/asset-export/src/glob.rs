//! A tiny wildcard matcher for the `--filter` option: `*` matches any run of characters
//! (including `/`), `?` matches one character, everything else matches itself. Matching is
//! case-insensitive because the VFS paths are lower case.

/// `true` if `text` matches `pattern` completely.
pub fn matches(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.to_lowercase().chars().collect();
    let t: Vec<char> = text.to_lowercase().chars().collect();
    let (mut pi, mut ti) = (0usize, 0usize);
    let (mut star, mut mark) = (None::<usize>, 0usize);
    while ti < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) && p[pi] != '*' {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ti;
            pi += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ti = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

#[cfg(test)]
mod tests {
    use super::matches;

    #[test]
    fn literal_and_question_mark() {
        assert!(matches("a.tex", "a.tex"));
        assert!(matches("a.tex", "A.TEX"));
        assert!(!matches("a.tex", "b.tex"));
        assert!(!matches("a.tex", "a.tex2"));
        assert!(matches("a.te?", "a.tex"));
        assert!(!matches("a.te?", "a.te"));
        assert!(matches("", ""));
        assert!(!matches("", "a"));
    }

    #[test]
    fn star_matches_runs_including_slashes() {
        assert!(matches("*", ""));
        assert!(matches("*", "anything/at/all"));
        assert!(matches("content/*/body.tex", "content/a/b/body.tex"));
        assert!(matches("*.tex", "x/y.tex"));
        assert!(!matches("*.tex", "x/y.bmf"));
        assert!(matches("a*b*c", "aXXbYYc"));
        assert!(matches("a*b*c", "abc"));
        assert!(!matches("a*b*c", "acb"));
        assert!(matches("**a", "a"));
        assert!(matches("*a*", "bab"));
    }

    #[test]
    fn backtracking_finds_a_later_match() {
        assert!(matches("*ab", "aab"));
        assert!(matches("*aab", "aaab"));
        assert!(!matches("*aab", "aaba"));
    }

    #[test]
    fn the_pattern_is_case_insensitive_and_text_may_contain_a_star() {
        assert!(matches("A.TEX", "a.tex"));
        assert!(matches("*B", "x/b"));
        // A literal star in the text is just a character; the star of the pattern is the wildcard.
        assert!(matches("*b", "*ab"));
        assert!(matches("a*", "a*"));
    }
}
