//! # clench-extract
//!
//! Turns a fetched resource into a [`clench_core::Document`]: typed blocks
//! plus metadata, ready for [`clench_core::Script`].
//!
//! The crate does no I/O. A host fetches the bytes and passes them to
//! [`extract`] together with the response's Content-Type and the URL.
//!
//! ## The one rule
//!
//! Extraction never alters the author's words. It decides which blocks to
//! keep or drop and collapses HTML white space (runs of space, tab, line
//! feed, form feed and carriage return become one space). It does not
//! rewrite, normalize, correct, translate or summarize, and there is no
//! model of any kind in the pipeline. The consequences, format by format:
//!
//! * **HTML** is parsed once; the main content is located with a port of
//!   Mozilla's Readability algorithm and walked into blocks. Words are kept
//!   or dropped, never edited. Non-breaking spaces are content and are kept.
//! * **Markdown** text is kept as written; only the markup syntax is
//!   consumed (`**bold**` becomes `bold`). Entities and escapes are decoded
//!   by the Markdown parser as the author's renderer would show them.
//! * **PDF** text is whatever the PDF's text layer holds. Lines hard-wrapped
//!   by the layout are joined with a space. A hyphen at the end of a line is
//!   never removed, because telling a typesetter's hyphenation from a real
//!   hyphen would be a guess; `trans-` `lation` stays `trans- lation`.
//!   Math, tables and multi-column pages come out as the text layer orders
//!   them, which can be garbled.
//!
//! ## Metadata
//!
//! [`Document::source`] says where the blocks came from. [`Document::partial`]
//! is set whenever a reader could be misled into believing they have the
//! whole article: the content came from a fallback (structured data, citation
//! abstract or the meta description), the body is implausibly short, or a
//! visible paywall prompt is corroborated by a short body or a declared word
//! count the body falls well short of. [`Document::paywall_hint`] reports a
//! paywall signal on its own. [`Document::word_count`] counts the words the
//! default reading policy plays; footnotes, captions, code and tables are
//! not in it.
//!
//! ## Features
//!
//! * `markdown` (default): [`extract_markdown`], using `pulldown-cmark`.
//! * `pdf` (default): [`extract_pdf`], using `pdf-extract`.
//!
//! Without a feature, input of that type returns
//! [`ExtractError::FeatureDisabled`] from [`extract`].
#![warn(missing_docs)]

mod encoding;
mod html;
#[cfg(feature = "markdown")]
mod markdown;
#[cfg(feature = "pdf")]
mod pdf;
mod plain;
mod sniff;
mod text;

use core::fmt;

pub use clench_core::Document;

pub use html::extract_html;
#[cfg(feature = "markdown")]
pub use markdown::extract_markdown;
#[cfg(feature = "pdf")]
pub use pdf::extract_pdf;
pub use plain::extract_plain;

/// Why extraction failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtractError {
    /// The input was readable but held no text to read.
    NoContent,
    /// The content type cannot be read as text (an image, an archive, ...).
    /// Holds the content type or the sniffed kind.
    UnsupportedType(String),
    /// The format is recognised but its Cargo feature is disabled. Holds the
    /// feature name.
    FeatureDisabled(&'static str),
    /// The PDF could not be parsed or has no text layer. Holds the reason.
    Pdf(String),
    /// The page nests elements so deeply that reading it risks exhausting
    /// the stack or taking unbounded time; no real page does. `limit` is the
    /// deepest nesting accepted.
    TooDeeplyNested {
        /// The deepest nesting accepted.
        limit: usize,
    },
}

impl fmt::Display for ExtractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExtractError::NoContent => f.write_str("no readable text found"),
            ExtractError::UnsupportedType(t) => write!(f, "unsupported content type: {t}"),
            ExtractError::FeatureDisabled(name) => {
                write!(f, "support for this format needs the `{name}` feature")
            }
            ExtractError::Pdf(why) => write!(f, "could not read PDF: {why}"),
            ExtractError::TooDeeplyNested { limit } => {
                write!(f, "the page nests elements more than {limit} levels deep")
            }
        }
    }
}

impl std::error::Error for ExtractError {}

/// Extracts a readable [`Document`] from a fetched resource.
///
/// The format is chosen from the bytes and the headers: a PDF header at the
/// very start of the bytes (after an optional byte order mark and white
/// space) wins, then the Content-Type (`text/html`, `text/markdown`, `text/plain`, ...),
/// then the URL's path extension (`.md`, `.markdown`) and finally a sniff of
/// the first bytes for HTML. Unknown text is treated as plain text.
///
/// `content_type` is the raw header value and may carry a `charset`
/// parameter; the text is decoded accordingly (a byte order mark overrides
/// the header, and HTML falls back to its `<meta>` declaration). With no
/// charset declared, valid UTF-8 is read as UTF-8 and other bytes as
/// windows-1252.
/// `url` is recorded on the document and resolves relative links; it may be
/// empty.
///
/// ```
/// use clench_extract::extract;
///
/// let doc = extract(b"A Title\n\nFirst paragraph.\n\nSecond one.", Some("text/plain"), "").unwrap();
/// assert_eq!(doc.title, "A Title");
/// assert_eq!(doc.blocks.len(), 3);
/// ```
///
/// # Errors
///
/// [`ExtractError::UnsupportedType`] for images and other binary types,
/// [`ExtractError::NoContent`] when nothing readable remains,
/// [`ExtractError::TooDeeplyNested`] for HTML nested absurdly deep, and
/// [`ExtractError::Pdf`] or [`ExtractError::FeatureDisabled`] for PDFs.
pub fn extract(
    bytes: &[u8],
    content_type: Option<&str>,
    url: &str,
) -> Result<Document, ExtractError> {
    match sniff::detect(bytes, content_type, url)? {
        sniff::Format::Pdf => pdf_dispatch(bytes, url),
        sniff::Format::Html => extract_html(&encoding::decode_html(bytes, content_type), url),
        sniff::Format::Markdown => {
            markdown_dispatch(&encoding::decode_text(bytes, content_type), url)
        }
        sniff::Format::Plain => extract_plain(&encoding::decode_text(bytes, content_type), url),
    }
}

#[cfg(feature = "pdf")]
fn pdf_dispatch(bytes: &[u8], url: &str) -> Result<Document, ExtractError> {
    extract_pdf(bytes, url)
}

#[cfg(not(feature = "pdf"))]
fn pdf_dispatch(_bytes: &[u8], _url: &str) -> Result<Document, ExtractError> {
    Err(ExtractError::FeatureDisabled("pdf"))
}

#[cfg(feature = "markdown")]
fn markdown_dispatch(text: &str, url: &str) -> Result<Document, ExtractError> {
    extract_markdown(text, url)
}

#[cfg(not(feature = "markdown"))]
fn markdown_dispatch(text: &str, url: &str) -> Result<Document, ExtractError> {
    // Without the parser, Markdown is still readable as the plain text it is.
    extract_plain(text, url)
}
