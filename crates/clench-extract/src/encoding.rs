//! Decoding raw bytes into text.
//!
//! The charset comes from, in order: a byte order mark, the `charset`
//! parameter of the Content-Type header, and (for HTML) a `<meta>` tag near
//! the top of the document. With nothing declared, bytes that are valid UTF-8
//! are read as UTF-8 and anything else as windows-1252, the WHATWG Encoding
//! Standard's default for most locales. A declared charset is always obeyed;
//! invalid sequences in it are replaced by U+FFFD.

use encoding_rs::{Encoding, UTF_8, WINDOWS_1252};

/// How far into an HTML document a `<meta>` charset declaration is looked for.
const META_PRESCAN_BYTES: usize = 4096;

/// The `charset` parameter of a Content-Type header value.
pub(crate) fn header_charset(content_type: Option<&str>) -> Option<&'static Encoding> {
    let ct = content_type?;
    let lower = ct.to_ascii_lowercase();
    let at = lower.find("charset")?;
    let rest = lower[at + "charset".len()..].trim_start();
    let rest = rest.strip_prefix('=')?.trim_start();
    let label: String = rest
        .trim_start_matches(['"', '\''])
        .chars()
        .take_while(|c| !matches!(c, ';' | '"' | '\'' | ' ' | '\t'))
        .collect();
    Encoding::for_label(label.as_bytes())
}

/// Finds a charset declared by `<meta charset=...>` or
/// `<meta http-equiv="Content-Type" content="...; charset=...">` in the first
/// bytes of `bytes`.
pub(crate) fn meta_charset(bytes: &[u8]) -> Option<&'static Encoding> {
    let head = &bytes[..bytes.len().min(META_PRESCAN_BYTES)];
    // Declarations are ASCII; map every other byte to a placeholder so the
    // search is safe on any encoding that is ASCII-compatible.
    let ascii: String = head
        .iter()
        .map(|&b| if b < 0x80 { b as char } else { '?' })
        .collect::<String>()
        .to_ascii_lowercase();
    let mut from = 0;
    while let Some(i) = ascii[from..].find("<meta") {
        let start = from + i;
        let end = ascii[start..].find('>').map_or(ascii.len(), |e| start + e);
        let tag = &ascii[start..end];
        if let Some(label) = attribute_after(tag, "charset") {
            if let Some(enc) = label_to_encoding(&label) {
                return Some(enc);
            }
        }
        from = end.max(start + 5);
    }
    None
}

/// The value following `name` in `tag`, tolerating `=` spacing and quotes.
fn attribute_after(tag: &str, name: &str) -> Option<String> {
    let at = tag.find(name)?;
    let rest = tag[at + name.len()..].trim_start();
    let rest = rest.strip_prefix('=')?.trim_start();
    let value: String = rest
        .trim_start_matches(['"', '\''])
        .chars()
        .take_while(|c| !matches!(c, ';' | '"' | '\'' | ' ' | '\t' | '\n' | '\r' | '>' | '/'))
        .collect();
    (!value.is_empty()).then_some(value)
}

/// Maps a declared label to an encoding. A UTF-16 declaration inside an
/// ASCII-compatible byte stream cannot be true, so it is read as UTF-8.
fn label_to_encoding(label: &str) -> Option<&'static Encoding> {
    let enc = Encoding::for_label(label.as_bytes())?;
    if enc == encoding_rs::UTF_16LE || enc == encoding_rs::UTF_16BE {
        Some(UTF_8)
    } else {
        Some(enc)
    }
}

/// The encoding to assume when none is declared: UTF-8 if the bytes are
/// UTF-8 (a sequence cut off at the very end, as by a size limit, does not
/// count against it), windows-1252 otherwise.
fn undeclared(bytes: &[u8]) -> &'static Encoding {
    match std::str::from_utf8(bytes) {
        Ok(_) => UTF_8,
        Err(e) if e.error_len().is_none() => UTF_8,
        Err(_) => WINDOWS_1252,
    }
}

/// Decodes a non-HTML body (Markdown, plain text).
pub(crate) fn decode_text(bytes: &[u8], content_type: Option<&str>) -> String {
    let enc = header_charset(content_type).unwrap_or_else(|| undeclared(bytes));
    // `decode` lets a byte order mark override `enc`.
    enc.decode(bytes).0.into_owned()
}

/// Decodes an HTML body.
pub(crate) fn decode_html(bytes: &[u8], content_type: Option<&str>) -> String {
    let enc = header_charset(content_type)
        .or_else(|| meta_charset(bytes))
        .unwrap_or_else(|| undeclared(bytes));
    enc.decode(bytes).0.into_owned()
}
