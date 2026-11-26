//! Content and signals from outside the article body: JSON-LD structured
//! data, the meta description, and paywall markers.

use dom_smoothie::dom_query::Document as Dom;
use meatread_core::{Block, BlockKind};
use serde_json::Value;

use crate::text::{block, collapse, paragraphs};

/// Evidence that a page is gated behind a paywall.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct PaywallSignals {
    /// Structured data declares the content not freely accessible. Some free
    /// sites declare this too, so it is only trusted together with a short
    /// body.
    pub structured: bool,
    /// The page contains a subscription-gate element.
    pub gate: bool,
}

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

pub(super) fn paywall_signals(page: &Dom) -> PaywallSignals {
    let structured = json_ld(page)
        .iter()
        .any(|v| match find_key(v, "isAccessibleForFree") {
            Some(Value::Bool(free)) => !free,
            Some(Value::String(s)) => s.eq_ignore_ascii_case("false"),
            _ => false,
        });
    let gate = page.select("[class]").nodes().iter().any(|n| {
        n.attr("class").is_some_and(|c| {
            c.split_whitespace().any(|t| {
                let t = t.to_ascii_lowercase();
                matches!(
                    t.as_str(),
                    "paywall" | "paywalled" | "post-access-cta" | "gh-post-upgrade-cta"
                )
            })
        })
    });
    PaywallSignals { structured, gate }
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
