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

// ------------------------------------------------------------------ Markdown

const HTML_BLOCK_README: &str = "<div align=\"center\">
  <picture>
    <source media=\"(prefers-color-scheme: dark)\" srcset=\"https://example.com/dark.svg\">
    <img alt=\"The Project: a tagline\"
         src=\"https://example.com/light.svg\"
         width=\"50%\">
  </picture>
</div>

This is the main repository for the project. It holds the code.

<h2 align=\"center\">Why this project</h2>

<p align=\"center\">Fast, <a href=\"https://example.com/docs\">documented</a> and small.</p>
";

#[test]
fn multi_line_html_blocks_in_markdown_never_leak_tags_or_attributes() {
    let doc = extract(
        HTML_BLOCK_README.as_bytes(),
        Some("text/markdown"),
        "https://example.com/README.md",
    )
    .unwrap();
    let all = all_text(&doc);
    for leaked in [
        "src=",
        "srcset",
        "width",
        "https://example.com/",
        "<",
        "align",
    ] {
        assert!(!all.contains(leaked), "{leaked:?} leaked: {all}");
    }
    let figure = doc
        .blocks
        .iter()
        .find(|b| b.kind == BlockKind::Figure)
        .expect("the image's alt text is kept as a figure");
    assert_eq!(figure.text, "The Project: a tagline");
    assert!(figure.skip);
    assert!(
        doc.blocks
            .iter()
            .any(|b| b.kind == BlockKind::Heading && b.text == "Why this project")
    );
    assert!(
        doc.blocks
            .iter()
            .any(|b| b.text == "Fast, documented and small.")
    );
    assert_eq!(
        readable(&doc).next().unwrap().text,
        "This is the main repository for the project. It holds the code."
    );
}

#[test]
fn markdown_title_falls_back_to_a_heading_then_the_file_name() {
    let doc = extract(
        b"<h1 align=\"center\">Project Name</h1>\n\nBody text of the readme.\n",
        Some("text/markdown"),
        "https://example.com/README.md",
    )
    .unwrap();
    assert_eq!(doc.title, "Project Name");

    let doc = extract(
        b"Intro words.\n\n## A Section\n\nMore words.\n",
        Some("text/markdown"),
        "https://example.com/docs/GUIDE.md?raw=1",
    )
    .unwrap();
    assert_eq!(doc.title, "A Section");

    let doc = extract(
        b"Only words here, no headings at all.\n",
        Some("text/markdown"),
        "https://example.com/docs/GUIDE.md?raw=1",
    )
    .unwrap();
    assert_eq!(doc.title, "GUIDE");

    let doc = extract(b"Only words.\n", Some("text/markdown"), "").unwrap();
    assert_eq!(doc.title, "");
}

// ------------------------------------------------------------------ paywalls

/// An article of roughly `paragraphs` x 30 words with `extra` appended to the
/// page.
fn long_page(paragraphs: usize, head: &str, extra: &str) -> meatread_core::Document {
    let body: String = (0..paragraphs)
        .map(|_| format!("<p>{FILLER}</p>"))
        .collect();
    let html = format!(
        "<!doctype html><html><head><title>T</title>{head}</head><body><article><h1>T</h1>{body}{extra}</article></body></html>"
    );
    extract_html(&html, URL).unwrap()
}

#[test]
fn an_empty_or_hidden_paywall_wrapper_does_not_make_a_full_article_partial() {
    for extra in [
        "<div class=\"paywall\"></div>",
        "<div class=\"paywall\" hidden>Subscribe to read more</div>",
        "<div style=\"display: none\"><div class=\"paywall\">Subscribe to read more</div></div>",
        "<div class=\"paywall\" aria-hidden=\"true\">Subscribe to read more</div>",
    ] {
        let doc = long_page(12, "", extra);
        assert!(
            doc.word_count > 250 && doc.word_count < 400,
            "{}",
            doc.word_count
        );
        assert!(!doc.partial, "{extra}");
        assert!(doc.paywall_hint, "{extra}: the signal is still reported");
    }
}

#[test]
fn a_visible_gate_corroborated_by_a_short_or_shorter_than_declared_body_is_partial() {
    let gate = "<div class=\"paywall\"><p>Subscribe to keep reading.</p></div>";
    let short = long_page(12, "", gate);
    assert!(short.partial && short.paywall_hint);

    let long_undeclared = long_page(20, "", gate);
    assert!(long_undeclared.word_count >= 400);
    assert!(!long_undeclared.partial);
    assert!(long_undeclared.paywall_hint);

    let declared =
        "<script type=\"application/ld+json\">{\"@type\":\"Article\",\"wordCount\":2500}</script>";
    let long_declared = long_page(20, declared, gate);
    assert!(long_declared.partial && long_declared.paywall_hint);
}

#[test]
fn a_visible_subscription_cut_marks_even_a_long_body_partial() {
    let doc = long_page(
        20,
        "",
        "<div class=\"post-access-cta\"><p>Subscribe to continue reading.</p></div>",
    );
    assert!(doc.word_count >= 400);
    assert!(doc.partial && doc.paywall_hint);
}

// --------------------------------------------------------- hostile nesting

fn nested_page(open: &str, close: &str, depth: usize, middle: &str) -> String {
    format!(
        "<!doctype html><html><head><title>T</title></head><body><article><h1>T</h1>\
<p>{FILLER} {FILLER}</p><p>{FILLER} {FILLER}</p>{}{middle}{}</article></body></html>",
        open.repeat(depth),
        close.repeat(depth)
    )
}

