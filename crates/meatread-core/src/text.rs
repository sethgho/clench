//! Text-level rules: word splitting, pivot (ORP) selection, long-word
//! splitting and sentence-boundary heuristics.

use unicode_segmentation::UnicodeSegmentation;

/// Whether `c` separates words. Unicode whitespace separates words except the
/// no-break spaces (U+00A0, U+2007, U+202F), which authors use to keep words
/// together.
pub(crate) fn is_separator(c: char) -> bool {
    c.is_whitespace() && !matches!(c, '\u{00A0}' | '\u{2007}' | '\u{202F}')
}

/// Byte ranges of whitespace-delimited words in `text`.
pub(crate) fn words(text: &str) -> impl Iterator<Item = (usize, usize)> + '_ {
    let mut chars = text.char_indices().peekable();
    core::iter::from_fn(move || {
        while let Some(&(_, c)) = chars.peek() {
            if is_separator(c) {
                chars.next();
            } else {
                break;
            }
        }
        let (start, _) = *chars.peek()?;
        let mut end = text.len();
        while let Some(&(i, c)) = chars.peek() {
            if is_separator(c) {
                end = i;
                break;
            }
            chars.next();
        }
        Some((start, end))
    })
}

fn is_letterlike(grapheme: &str) -> bool {
    grapheme.chars().next().is_some_and(char::is_alphanumeric)
}

/// Grapheme index of the pivot within a core of `len` graphemes.
pub(crate) const fn pivot_index(len: usize) -> usize {
    match len {
        0 | 1 => 0,
        2..=5 => 1,
        6..=9 => 2,
        10..=13 => 3,
        _ => 4,
    }
}

/// The pivot of `word` as a byte range within it, plus the grapheme length of
/// the word's core.
///
/// The core runs from the first to the last letter-like grapheme (one whose
/// first scalar is alphanumeric), so leading and trailing punctuation are
/// ignored while inner punctuation such as an apostrophe counts. A word with
/// no letter-like grapheme uses all of its graphemes as the core.
pub(crate) fn pivot(word: &str) -> (usize, usize, usize) {
    let mut first = None;
    let mut last = 0;
    let mut count = 0;
    for (i, g) in word.graphemes(true).enumerate() {
        count = i + 1;
        if is_letterlike(g) {
            first.get_or_insert(i);
            last = i;
        }
    }
    let (core_start, core_len) = match first {
        Some(f) => (f, last - f + 1),
        None => (0, count),
    };
    let target = core_start + pivot_index(core_len);
    match word.grapheme_indices(true).nth(target) {
        Some((at, g)) => (at, at + g.len(), core_len),
        None => (0, 0, 0),
    }
}

/// Whether a long word may be broken immediately after `grapheme`.
fn is_break_after(grapheme: &str) -> bool {
    matches!(
        grapheme,
        "/" | "\\"
            | "-"
            | "\u{2010}"
            | "\u{2013}"
            | "\u{2014}"
            | "_"
            | "."
            | ","
            | ";"
            | ":"
            | "?"
            | "&"
            | "="
            | "#"
            | "+"
            | "|"
            | "~"
            | "@"
    )
}

/// Splits `word` into contiguous pieces of at most `max_width` graphemes and
/// calls `emit(start, end)` for each byte range, in order.
///
/// Pieces end just after a break character (see the crate docs) and are packed
/// greedily: as many break-delimited segments as fit go into one piece. A
/// segment that alone exceeds `max_width` is cut every `max_width` graphemes.
/// A `max_width` of `0` disables splitting.
pub(crate) fn split_long(word: &str, max_width: usize, mut emit: impl FnMut(usize, usize)) {
    if max_width == 0 || word.graphemes(true).nth(max_width).is_none() {
        emit(0, word.len());
        return;
    }
    let mut piece_start = 0;
    let mut piece_len = 0;
    let mut seg_start = 0;
    let mut seg_len = 0;
    for (i, g) in word.grapheme_indices(true) {
        seg_len += 1;
        let seg_end = i + g.len();
        if seg_end != word.len() && !is_break_after(g) {
            continue;
        }
        if piece_len + seg_len <= max_width {
            piece_len += seg_len;
        } else {
            if piece_len > 0 {
                emit(piece_start, seg_start);
                piece_start = seg_start;
            }
            if seg_len <= max_width {
                piece_len = seg_len;
            } else {
                let mut n = 0;
                for (j, _) in word[seg_start..seg_end].grapheme_indices(true) {
                    if n == max_width {
                        emit(piece_start, seg_start + j);
                        piece_start = seg_start + j;
                        n = 0;
                    }
                    n += 1;
                }
                piece_len = n;
            }
        }
        seg_start = seg_end;
        seg_len = 0;
    }
    emit(piece_start, word.len());
}

