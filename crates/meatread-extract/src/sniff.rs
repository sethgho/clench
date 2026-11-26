//! Choosing an extractor from bytes, headers and URL.

use crate::ExtractError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Format {
    Pdf,
    Html,
    Markdown,
    Plain,
}

/// The media type of a Content-Type value, lower-cased, without parameters.
fn media_type(content_type: Option<&str>) -> String {
    content_type
        .and_then(|c| c.split(';').next())
        .map(|m| m.trim().to_ascii_lowercase())
        .unwrap_or_default()
}

/// The path part of `url`, lower-cased, without query or fragment.
fn url_path(url: &str) -> String {
    let end = url.find(['?', '#']).unwrap_or(url.len());
    url[..end].to_ascii_lowercase()
}

fn has_markdown_extension(url: &str) -> bool {
    let p = url_path(url);
    p.ends_with(".md") || p.ends_with(".markdown")
}

/// Whether the start of `bytes` looks like an HTML document.
fn looks_like_html(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(1024)];
    let head = String::from_utf8_lossy(head).to_ascii_lowercase();
    let t = head.trim_start_matches(['\u{feff}', ' ', '\t', '\r', '\n']);
    t.starts_with("<!doctype html")
        || t.starts_with("<html")
        || head.contains("<html")
        || head.contains("<head")
        || head.contains("<body")
}

/// Whether `bytes` start with the PDF header, allowing a byte order mark and
/// leading white space. The marker anywhere later is just text.
fn is_pdf(bytes: &[u8]) -> bool {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    let start = bytes
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    bytes[start..].starts_with(b"%PDF-")
}

pub(crate) fn detect(
    bytes: &[u8],
    content_type: Option<&str>,
    url: &str,
) -> Result<Format, ExtractError> {
    if is_pdf(bytes) {
        return Ok(Format::Pdf);
    }
    let media = media_type(content_type);
    match media.as_str() {
        "text/html" | "application/xhtml+xml" => return Ok(Format::Html),
        "text/markdown" | "text/x-markdown" | "application/markdown" => {
            return Ok(Format::Markdown);
        }
        "application/pdf" => return Ok(Format::Pdf),
        _ => {}
    }
    let binary = media.starts_with("image/")
        || media.starts_with("audio/")
        || media.starts_with("video/")
        || media.starts_with("font/")
        || matches!(
            media.as_str(),
            "application/zip"
                | "application/gzip"
                | "application/x-tar"
                | "application/wasm"
                | "application/x-msdownload"
        );
    if binary {
        return Err(ExtractError::UnsupportedType(media));
    }
    if has_markdown_extension(url) {
        return Ok(Format::Markdown);
    }
    if looks_like_html(bytes) {
        return Ok(Format::Html);
    }
    Ok(Format::Plain)
}
