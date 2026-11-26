//! Format dispatch, decoding, Markdown, plain text and PDF.

mod common;

use common::*;
use meatread_core::{BlockKind, Source};
use meatread_extract::{ExtractError, extract, extract_html, extract_plain};

const URL: &str = "https://example.com/post";

/// An article-sized HTML page around `body_text`, so main-content detection
/// has enough to work with.
fn page(head: &str, body_text: &str) -> String {
    let filler = "The quick brown fox jumps over the lazy dog while the editor reads every word carefully, \
and the reader follows along one word at a time without any summary in between. ";
    format!(
        "<!doctype html><html><head>{head}</head><body><article><h1>Heading</h1><p>{body_text} {filler}{filler}{filler}</p>\
<p>{filler}{filler}</p><p>{filler}{filler}</p></article></body></html>"
    )
}

// ------------------------------------------------------------- encodings

/// Encodes `s` as windows-1252, for the characters these tests use.
fn windows_1252(s: &str) -> Vec<u8> {
    s.chars()
        .map(|c| match c {
            '\u{201c}' => 0x93,
            '\u{201d}' => 0x94,
            '\u{2019}' => 0x92,
            '\u{2013}' => 0x96,
            c if (c as u32) < 0x100 => c as u32 as u8,
            c => panic!("not windows-1252: {c}"),
        })
        .collect()
}

const CP1252_TEXT: &str = "\u{201c}Curly quotes\u{201d} and Ren\u{e9}e\u{2019}s caf\u{e9} \u{2013} Zo\u{eb} Fran\u{e7}ois.";