fn is_closer(c: char) -> bool {
    matches!(
        c,
        '"' | '\'' | ')' | ']' | '}' | '\u{201D}' | '\u{2019}' | '\u{00BB}' | '\u{203A}' | '*'
    )
}

fn is_opener(c: char) -> bool {
    matches!(
        c,
        '"' | '\''
            | '('
            | '['
            | '{'
            | '\u{201C}'
            | '\u{2018}'
            | '\u{00AB}'
            | '\u{2039}'
            | '*'
            | '\u{00BF}'
            | '\u{00A1}'
    )
}

/// Titles and reference abbreviations that are followed by a name or number
/// rather than ending a sentence. Compared case-insensitively without the dot.
const NON_TERMINAL_ABBREVIATIONS: &[&str] = &[
    "mr", "mrs", "ms", "dr", "prof", "sr", "jr", "st", "mt", "vs", "no", "nos", "fig", "figs",
    "vol", "p", "pp", "ch", "sec", "eq", "approx", "cf", "viz", "gen", "gov", "sen", "rep", "rev",
    "capt", "lt", "col", "sgt", "hon",
];

/// Whether `core` (a word without surrounding punctuation or its final dot)
/// is an initialism such as `U.S`, `e.g`, `a.m` or a single initial `J`.
fn is_initialism(core: &str) -> bool {
    let mut parts = 0;
    for part in core.split('.') {
        let mut chars = part.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) if c.is_alphabetic() => parts += 1,
            _ => return false,
        }
    }
    parts > 1 || core.chars().next().is_some_and(char::is_uppercase)
}

/// Pause classes a word's trailing punctuation implies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ending {
    None,
    Clause,
    Terminal,
}

/// Classifies the punctuation at the end of `word`.
pub(crate) fn ending(word: &str) -> Ending {
    let bare = word.trim_end_matches(is_closer);
    let closers = &word[bare.len()..];
    match bare.chars().next_back() {
        Some('.' | '!' | '?' | '\u{2026}' | '\u{203C}' | '\u{2047}' | '\u{2048}' | '\u{2049}') => {
            Ending::Terminal
        }
        Some(',' | ';' | ':' | '\u{2014}' | '\u{2013}') => Ending::Clause,
        _ if closers.contains([')', ']']) => Ending::Clause,
        _ if !bare.is_empty() && bare.chars().all(|c| c == '-') => Ending::Clause,
        _ => Ending::None,
    }
}

