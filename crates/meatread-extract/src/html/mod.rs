//! HTML extraction.
//!
//! The pipeline has four stages:
//!
//! 1. Readability locates the main content and returns a cleaned DOM.
//! 2. [`walk`] turns that DOM into typed blocks, keeping or dropping text and
//!    collapsing HTML white space, nothing more.
//! 3. [`trim`] removes leftover page chrome at the edges of the body and
//!    restores a standfirst that sits outside the scored container.
//! 4. [`fallback`] supplies content when the body is implausibly short:
//!    JSON-LD `articleBody`, then the meta description. Fallback content is
//!    always flagged as partial. Scholarly landing pages (pages carrying
//!    `citation_abstract` whose body is empty or just the abstract) read as
//!    their abstract, also flagged partial.

mod chrome;
mod fallback;
mod shield;
mod trim;
mod walk;

use std::collections::HashSet;

use dom_smoothie::dom_query::Document as Dom;
use dom_smoothie::{Config, Readability};
use meatread_core::{Block, Document, Source};

use crate::ExtractError;
use crate::text::{collapse, readable_words};

/// Bodies with fewer words than this are not trusted to be a whole article.
const MIN_ARTICLE_WORDS: usize = 80;

/// A body at most this many times as long as its page's citation abstract,
/// and containing it, is the abstract plus surrounding metadata (submission
/// history, subjects, links), not an article.
const ABSTRACT_BODY_FACTOR: usize = 2;

/// A paywalled page whose body is shorter than this is treated as truncated.
const MIN_PAYWALLED_WORDS: usize = 400;

/// A page declaring more than 3/2 of the words extracted from it (structured
/// data `wordCount`) is missing text.
const DECLARED_LONGER_NUM: usize = 3;
const DECLARED_LONGER_DEN: usize = 2;

/// Extracts a document from decoded HTML.
///
/// `url` is recorded on the document and used to resolve relative links; it
/// may be empty or unparsable. Use [`crate::extract`] to start from raw bytes.
///
/// # Errors
///
/// [`ExtractError::NoContent`] if neither the page body, its structured data
/// nor its meta description yields any text.
pub fn extract_html(html: &str, url: &str) -> Result<Document, ExtractError> {
    let page = Dom::from(html);
    let mut doc = Document {
        url: url.to_string(),
        ..Document::default()
    };

    let mut closing_notes = HashSet::new();
    let article = readability(&shield::run(html), url);
    if let Some(article) = &article {
        let m = &article.metadata;
        doc.title = collapse(&m.title);
        doc.byline = non_empty(m.byline.as_deref());
        doc.site_name = non_empty(m.site_name.as_deref());
        doc.lang = non_empty(m.lang.as_deref());
        doc.excerpt = non_empty(m.excerpt.as_deref());
        let walked = walk::blocks(&article.root());
        doc.blocks = walked.blocks;
        closing_notes = walked.closing_notes;
        doc.source = Source::Readability;
    }

    if doc.title.is_empty() {
        doc.title =
            meta(&page, "og:title").unwrap_or_else(|| collapse(&page.select("title").text()));
    }
    if doc.excerpt.is_none() {
        doc.excerpt = meta(&page, "og:description").or_else(|| meta(&page, "description"));
    }
    if doc.site_name.is_none() {
        doc.site_name = meta(&page, "og:site_name");
    }
    if doc.byline.is_none() {
        doc.byline = meta(&page, "author");
    }
    if doc.lang.is_none() {
        doc.lang = non_empty(page.select("html").attr("lang").as_deref());
    }

    let signals = fallback::paywall_signals(&page);
    doc.paywall_hint = signals.structured || signals.gate;

    if doc.source == Source::Readability {
        chrome::drop_noise(&mut doc.blocks);
        trim::trim_edge_chrome(&mut doc);
        chrome::trim_tail(&mut doc.blocks, &closing_notes);
        trim::rescue_lede(&page, &mut doc);
    }

    let mut words = readable_words(&doc.blocks);
    if let Some(blocks) = fallback::citation_abstract(&page) {
        if abstract_replaces_body(&doc, words, &blocks) {
            doc.blocks = blocks;
            doc.source = Source::CitationAbstract;
            words = readable_words(&doc.blocks);
            if let Some(authors) = fallback::citation_authors(&page) {
                doc.byline = Some(authors);
            }
        }
    }
    if words < MIN_ARTICLE_WORDS {
        if let Some(blocks) = fallback::json_ld_article_body(&page) {
            if readable_words(&blocks) > words {
                doc.blocks = blocks;
                doc.source = Source::JsonLd;
            }
        }
    }
    if doc.blocks.is_empty() {
        if let Some(blocks) = doc.excerpt.as_deref().map(fallback::paragraphs_of) {
            doc.blocks = blocks;
            doc.source = Source::MetaDescription;
        }
    }
    words = readable_words(&doc.blocks);
    doc.word_count = words;
    if doc.blocks.is_empty() || words == 0 {
        return Err(ExtractError::NoContent);
    }
    // A body far shorter than the page says the article is.
    let declared_longer = signals
        .declared_words
        .is_some_and(|d| d > words * DECLARED_LONGER_NUM / DECLARED_LONGER_DEN);
    let short_for_a_gate = words < MIN_PAYWALLED_WORDS || declared_longer;
    doc.partial = doc.source != Source::Readability
        || words < MIN_ARTICLE_WORDS
        || signals.cut_gate
        || (signals.shown_gate && short_for_a_gate)
        || (signals.structured && short_for_a_gate);
    Ok(doc)
}

