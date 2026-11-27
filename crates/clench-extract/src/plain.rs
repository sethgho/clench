//! Plain-text input.

use clench_core::{Document, Source};

use crate::ExtractError;
use crate::text::{self, collapse};

/// Extracts a document from plain text: paragraphs are separated by blank
/// lines, and line breaks inside a paragraph become single spaces.
///
/// The title is the first line when it is short and not sentence-like;
/// otherwise the title is empty. The text itself, including that first line,
/// is kept.
///
/// # Errors
///
/// [`ExtractError::NoContent`] if `text` has no words.
pub fn extract_plain(text: &str, url: &str) -> Result<Document, ExtractError> {
    let blocks = text::paragraphs(text);
    if blocks.is_empty() {
        return Err(ExtractError::NoContent);
    }
    let title = blocks
        .first()
        .filter(|b| blocks.len() > 1 && !b.text.contains('\n'))
        .map(|b| b.text.clone())
        .filter(|t| text::word_count(t) <= 12 && !t.ends_with(['.', '!', '?', ',', ';']))
        .unwrap_or_default();
    let words = text::readable_words(&blocks);
    Ok(Document {
        url: url.to_string(),
        title: collapse(&title),
        source: Source::PlainText,
        word_count: words,
        blocks,
        ..Document::default()
    })
}
