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
//!    `citation_abstract`) read as their abstract, also flagged partial.

mod chrome;
mod fallback;
mod shield;
mod trim;
mod walk;

use std::collections::HashSet;

use dom_smoothie::dom_query::Document as Dom;
use dom_smoothie::{Config, Readability};
use meatread_core::{Document, Source};

use crate::ExtractError;
use crate::text::{collapse, readable_words};

/// Bodies with fewer words than this are not trusted to be a whole article.
const MIN_ARTICLE_WORDS: usize = 80;

/// A page with a citation abstract and a body shorter than this is a landing
/// page for a paper, not the paper.
const MAX_ABSTRACT_PAGE_WORDS: usize = 600;

/// A paywalled page whose body is shorter than this is treated as truncated.
const MIN_PAYWALLED_WORDS: usize = 400;

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
    if words < MAX_ABSTRACT_PAGE_WORDS {
        // A scholarly landing page: the abstract is the article here, and the
        // paper itself lives elsewhere.
        if let Some(blocks) = fallback::citation_abstract(&page) {
            doc.blocks = blocks;
            doc.source = Source::MetaDescription;
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
    doc.partial = doc.source != Source::Readability
        || words < MIN_ARTICLE_WORDS
        || signals.gate
        || (signals.structured && words < MIN_PAYWALLED_WORDS);
    Ok(doc)
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
