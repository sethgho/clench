//! Regression tests for extraction rules that once dropped, mutilated or
//! mislabeled article text. Each test builds a small page that exercises one
//! rule.

mod common;

use common::*;
use meatread_core::{BlockKind, Source};
use meatread_extract::{ExtractError, extract, extract_html};

const URL: &str = "https://example.com/post";

const FILLER: &str = "The quick brown fox jumps over the lazy dog while the editor reads every word carefully, \
and the reader follows along one word at a time without any summary in between.";

/// An article page: a heading, `lead` filler paragraphs, then `tail` as raw
/// HTML inside the article element.
fn article_page(head: &str, tail: &str) -> String {
    format!(
        "<!doctype html><html><head><title>A Post</title>{head}</head><body><article>\
<h1>A Post</h1><p>{FILLER} {FILLER}</p><p>{FILLER} {FILLER}</p><p>{FILLER} {FILLER}</p>{tail}\
</article></body></html>"
    )
}

fn html_doc(head: &str, tail: &str) -> meatread_core::Document {
    extract_html(&article_page(head, tail), URL).expect("extracts")
}

fn texts(doc: &meatread_core::Document) -> Vec<String> {
    readable(doc).map(|b| b.text.clone()).collect()
}

// ------------------------------------------------------ closing paragraphs

#[test]
fn closing_paragraphs_that_mention_subscribing_are_article_text() {
    let doc = html_doc(
        "",
        "<p>It all started when a friend dared me, so I decided to sign up for a marathon.</p>\
<p>If you take one thing from this, I'd encourage you to subscribe to the idea too.</p>",
    );
    let t = texts(&doc);
    assert_eq!(
        t.last().unwrap(),
        "If you take one thing from this, I'd encourage you to subscribe to the idea too."
    );
    assert!(t.iter().any(|p| p.ends_with("sign up for a marathon.")));
    assert!(!doc.partial);
}

#[test]
fn an_italic_linked_closing_promo_is_still_trimmed() {
    let doc = html_doc(
        "",
        "<p>The last real sentence of the article stands here.</p>\
<p><em>Jane Doe is a reporter. <a href=\"/news\">Subscribe to the newsletter</a> for more.</em></p>\
<p><em>This story originally appeared on <a href=\"/orig\">Example</a>.</em></p>",
    );
    let t = texts(&doc);
    assert_eq!(
        t.last().unwrap(),
        "The last real sentence of the article stands here."
    );
}

// ------------------------------------------------------------------- <time>

#[test]
fn a_time_element_inside_a_sentence_keeps_its_text() {
    let doc = html_doc(
        "",
        "<p>The deadline was <strong><time datetime=\"2026-03-01\">March 1st</time></strong>, and we missed it.</p>",
    );
    assert!(
        texts(&doc).contains(&"The deadline was March 1st, and we missed it.".to_string()),
        "{:?}",
        texts(&doc)
    );
}

#[test]
fn a_lone_dateline_is_still_dropped() {
    let doc = html_doc(
        "",
        "<div class=\"meta\"><span><time datetime=\"2026-03-01\">March 1, 2026</time></span></div>",
    );
    assert!(!reading_text(&doc).contains("March 1, 2026"));
}

// ------------------------------------------------------ inline label classes

#[test]
fn an_overline_utility_class_on_inline_text_keeps_the_words() {
    let doc = html_doc(
        "",
        "<p>This is <span class=\"overline\">very important</span> text that we keep.</p>\
<p>Also <span class=\"kicker\">this part</span> and <span class=\"eyebrow\">that part</span> stay.</p>",
    );
    let t = texts(&doc);
    assert!(t.contains(&"This is very important text that we keep.".to_string()));
    assert!(t.contains(&"Also this part and that part stay.".to_string()));
}

