//! Content and signals from outside the article body: JSON-LD structured
//! data, the meta description, and paywall markers.

use clench_core::{Block, BlockKind};
use dom_smoothie::dom_query::{Document as Dom, NodeRef};
use serde_json::Value;

use crate::text::{block, collapse, paragraphs};

/// Evidence that a page is gated behind a paywall.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct PaywallSignals {
    /// Structured data declares the content not freely accessible. Some free
    /// sites declare this too, so it is only trusted together with a short
    /// body.
    pub structured: bool,
    /// The page has an element named like a paywall or subscription gate,
    /// visible or not. Enough for a hint, not for calling the body partial.
    pub gate: bool,
    /// A visible, non-empty element named like a subscription prompt that
    /// replaces the rest of an article (`post-access-cta`): the text after it
    /// is missing.
    pub cut_gate: bool,
    /// A visible, non-empty element with a generic paywall class. Whether
    /// the article is partial then depends on how short the body is.
    pub shown_gate: bool,
    /// The article's word count as declared by structured data.
    pub declared_words: Option<usize>,
}

/// Class tokens of prompts that replace the remainder of a gated article.
const CUT_GATE_CLASSES: &[&str] = &["post-access-cta", "gh-post-upgrade-cta"];

/// Generic paywall class tokens, which also appear on empty or hidden
/// wrappers of free pages.
const GENERIC_GATE_CLASSES: &[&str] = &["paywall", "paywalled"];

/// Class tokens that hide an element.
const HIDING_CLASSES: &[&str] = &["hidden", "d-none", "is-hidden", "u-hidden", "hide"];

fn json_ld(page: &Dom) -> Vec<Value> {
    page.select(r#"script[type="application/ld+json"]"#)
        .nodes()
        .iter()
        .filter_map(|n| serde_json::from_str::<Value>(&n.text()).ok())
        .collect()
}

fn find_key<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    match v {
        Value::Object(m) => m
            .get(key)
            .or_else(|| m.values().find_map(|v| find_key(v, key))),
        Value::Array(a) => a.iter().find_map(|v| find_key(v, key)),
        _ => None,
    }
}

/// Whether an element and all its ancestors are displayed.
fn is_visible(n: &NodeRef) -> bool {
    std::iter::once(*n).chain(n.ancestors_it(None)).all(|a| {
        if !a.is_element() {
            return true;
        }
        let style = a
            .attr("style")
            .map(|s| s.to_ascii_lowercase().replace(' ', ""))
            .unwrap_or_default();
        let hiding_class = a.attr("class").is_some_and(|c| {
            c.split_whitespace()
                .any(|t| HIDING_CLASSES.contains(&t.to_ascii_lowercase().as_str()))
        });
        !(a.has_attr("hidden")
            || a.attr("aria-hidden").is_some_and(|v| &*v == "true")
            || style.contains("display:none")
            || style.contains("visibility:hidden")
            || hiding_class
            || ["template", "noscript", "script", "style"]
                .iter()
                .any(|t| a.has_name(t)))
    })
}

fn declared_word_count(v: &Value) -> Option<usize> {
    match find_key(v, "wordCount")? {
        Value::Number(n) => n.as_u64().map(|n| n as usize),
        Value::String(s) => s.trim().replace(',', "").parse().ok(),
        _ => None,
    }
}

pub(super) fn paywall_signals(page: &Dom) -> PaywallSignals {
    let ld = json_ld(page);
    let structured = ld.iter().any(|v| match find_key(v, "isAccessibleForFree") {
        Some(Value::Bool(free)) => !free,
        Some(Value::String(s)) => s.eq_ignore_ascii_case("false"),
        _ => false,
    });
    let declared_words = ld.iter().find_map(declared_word_count);
    let mut signals = PaywallSignals {
        structured,
        declared_words,
        ..PaywallSignals::default()
    };
    for n in page.select("[class]").nodes() {
        let Some(class) = n.attr("class") else {
            continue;
        };
        let has = |names: &[&str]| {
            class
                .split_whitespace()
                .any(|t| names.contains(&t.to_ascii_lowercase().as_str()))
        };
        let cut = has(CUT_GATE_CLASSES);
        let generic = has(GENERIC_GATE_CLASSES);
        if !cut && !generic {
            continue;
        }
        signals.gate = true;
        if is_visible(n) && !collapse(&n.text()).is_empty() {
            signals.cut_gate |= cut;
            signals.shown_gate |= generic;
        }
    }
    signals
}

/// The page's JSON-LD `articleBody`, as paragraphs.
pub(super) fn json_ld_article_body(page: &Dom) -> Option<Vec<Block>> {
    json_ld(page)
        .iter()
        .find_map(|v| match find_key(v, "articleBody") {
            Some(Value::String(s)) => {
                let blocks = paragraphs(s);
                (!blocks.is_empty()).then_some(blocks)
            }
            _ => None,
        })
}

/// A description as a single paragraph.
pub(super) fn paragraphs_of(description: &str) -> Vec<Block> {
    let text = collapse(description);
    if text.is_empty() {
        Vec::new()
    } else {
        vec![block(BlockKind::Paragraph, text)]
    }
}

/// The abstract declared by scholarly citation metadata
/// (`<meta name="citation_abstract">`), as a paragraph.
pub(super) fn citation_abstract(page: &Dom) -> Option<Vec<Block>> {
    let sel = r#"meta[name="citation_abstract"]"#;
    let content = page.select(sel).attr("content")?;
    let blocks = paragraphs(&content);
    (!blocks.is_empty()).then_some(blocks)
}

/// Authors declared by `<meta name="citation_author">`, joined with `; `.
pub(super) fn citation_authors(page: &Dom) -> Option<String> {
    let authors: Vec<String> = page
        .select(r#"meta[name="citation_author"]"#)
        .nodes()
        .iter()
        .filter_map(|n| n.attr("content").map(|c| collapse(&c)))
        .filter(|a| !a.is_empty())
        .collect();
    (!authors.is_empty()).then(|| authors.join("; "))
}
