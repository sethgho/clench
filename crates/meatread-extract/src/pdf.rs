//! PDF extraction (feature `pdf`).

use std::panic::{self, AssertUnwindSafe};

use meatread_core::{Block, BlockKind, Document, Source};
use pdf_extract::{Document as Pdf, Object, PlainTextOutput};

use crate::ExtractError;
use crate::text::{self, block, collapse, readable_words, word_count};

/// PDFs with fewer words than this are flagged as partial.
const MIN_PDF_WORDS: usize = 80;

/// A title is taken from the first block this short, within the first blocks.
const TITLE_MAX_WORDS: usize = 14;
const TITLE_SEARCH_BLOCKS: usize = 6;

/// A single-line paragraph this short can be a section heading.
const HEADING_MAX_WORDS: usize = 10;

/// Section titles that carry no number.
const BARE_HEADINGS: &[&str] = &[
    "abstract",
    "references",
    "acknowledgements",
    "acknowledgments",
    "appendix",
    "conclusion",
    "conclusions",
    "introduction",
    "bibliography",
];

/// Extracts a document from the bytes of a PDF.
///
/// Only the text layer is read. Lines that the page layout wrapped are joined
/// with a single space; blank lines separate paragraphs. A hyphen at the end
/// of a line is kept as written and the next line's first word stays a
/// separate word, because deciding whether the hyphen was typographic would
/// be a guess. Short numbered lines such as `3.2 Attention` become headings.
/// Mathematics, tables and multi-column layouts come out in the order the
/// text layer stores them and can be garbled.
///
/// # Errors
///
/// [`ExtractError::Pdf`] if the file cannot be parsed, is encrypted with a
/// password, or has no text layer (a scanned document, for instance).
pub fn extract_pdf(bytes: &[u8], url: &str) -> Result<Document, ExtractError> {
    // The PDF reader panics on some malformed files; report those as errors
    // where the build supports unwinding.
    let parsed = panic::catch_unwind(AssertUnwindSafe(|| parse(bytes)))
        .map_err(|_| ExtractError::Pdf("the file is malformed".to_string()))??;
    let (raw, info_title, info_author) = parsed;

    let mut blocks = blocks_from_text(&raw);
    // A lone number between paragraphs is a page number.
    blocks.retain(|b| !(b.text.len() <= 4 && b.text.chars().all(|c| c.is_ascii_digit())));
    if blocks.is_empty() {
        return Err(ExtractError::Pdf(
            "no text layer found (is this a scanned document?)".to_string(),
        ));
    }
    let title = info_title.unwrap_or_else(|| guess_title(&blocks));
    if let Some(b) = blocks
        .iter_mut()
        .take(TITLE_SEARCH_BLOCKS)
        .find(|b| !title.is_empty() && b.text == title)
    {
        b.kind = BlockKind::Heading;
        b.level = Some(1);
    }
    let words = readable_words(&blocks);
    Ok(Document {
        url: url.to_string(),
        title,
        byline: info_author,
        source: Source::Pdf,
        partial: words < MIN_PDF_WORDS,
        word_count: words,
        blocks,
        ..Document::default()
    })
}

type Parsed = (String, Option<String>, Option<String>);

fn parse(bytes: &[u8]) -> Result<Parsed, ExtractError> {
    let mut doc = Pdf::load_mem(bytes).map_err(|e| ExtractError::Pdf(e.to_string()))?;
    if doc.is_encrypted() {
        doc.decrypt("")
            .map_err(|_| ExtractError::Pdf("the file is password protected".to_string()))?;
    }
    let (title, author) = info(&doc);
    let mut raw = String::new();
    {
        let mut out = PlainTextOutput::new(&mut raw);
        pdf_extract::output_doc(&doc, &mut out).map_err(|e| ExtractError::Pdf(e.to_string()))?;
    }
    Ok((raw, title, author))
}

/// Title and author from the document information dictionary.
fn info(doc: &Pdf) -> (Option<String>, Option<String>) {
    let dict = doc
        .trailer
        .get(b"Info")
        .ok()
        .and_then(|o| doc.dereference(o).ok())
        .and_then(|(_, o)| o.as_dict().ok());
    let field = |key: &[u8]| -> Option<String> {
        match dict?.get(key).ok()? {
            Object::String(raw, _) => {
                let s = decode_pdf_string(raw);
                let s = collapse(&s);
                (!s.is_empty()).then_some(s)
            }
            _ => None,
        }
    };
    (field(b"Title"), field(b"Author"))
}

/// Decodes a PDF text string: UTF-16BE with a byte order mark, otherwise
/// PDFDocEncoding, which matches Latin-1 for the characters in use.
fn decode_pdf_string(raw: &[u8]) -> String {
    if raw.starts_with(&[0xfe, 0xff]) {
        encoding_rs::UTF_16BE.decode(&raw[2..]).0.into_owned()
    } else if raw.starts_with(&[0xef, 0xbb, 0xbf]) {
        String::from_utf8_lossy(&raw[3..]).into_owned()
    } else {
        raw.iter().map(|&b| b as char).collect()
    }
}

fn guess_title(blocks: &[Block]) -> String {
    blocks
        .iter()
        .take(TITLE_SEARCH_BLOCKS)
        .find(|b| {
            word_count(&b.text) <= TITLE_MAX_WORDS
                && word_count(&b.text) >= 2
                && !b.text.ends_with(['.', ',', ';', ':'])
        })
        .map(|b| b.text.clone())
        .unwrap_or_default()
}

/// Splits extracted text into paragraphs on blank lines and types headings.
fn blocks_from_text(raw: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut lines: Vec<&str> = Vec::new();
    let finish = |lines: &mut Vec<&str>, blocks: &mut Vec<Block>| {
        if lines.is_empty() {
            return;
        }
        let single = lines.len() == 1;
        let joined = collapse(&lines.join(" "));
        lines.clear();
        if joined.is_empty() {
            return;
        }
        match single.then(|| heading_level(&joined)).flatten() {
            Some(level) => blocks.push(Block {
                level: Some(level),
                ..block(BlockKind::Heading, joined)
            }),
            None => blocks.push(block(BlockKind::Paragraph, joined)),
        }
    };
    for line in raw.lines() {
        if line.trim_matches(text::is_html_space).is_empty() {
            finish(&mut lines, &mut blocks);
        } else {
            lines.push(line);
        }
    }
    finish(&mut lines, &mut blocks);
    blocks
}

/// The heading level of a one-line paragraph that looks like a section title.
fn heading_level(line: &str) -> Option<u8> {
    if word_count(line) > HEADING_MAX_WORDS || line.ends_with(['.', ',', ';', ':']) {
        return None;
    }
    if BARE_HEADINGS.contains(&line.to_lowercase().as_str()) {
        return Some(2);
    }
    let (number, rest) = line.split_once(' ')?;
    let numeric = !number.is_empty()
        && number
            .split('.')
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()));
    let starts_upper = rest.chars().next().is_some_and(char::is_uppercase);
    if numeric && starts_upper {
        let depth = number.split('.').count() as u8;
        return Some((depth + 1).min(6));
    }
    None
}