fn assert_cp1252_decoded(doc: &meatread_core::Document) {
    let all = all_text(doc);
    assert!(all.contains("\"Curly quotes\" and Renée's café"), "{all}");
    let raw: String = doc
        .blocks
        .iter()
        .map(|b| b.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(raw.contains("\u{201c}Curly quotes\u{201d} and Ren\u{e9}e\u{2019}s caf\u{e9} \u{2013} Zo\u{eb} Fran\u{e7}ois."));
    assert!(!raw.contains('\u{fffd}'), "no replacement characters");
    assert_eq!(doc.title, "Caf\u{e9} de Ren\u{e9}e");
}

#[test]
fn windows_1252_page_declared_in_meta_charset() {
    let html = page(
        "<meta charset=\"windows-1252\"><title>Caf\u{e9} de Ren\u{e9}e</title>",
        CP1252_TEXT,
    );
    let bytes = windows_1252(&html);
    assert!(
        std::str::from_utf8(&bytes).is_err(),
        "the fixture must not be valid UTF-8"
    );
    let doc = extract(&bytes, None, URL).unwrap();
    assert_cp1252_decoded(&doc);
}

#[test]
fn windows_1252_page_declared_in_http_equiv() {
    let html = page(
        "<meta http-equiv=\"Content-Type\" content=\"text/html; charset=windows-1252\"><title>Caf\u{e9} de Ren\u{e9}e</title>",
        CP1252_TEXT,
    );
    let doc = extract(&windows_1252(&html), Some("text/html"), URL).unwrap();
    assert_cp1252_decoded(&doc);
}

#[test]
fn header_charset_wins_over_meta() {
    let html = page(
        "<meta charset=\"utf-8\"><title>Caf\u{e9} de Ren\u{e9}e</title>",
        CP1252_TEXT,
    );
    let doc = extract(
        &windows_1252(&html),
        Some("text/html; charset=\"ISO-8859-1\""),
        URL,
    )
    .unwrap();
    // ISO-8859-1 is windows-1252 for web content.
    assert_cp1252_decoded(&doc);
}

#[test]
fn utf8_bom_beats_a_wrong_header_and_utf16_is_decoded() {
    let html = page("<title>Caf\u{e9} de Ren\u{e9}e</title>", CP1252_TEXT);
    let mut bytes = vec![0xef, 0xbb, 0xbf];
    bytes.extend_from_slice(html.as_bytes());
    let doc = extract(&bytes, Some("text/html; charset=windows-1252"), URL).unwrap();
    assert_cp1252_decoded(&doc);

    let mut utf16 = vec![0xff, 0xfe];
    for unit in html.encode_utf16() {
        utf16.extend_from_slice(&unit.to_le_bytes());
    }
    let doc = extract(&utf16, Some("text/html"), URL).unwrap();
    assert_cp1252_decoded(&doc);
}

#[test]
fn undeclared_pages_are_utf8_and_declared_utf8_replaces_invalid_bytes() {
    let html = page("<title>Plain</title>", "Ren\u{e9}e \u{4f60}\u{597d}");
    let doc = extract(html.as_bytes(), None, URL).unwrap();
    assert!(all_text(&doc).contains("Renée 你好"));

    let html = page("<meta charset=\"utf-8\"><title>Plain</title>", "Ren\u{e9}e");
    let mut bytes = html.into_bytes();
    let at = bytes.windows(3).position(|w| w == b"The").unwrap();
    bytes[at] = 0xff;
    let doc = extract(&bytes, Some("text/html"), URL).unwrap();
    assert!(doc.blocks.iter().any(|b| b.text.contains('\u{fffd}')));
}

#[test]
fn plain_text_honors_header_charset() {
    let body = format!("{CP1252_TEXT}\n\nSecond paragraph.");
    let doc = extract(
        &windows_1252(&body),
        Some("text/plain; charset=windows-1252"),
        URL,
    )
    .unwrap();
    assert!(doc.blocks[0].text.contains("Ren\u{e9}e\u{2019}s caf\u{e9}"));
}

// -------------------------------------------------------------- dispatch

#[test]
fn dispatch_by_content_type_url_and_sniffing() {
    let html = page("<title>T</title>", "Hello");
    assert_eq!(
        extract(html.as_bytes(), Some("text/html"), URL)
            .unwrap()
            .source,
        Source::Readability
    );
    // Mislabelled as plain text: the bytes say HTML.
    assert_eq!(
        extract(html.as_bytes(), Some("text/plain"), URL)
            .unwrap()
            .source,
        Source::Readability
    );
    assert_eq!(
        extract(html.as_bytes(), None, "").unwrap().source,
        Source::Readability
    );

    let md = b"# Title\n\nSome *text* here.\n";
    let markdown = if cfg!(feature = "markdown") {
        Source::Markdown
    } else {
        Source::PlainText
    };
    assert_eq!(
        extract(md, Some("text/markdown"), URL).unwrap().source,
        markdown
    );
    assert_eq!(
        extract(md, Some("text/plain"), "https://x.test/README.md?raw=1")
            .unwrap()
            .source,
        markdown
    );
    assert_eq!(
        extract(md, None, "https://x.test/a/b.markdown#top")
            .unwrap()
            .source,
        markdown
    );
    assert_eq!(
        extract(md, Some("text/plain"), URL).unwrap().source,
        Source::PlainText
    );
    assert_eq!(
        extract(b"Just words.\n", Some("application/octet-stream"), URL)
            .unwrap()
            .source,
        Source::PlainText
    );
}

#[test]
fn binary_types_and_empty_input_are_errors() {
    assert_eq!(
        extract(b"\x89PNG....", Some("image/png"), URL),
        Err(ExtractError::UnsupportedType("image/png".to_string()))
    );
    assert_eq!(extract(b"", None, URL), Err(ExtractError::NoContent));
    assert_eq!(
        extract(b"  \n\n \t", Some("text/plain"), URL),
        Err(ExtractError::NoContent)
    );
    assert_eq!(
        extract_html("<html><body></body></html>", URL),
        Err(ExtractError::NoContent)
    );
}

// ----------------------------------------------------------- plain text

#[test]
fn plain_text_paragraphs_and_title() {
    let doc = extract_plain(
        "A Short Title\n\nFirst paragraph wraps\nacross two lines.\n\n\n  \nSecond paragraph.\n",
        URL,
    )
    .unwrap();
    assert_eq!(doc.title, "A Short Title");
    assert_eq!(doc.source, Source::PlainText);
    let texts: Vec<_> = doc.blocks.iter().map(|b| b.text.as_str()).collect();
    assert_eq!(
        texts,
        [
            "A Short Title",
            "First paragraph wraps across two lines.",
            "Second paragraph."
        ]
    );
    assert_eq!(doc.word_count, 3 + 6 + 2);
    assert!(!doc.partial);

    let no_title =
        extract_plain("This is a full sentence that ends.\n\nAnother one.", URL).unwrap();
    assert_eq!(no_title.title, "");
}

// -------------------------------------------------------------- markdown

#[cfg(feature = "markdown")]
mod markdown {
    use super::*;
    use meatread_extract::extract_markdown;

    const MD: &str = "---\ntitle: \"Front Matter Title\"\nauthor: Ada Lovelace\n---\n\
# Top Heading\n\n\
Intro with **bold**, *italic*, `code`, a [link](https://example.com/x) and an escaped \\* star &amp; entity.\n\n\
## Second\n\n\
- one\n- two\n  - nested\n\n\
1. first\n2. second\n\n\
> quoted line\n>\n> > deeper\n\n\
```rust\nfn main() {\n    println!(\"hi\");\n}\n```\n\n\
\x20\x20\x20\x20indented code\n\n\
| a | b |\n|---|---|\n| 1 | 2 |\n\n\
---\n\n\
![alt text](pic.png)\n\n\
Footnote ref[^1].\n\n\
[^1]: The note text.\n";

    #[test]
    fn maps_syntax_onto_block_kinds() {
        let doc = extract_markdown(MD, URL).unwrap();
        assert_eq!(doc.title, "Front Matter Title");
        assert_eq!(doc.byline.as_deref(), Some("Ada Lovelace"));
        assert_eq!(doc.source, Source::Markdown);
        assert!(!doc.partial);

        let b = &doc.blocks;
        assert_eq!(
            (b[0].kind, b[0].level, b[0].text.as_str()),
            (BlockKind::Heading, Some(1), "Top Heading")
        );
        assert_eq!(b[1].kind, BlockKind::Paragraph);
        assert_eq!(
            b[1].text,
            "Intro with bold, italic, code, a link and an escaped * star & entity."
        );
        assert_eq!((b[2].kind, b[2].level), (BlockKind::Heading, Some(2)));

        let items: Vec<_> = b.iter().filter(|x| x.kind == BlockKind::ListItem).collect();
        let shape: Vec<_> = items
            .iter()
            .map(|x| (x.text.as_str(), x.depth, x.ordered))
            .collect();
        assert_eq!(
            shape,
            [
                ("one", Some(1), Some(false)),
                ("two", Some(1), Some(false)),
                ("nested", Some(2), Some(false)),
                ("first", Some(1), Some(true)),
                ("second", Some(1), Some(true)),
            ]
        );

        let quotes: Vec<_> = b
            .iter()
            .filter(|x| x.kind == BlockKind::Blockquote)
            .map(|x| (x.text.as_str(), x.quote))
            .collect();
        assert_eq!(quotes, [("quoted line", 1), ("deeper", 2)]);

        let code: Vec<_> = b.iter().filter(|x| x.kind == BlockKind::Code).collect();
        assert_eq!(code.len(), 2);
        assert_eq!(code[0].lang.as_deref(), Some("rust"));
        assert_eq!(code[0].text, "fn main() {\n    println!(\"hi\");\n}");
        assert_eq!(code[1].lang, None);
        assert_eq!(code[1].text, "indented code");
        assert!(code.iter().all(|c| c.skip));

        let table = b.iter().find(|x| x.kind == BlockKind::Table).unwrap();
        assert_eq!(table.text, "a\tb\n1\t2");
        assert!(b.iter().any(|x| x.kind == BlockKind::Rule));
        assert!(
            b.iter()
                .any(|x| x.kind == BlockKind::Figure && x.text == "alt text")
        );
        let note = b.iter().find(|x| x.kind == BlockKind::Footnote).unwrap();
        assert_eq!(note.text, "The note text.");
        let fnref = b
            .iter()
            .find(|x| x.text.starts_with("Footnote ref"))
            .unwrap();
        assert_eq!(fnref.text, "Footnote ref.");
    }

    #[test]
    fn title_falls_back_to_first_h1_and_html_is_reduced_to_text() {
        let doc = extract_markdown(
            "# Plain Title\n\n<div>Raw <b>html</b> &amp; text</div>\n\nBody.\n",
            URL,
        )
        .unwrap();
        assert_eq!(doc.title, "Plain Title");
        assert!(doc.blocks.iter().any(|b| b.text == "Raw html & text"));
    }

    #[test]
    fn words_are_not_smartened() {
        let doc = extract_markdown("He said \"it's -- fine...\"\n", URL).unwrap();
        assert_eq!(doc.blocks[0].text, "He said \"it's -- fine...\"");
    }

    #[test]
    fn empty_markdown_is_an_error() {
        assert_eq!(extract_markdown("\n\n", URL), Err(ExtractError::NoContent));
        assert_eq!(
            extract_markdown("<!-- only a comment -->\n", URL),
            Err(ExtractError::NoContent)
        );
    }

    #[test]
    fn words_survive_markup_removal() {
        // Every word the reader sees is a word of the source once markup
        // punctuation is ignored.
        let doc = extract_markdown(MD, URL).unwrap();
        let source: String = MD.to_string();
        for b in doc.blocks.iter().filter(|b| b.kind == BlockKind::Paragraph) {
            for w in b.text.split_whitespace() {
                let core = w.trim_matches(|c: char| !c.is_alphanumeric());
                assert!(core.is_empty() || source.contains(core), "{w}");
            }
        }
    }
}

// ------------------------------------------------------------------- pdf

#[cfg(feature = "pdf")]
mod pdf {
    use super::*;
    use meatread_extract::extract_pdf;

    fn sample() -> Vec<u8> {
        std::fs::read(fixtures_dir().join("attention.pdf")).unwrap()
    }

    #[test]
    fn sample_paper() {
        let doc = extract(
            &sample(),
            Some("application/pdf"),
            "https://arxiv.org/pdf/1706.03762",
        )
        .unwrap();
        assert_eq!(doc.source, Source::Pdf);
        assert_eq!(doc.title, "Attention Is All You Need");
        assert!(!doc.partial);
        assert!(doc.word_count > 5000, "{}", doc.word_count);

        let headings: Vec<_> = doc
            .blocks
            .iter()
            .filter(|b| b.kind == BlockKind::Heading)
            .map(|b| b.text.as_str())
            .collect();
        for expected in ["Abstract", "3.2 Attention", "6 Results", "References"] {
            assert!(headings.contains(&expected), "{expected} in {headings:?}");
        }
        assert!(
            doc.blocks
                .iter()
                .any(|b| b.level == Some(1) && b.text == doc.title)
        );

        // Wrapped lines are joined; no block holds a hard line break.
        assert!(doc.blocks.iter().all(|b| !b.text.contains('\n')));
        let abstract_at = doc
            .blocks
            .iter()
            .position(|b| b.text == "Abstract")
            .unwrap();
        assert!(
            doc.blocks[abstract_at + 1]
                .text
                .starts_with("The dominant sequence transduction models")
        );
        assert!(
            doc.blocks[abstract_at + 1]
                .text
                .contains("Our model achieves 28.4 BLEU on the WMT 2014 English-")
        );
    }

    #[test]
    fn end_of_line_hyphens_are_kept_not_guessed() {
        let doc = extract_pdf(&sample(), "").unwrap();
        let text = doc
            .blocks
            .iter()
            .map(|b| b.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        // "transduc-" ends a printed line and "tion" starts the next.
        assert!(
            text.contains("transduc- tion models"),
            "hyphenation must not be undone"
        );
        assert!(
            !text.contains("transduction models in various"),
            "sanity: the join is by space"
        );
    }

    #[test]
    fn page_numbers_are_dropped() {
        let doc = extract_pdf(&sample(), "").unwrap();
        assert!(
            doc.blocks
                .iter()
                .all(|b| !(b.text.len() <= 4 && b.text.chars().all(|c| c.is_ascii_digit())))
        );
    }

    #[test]
    fn bad_pdfs_are_errors() {
        assert!(matches!(
            extract(b"%PDF-1.4 not really a pdf", None, URL),
            Err(ExtractError::Pdf(_))
        ));
        assert!(matches!(extract_pdf(b"", URL), Err(ExtractError::Pdf(_))));
        let mut truncated = sample();
        truncated.truncate(2000);
        assert!(matches!(
            extract_pdf(&truncated, URL),
            Err(ExtractError::Pdf(_))
        ));
    }

    #[test]
    fn pdf_scripts_and_plays() {
        use meatread_core::{Pacing, PlayState, Player, Script};
        let doc = extract_pdf(&sample(), "").unwrap();
        let mut player = Player::new(Script::new(&doc), Pacing::default());
        player.play();
        while player.state() == PlayState::Playing {
            let wait = player.next_change_in().unwrap();
            player.advance(wait);
        }
        assert_eq!(player.state(), PlayState::Finished);
    }
}

#[cfg(not(feature = "pdf"))]
#[test]
fn pdf_without_feature_reports_it() {
    assert_eq!(
        extract(b"%PDF-1.4", None, URL),
        Err(ExtractError::FeatureDisabled("pdf"))
    );
}