/// Whether a page's citation abstract should stand in for its extracted body:
/// the body holds almost nothing, or is the abstract plus only a little
/// surrounding metadata. A body with real content of its own is kept.
fn abstract_replaces_body(doc: &Document, body_words: usize, abstract_blocks: &[Block]) -> bool {
    let abstract_words = readable_words(abstract_blocks);
    if body_words < MIN_ARTICLE_WORDS {
        return abstract_words >= body_words;
    }
    let abstract_text = abstract_blocks
        .iter()
        .map(|b| b.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let probe: String = abstract_text
        .split_whitespace()
        .take(8)
        .collect::<Vec<_>>()
        .join(" ");
    let body_text = doc
        .blocks
        .iter()
        .map(|b| collapse(&b.text))
        .collect::<Vec<_>>()
        .join(" ");
    body_words <= abstract_words * ABSTRACT_BODY_FACTOR && body_text.contains(&probe)
}

/// The blocks of an HTML fragment (a raw HTML block of a Markdown document),
/// read with the same walker as a full page but without main-content
/// detection or page-furniture trimming. Tags and attributes never reach the
/// text; script and style content is dropped.
#[cfg(feature = "markdown")]
pub(crate) fn fragment_blocks(raw: &str) -> Vec<Block> {
    let dom = Dom::from(raw);
    let body = dom.select("body");
    match body.nodes().first() {
        Some(root) => walk::blocks(root).blocks,
        None => Vec::new(),
    }
}

/// Runs Readability, retrying without a base URL if the URL is unusable.
fn readability(html: &str, url: &str) -> Option<dom_smoothie::Extracted> {
    let config = || Config {
        keep_classes: true,
        ..Config::default()
    };
    let base = (!url.is_empty()).then_some(url);
    Readability::new(html, base, Some(config()))
        .or_else(|_| Readability::new(html, None, Some(config())))
        .ok()
        .and_then(|r| r.extract().ok())
}

fn non_empty(s: Option<&str>) -> Option<String> {
    s.map(collapse).filter(|s| !s.is_empty())
}

/// Content of `<meta property|name=key>`.
fn meta(page: &Dom, key: &str) -> Option<String> {
    let sel = format!(r#"meta[property="{key}"], meta[name="{key}"]"#);
    non_empty(page.select(&sel).attr("content").as_deref())
}