#[test]
fn absurd_nesting_is_refused_with_an_error_instead_of_overflowing() {
    for (open, close, depth) in [
        ("<span>", "</span>", 50_000),
        ("<div>", "</div>", 50_000),
        ("<li>item<ul>", "</ul></li>", 5_000),
        ("<b>", "", 50_000),
    ] {
        let html = nested_page(open, close, depth, "<p>deep words</p>");
        let err = extract_html(&html, URL).unwrap_err();
        assert!(
            matches!(err, ExtractError::TooDeeplyNested { .. }),
            "{open:?}: {err:?}"
        );
        assert!(err.to_string().contains("levels deep"));
        let err = extract(html.as_bytes(), Some("text/html"), URL).unwrap_err();
        assert!(matches!(err, ExtractError::TooDeeplyNested { .. }));
    }
}

#[test]
fn deep_but_plausible_nesting_still_extracts_on_a_small_stack() {
    let run = |open: &'static str, close: &'static str, depth: usize| {
        let html = nested_page(open, close, depth, "<p>deep words stay</p>");
        std::thread::Builder::new()
            .stack_size(1 << 20)
            .spawn(move || extract_html(&html, URL))
            .unwrap()
            .join()
            .expect("no stack overflow")
    };
    for (open, close, depth) in [
        ("<span>", "</span>", 2_400),
        ("<div>", "</div>", 2_400),
        ("<li>x<ul>", "</ul></li>", 1_200),
    ] {
        let doc = run(open, close, depth).unwrap();
        assert!(reading_text(&doc).contains("deep words stay"), "{open}");
    }
}

#[test]
fn flat_runs_of_optional_end_tags_and_svg_paths_are_not_deep() {
    let items: String = (0..5_000).map(|i| format!("<li>item number {i}")).collect();
    let paths = "<path d=\"M0 0\"/>".repeat(5_000);
    let html = nested_page("", "", 0, &format!("<ul>{items}</ul><svg>{paths}</svg>"));
    let doc = extract_html(&html, URL).unwrap();
    assert!(reading_text(&doc).contains("item number 4999"));
}

#[test]
fn markdown_with_an_absurdly_nested_html_block_is_refused() {
    let md = format!(
        "{}x{}\n\nText.\n",
        "<div>".repeat(50_000),
        "</div>".repeat(50_000)
    );
    let err = extract(md.as_bytes(), Some("text/markdown"), "").unwrap_err();
    assert!(matches!(err, ExtractError::TooDeeplyNested { .. }));
}

// ------------------------------------------------- other drop rules (audit)

#[test]
fn a_button_inside_a_sentence_keeps_its_words_but_a_lone_control_is_dropped() {
    let doc = html_doc(
        "",
        "<p>Please <button class=\"linklike\">click here</button> to continue reading the story right now.</p>\
<div><button>Share this</button></div>",
    );
    let text = reading_text(&doc);
    assert!(
        text.contains("Please click here to continue reading the story right now."),
        "{text}"
    );
    assert!(!text.contains("Share this"));
}

#[test]
fn a_form_wrapping_article_paragraphs_is_read_but_a_small_form_is_dropped() {
    let long = format!("<p>{FILLER} {FILLER}</p>");
    let doc = html_doc(
        "",
        &format!(
            "<form id=\"aspnetForm\">{long}</form><form><label>Email</label><input name=\"e\"><button>Join</button></form>"
        ),
    );
    let text = reading_text(&doc);
    assert!(text.matches("The quick brown fox").count() >= 4, "{text}");
    assert!(!text.contains("Email"));
}

#[test]
fn only_short_footnote_marker_links_are_dropped() {
    let doc = html_doc(
        "",
        "<p>The claim was disputed<sup><a href=\"#fn1\">12</a></sup> and then the 4<sup><a href=\"#top\">th party reports</a></sup> repeated it again later on.</p>",
    );
    assert!(
        reading_text(&doc).contains(
            "The claim was disputed and then the 4th party reports repeated it again later on."
        ),
        "{}",
        reading_text(&doc)
    );
}

#[test]
fn a_figure_holding_paragraphs_is_read() {
    let doc = html_doc(
        "",
        "<figure><p>Roses are red and violets are blue and this is a poem.</p><figcaption>By A. Poet</figcaption></figure>",
    );
    assert!(reading_text(&doc).contains("Roses are red and violets are blue and this is a poem."));
    let caption = doc.blocks.iter().find(|b| b.text == "By A. Poet").unwrap();
    assert_eq!(caption.kind, BlockKind::Caption);
}

#[test]
fn a_paragraph_that_reads_like_a_timestamp_is_kept_unless_it_is_a_link() {
    let doc = html_doc(
        "",
        "<p>It was a long time. Everything changed.</p><p>10 years ago</p><p>The story continues after the gap in time.</p>\
<p>The end of the piece is near now.</p><p>3 years ago</p>",
    );
    let t = texts(&doc);
    assert_eq!(t.last().unwrap(), "3 years ago");
    assert!(t.contains(&"10 years ago".to_string()));

    let doc = html_doc(
        "",
        "<p>The end of the piece is near now.</p><p><a href=\"/c\">108 Comments</a></p>",
    );
    assert_eq!(
        texts(&doc).last().unwrap(),
        "The end of the piece is near now."
    );
}

#[test]
fn an_advertisement_label_is_dropped_only_inside_an_ad_slot() {
    let doc = html_doc(
        "",
        "<div class=\"ad ad-leaderboard\"><div class=\"ad__title\">Advertisement</div></div>\
<p>Advertisement</p><p>The article carries on with more words after that line.</p>",
    );
    let t = texts(&doc);
    assert_eq!(t.iter().filter(|p| *p == "Advertisement").count(), 1);
}
