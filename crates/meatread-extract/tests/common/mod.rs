#![allow(dead_code)]

use std::path::PathBuf;

use meatread_core::{Block, BlockAction, BlockKind, Document, ReadPolicy};
use meatread_extract::extract;

pub const FIXTURES: [&str; 16] = [
    "01-iankduncan",
    "02-news-arstechnica",
    "03-substack-acx",
    "04-ghost-404media",
    "05-static-matklad",
    "06-wikipedia",
    "07-rustbook",
    "08-mdn",
    "09-github-readme",
    "10-arxiv-abs",
    "11-paulgraham",
    "12-recipe-allrecipes",
    "13-hn-thread",
    "14-nonenglish-spiegel",
    "15-spa-bsky",
    "16-news-bbc",
];

pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

pub fn fixture_html_bytes(name: &str) -> Vec<u8> {
    std::fs::read(fixtures_dir().join(format!("{name}.html"))).expect("fixture html")
}

pub fn fixture_url(name: &str) -> String {
    std::fs::read_to_string(fixtures_dir().join(format!("{name}.url")))
        .expect("fixture url")
        .trim()
        .to_string()
}

/// Extracts a fixture the way a host would: raw bytes, a content type, a URL.
pub fn load(name: &str) -> Document {
    extract(
        &fixture_html_bytes(name),
        Some("text/html; charset=utf-8"),
        &fixture_url(name),
    )
    .unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// The blocks a reader plays under the default policy.
pub fn readable(doc: &Document) -> impl Iterator<Item = &Block> {
    let policy = ReadPolicy::default();
    doc.blocks
        .iter()
        .filter(move |b| policy.resolve(b) == BlockAction::Read)
}

pub fn of_kind(doc: &Document, kind: BlockKind) -> Vec<&Block> {
    doc.blocks.iter().filter(|b| b.kind == kind).collect()
}

/// Whitespace collapsed and typographic quotes straightened, so expectations
/// can be written with plain ASCII quotes.
pub fn norm(s: &str) -> String {
    let straightened: String = s
        .chars()
        .map(|c| match c {
            '\u{2018}' | '\u{2019}' => '\'',
            '\u{201c}' | '\u{201d}' => '"',
            '\u{a0}' => ' ',
            c => c,
        })
        .collect();
    straightened
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// The text a reader plays: readable blocks, one per line, normalized.
pub fn reading_text(doc: &Document) -> String {
    readable(doc)
        .map(|b| norm(&b.text))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Every block's text, normalized (includes code and tables).
pub fn all_text(doc: &Document) -> String {
    doc.blocks
        .iter()
        .map(|b| norm(&b.text))
        .collect::<Vec<_>>()
        .join("\n")
}