/// Whether `word`, followed by `next` (if any) in the same block, ends a
/// sentence. `first` is true for the first word of a block. See the crate
/// docs for the rules and their known gaps.
pub(crate) fn ends_sentence(word: &str, next: Option<&str>, first: bool) -> bool {
    let Some(next) = next else {
        return true;
    };
    if ending(word) != Ending::Terminal {
        return false;
    }
    let core = word
        .trim_start_matches(is_opener)
        .trim_end_matches(is_closer);
    if core.ends_with('.') && !core.ends_with("..") {
        let stem = &core[..core.len() - 1];
        if is_initialism(stem) {
            return false;
        }
        if NON_TERMINAL_ABBREVIATIONS
            .iter()
            .any(|a| a.eq_ignore_ascii_case(stem))
        {
            return false;
        }
        if first && !stem.is_empty() && stem.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }
    }
    let next_lead = next.chars().find(|c| c.is_alphanumeric());
    !next_lead.is_some_and(char::is_lowercase)
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use std::vec::Vec;

    fn pivot_str(w: &str) -> &str {
        let (s, e, _) = pivot(w);
        &w[s..e]
    }

    #[test]
    fn pivot_by_length() {
        assert_eq!(pivot_str("a"), "a");
        assert_eq!(pivot_str("an"), "n");
        assert_eq!(pivot_str("hello"), "e");
        assert_eq!(pivot_str("reader"), "a");
        assert_eq!(pivot_str("wonderful"), "n");
        assert_eq!(pivot_str("absolutely"), "o");
        assert_eq!(pivot_str("extraordinarily"), "a");
    }

    #[test]
    fn pivot_ignores_leading_punctuation() {
        assert_eq!(pivot("(hello,"), (2, 3, 5));
        assert_eq!(pivot_str("\u{201C}Quote\u{201D}"), "u");
        assert_eq!(pivot_str("—"), "—");
        assert_eq!(pivot_str("..."), ".");
    }

    #[test]
    fn pivot_is_grapheme_aware() {
        assert_eq!(pivot_str("e\u{301}te\u{301}"), "t");
        assert_eq!(pivot_str("ne\u{301}"), "e\u{301}");
        assert_eq!(pivot_str("👍🏽"), "👍🏽");
        assert_eq!(pivot_str("👨‍👩‍👧x"), "x");
    }

    fn pieces(w: &str, max: usize) -> Vec<&str> {
        let mut out = Vec::new();
        split_long(w, max, |s, e| out.push(&w[s..e]));
        out
    }

    #[test]
    fn short_words_are_not_split() {
        assert_eq!(pieces("hello", 5), ["hello"]);
        assert_eq!(pieces("hello", 0), ["hello"]);
    }

    #[test]
    fn long_words_split_after_break_characters() {
        assert_eq!(
            pieces("https://example.com/some/long/path", 12),
            ["https://", "example.com/", "some/long/", "path"]
        );
        assert_eq!(
            pieces("state-of-the-art-technology", 10),
            ["state-of-", "the-art-", "technology"]
        );
    }

    #[test]
    fn unbreakable_runs_are_cut_at_max_width() {
        assert_eq!(pieces("abcdefghij", 4), ["abcd", "efgh", "ij"]);
        assert_eq!(pieces("ab-cdefghijkl", 4), ["ab-", "cdef", "ghij", "kl"]);
    }

    #[test]
    fn sentence_ends() {
        assert!(ends_sentence("end.", Some("Next"), false));
        assert!(ends_sentence("end.\u{201D}", Some("Next"), false));
        assert!(ends_sentence("(end.)", Some("Next"), false));
        assert!(ends_sentence("why?", Some("Because"), false));
        assert!(!ends_sentence("why?\"", Some("she"), false));
        assert!(!ends_sentence("Dr.", Some("Smith"), false));
        assert!(!ends_sentence("e.g.", Some("Paris"), false));
        assert!(!ends_sentence("U.S.", Some("Army"), false));
        assert!(!ends_sentence("3.14", Some("Then"), false));
        assert!(!ends_sentence("wait...", Some("what"), false));
        assert!(ends_sentence("wait...", Some("What"), false));
        assert!(ends_sentence("wait\u{2026}", Some("What"), false));
        assert!(!ends_sentence("1.", Some("First"), true));
        assert!(ends_sentence("anything", None, false));
    }

    #[test]
    fn endings() {
        assert_eq!(ending("word,"), Ending::Clause);
        assert_eq!(ending("word;"), Ending::Clause);
        assert_eq!(ending("word—"), Ending::Clause);
        assert_eq!(ending("(aside)"), Ending::Clause);
        assert_eq!(ending("--"), Ending::Clause);
        assert_eq!(ending("word.\""), Ending::Terminal);
        assert_eq!(ending("word"), Ending::None);
        assert_eq!(ending("re-read"), Ending::None);
    }
}