#[test]
fn a_block_wrapped_in_a_label_utility_class_is_not_dropped() {
    let doc = html_doc(
        "",
        "<div class=\"overline\"><p>Wrapped paragraph one has plenty of words in it to be an article paragraph for sure.</p>\
<p>Wrapped paragraph two also has plenty of words in it to be an article paragraph too.</p></div>",
    );
    let text = reading_text(&doc);
    assert!(text.contains("Wrapped paragraph one"));
    assert!(text.contains("Wrapped paragraph two"));
}

#[test]
fn a_module_name_on_inline_text_keeps_the_words() {
    let doc = html_doc(
        "",
        "<p>The <span class=\"related\">closely related species</span> of this bird are all listed in the appendix at the back of the book.</p>",
    );
    assert!(reading_text(&doc).contains("The closely related species of this bird are all listed in the appendix at the back of the book."));
}

#[test]
fn readability_hook_classes_on_inline_text_do_not_remove_words() {
    let doc = html_doc(
        "",
        "<p>As <span class=\"author\">Jane Doe</span> wrote in her famous letter, the <a class=\"share\" href=\"/x\">shared meadow</a> and the <span id=\"comments\">quiet orchard</span> were never sold.</p>",
    );
    assert!(
        reading_text(&doc).contains(
            "As Jane Doe wrote in her famous letter, the shared meadow and the quiet orchard were never sold."
        ),
        "{}",
        reading_text(&doc)
    );
}

// --------------------------------------------------------- caption / credit

#[test]
fn a_container_with_a_caption_style_class_is_not_a_caption() {
    let body = format!("<p>{FILLER}</p>").repeat(6);
    let html = format!(
        "<!doctype html><html><head><title>T</title></head><body>\
<div class=\"entry-content has-caption-styles\"><h1>Title</h1>{body}</div></body></html>"
    );
    let doc = extract_html(&html, URL).expect("an article, not a caption");
    assert!(doc.word_count > 100, "{}", doc.word_count);
    assert!(doc.blocks.iter().all(|b| b.kind != BlockKind::Caption));
}

#[test]
fn a_span_with_a_caption_class_in_running_text_keeps_its_words() {
    let doc = html_doc(
        "",
        "<p>We saw a <span class=\"caption\">very large</span> bird land nearby.</p>",
    );
    assert!(reading_text(&doc).contains("We saw a very large bird land nearby."));
}

#[test]
fn a_real_credit_line_is_still_a_skipped_caption() {
    let doc = html_doc(
        "",
        "<p class=\"photo-credit\">Photo: Jane Doe</p><p>The article text continues here.</p>",
    );
    let credit = doc
        .blocks
        .iter()
        .find(|b| b.text == "Photo: Jane Doe")
        .expect("kept as a block");
    assert_eq!(credit.kind, BlockKind::Caption);
    assert!(credit.skip);
}

// -------------------------------------------------------- citation abstract

#[test]
fn a_citation_abstract_does_not_replace_a_real_body() {
    let head = "<meta name=\"citation_abstract\" content=\"A four word abstract.\">";
    let doc = html_doc(head, "");
    assert_ne!(doc.source, Source::CitationAbstract);
    assert!(doc.word_count > 100, "{}", doc.word_count);
}

#[test]
fn a_citation_abstract_page_without_a_body_reads_as_its_abstract() {
    let abstract_text = "We study the reading of long documents by people and find that \
they read them one word at a time when the words are shown at a fixed point on screen.";
    let html = format!(
        "<!doctype html><html><head><title>Paper</title>\
<meta name=\"citation_abstract\" content=\"{abstract_text}\">\
<meta name=\"citation_author\" content=\"Doe, Jane\"></head><body><nav>Menu</nav></body></html>"
    );
    let doc = extract_html(&html, URL).unwrap();
    assert_eq!(doc.source, Source::CitationAbstract);
    assert!(doc.partial);
    assert_eq!(doc.byline.as_deref(), Some("Doe, Jane"));
    assert_eq!(doc.blocks[0].text, abstract_text);
}

// ------------------------------------------------------------ format sniffing

