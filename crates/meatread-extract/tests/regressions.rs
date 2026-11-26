//! Regression tests for extraction rules that once dropped, mutilated or
//! mislabeled article text. Each test builds a small page that exercises one
//! rule.

mod common;

use common::*;
use meatread_extract::extract_html;

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