#[test]
fn a_pdf_marker_inside_a_page_does_not_make_it_a_pdf() {
    let html = article_page(
        "",
        "<p>See the header <code>%PDF-1.7</code> for details.</p>",
    )
    .replace(
        "<title>A Post</title>",
        "<title>On %PDF-1.7 headers</title>",
    );
    let doc = extract(html.as_bytes(), Some("text/html"), URL).unwrap();
    assert!(doc.word_count > 50);

    let text = "Notes\n\nThe file begins with %PDF-1.4 and then the objects follow.";
    let doc = extract(text.as_bytes(), Some("text/plain"), "").unwrap();
    assert_eq!(doc.source, Source::PlainText);

    // Without a Content-Type the marker still counts only at the very start.
    let doc = extract(text.as_bytes(), None, "").unwrap();
    assert_eq!(doc.source, Source::PlainText);
}

#[test]
fn pdf_magic_at_the_start_is_still_detected() {
    for prefix in ["", "\u{feff}", "\n  "] {
        let bytes = format!("{prefix}%PDF-1.4\nnot really a pdf");
        let err = extract(bytes.as_bytes(), Some("text/plain"), "").unwrap_err();
        assert!(matches!(err, ExtractError::Pdf(_)), "{prefix:?}: {err:?}");
    }
}

// ----------------------------------------------------------------- encodings

#[test]
fn undeclared_non_utf8_bytes_are_read_as_windows_1252() {
    let html = article_page(
        "",
        "<p>Caf\u{e9} na\u{ef}ve and caf\u{e9} again for good measure here.</p>",
    );
    // Latin-1 range characters are the same single bytes in windows-1252.
    let bytes: Vec<u8> = html.chars().map(|c| c as u32 as u8).collect();
    assert!(std::str::from_utf8(&bytes).is_err());
    let doc = extract(&bytes, Some("text/html"), URL).unwrap();
    let text = reading_text(&doc);
    assert!(text.contains("Café naïve"), "{text}");
    assert!(!text.contains('\u{fffd}'));

    let doc = extract(&bytes, None, URL).unwrap();
    assert!(reading_text(&doc).contains("Café naïve"));
}

#[test]
fn utf8_cut_off_mid_character_is_still_utf8() {
    let html = article_page(
        "",
        "<p>Caf\u{e9} na\u{ef}ve \u{4f60}\u{597d} and the rest of this line.</p>",
    );
    let mut bytes = html.into_bytes();
    bytes.extend_from_slice(&"\u{4f60}".as_bytes()[..2]);
    let doc = extract(&bytes, Some("text/html"), URL).unwrap();
    assert!(reading_text(&doc).contains("Café naïve 你好"));
}

// ----------------------------------------------------------------- footnotes

#[test]
fn word_count_and_partial_ignore_footnotes_the_reader_skips() {
    let notes: String = (1..=12)
        .map(|n| format!("<li id=\"fn{n}\">Note {n}: {FILLER}</li>"))
        .collect();
    let html = format!(
        "<!doctype html><html><head><title>T</title></head><body><article><h1>T</h1>\
<p>{FILLER}</p><p>{FILLER}</p><ol class=\"footnotes\">{notes}</ol></article></body></html>"
    );
    let doc = extract_html(&html, URL).unwrap();
    assert!(doc.blocks.iter().any(|b| b.kind == BlockKind::Footnote));
    let played: usize = meatread_core::Script::new(&doc)
        .tokens()
        .iter()
        .filter(|t| !t.joined && !t.is_marker())
        .count();
    assert_eq!(doc.word_count, played);
    assert!(doc.word_count < 80, "{}", doc.word_count);
    assert!(
        doc.partial,
        "the readable body is too short to be the article"
    );
}

#[test]
fn markdown_word_count_matches_what_the_reader_plays() {
    let doc = extract(
        b"# T\n\nBody text here[^1] and more.\n\n[^1]: A long footnote with many words in it.\n",
        Some("text/markdown"),
        "",
    )
    .unwrap();
    let played = meatread_core::Script::new(&doc).tokens().len();
    assert_eq!(doc.word_count, played);
}
