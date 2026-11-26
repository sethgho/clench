//! A cheap guard against hostile nesting.
//!
//! Parsing and main-content detection take time that grows faster than
//! linearly with nesting depth, and deep trees can exhaust the stack (and
//! trap a WebAssembly instance). Browsers cap tree depth at a few hundred.
//! [`max_depth`] estimates how deep the parser will build the tree, without
//! building it.

/// Elements that never have content or an end tag.
const VOID: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

/// Elements whose content is raw text, so `<` inside it is not a tag.
const RAW_TEXT: &[&str] = &["script", "style", "textarea", "title", "xmp", "plaintext"];

/// The estimated maximum element nesting depth of `html`, stopping early once
/// it exceeds `limit`.
///
/// Tags are matched the way the HTML parser nests them: a void element or a
/// self-closing tag adds no depth, an end tag closes everything opened after
/// its start tag, a `/>` closes only SVG and MathML elements, and the
/// elements with optional end tags (`li`, `p`, `td`,
/// ...) close a previous sibling of the same kind. The estimate can differ
/// from the parser's by a few levels on malformed input, never by a factor.
pub(super) fn max_depth(html: &str, limit: usize) -> usize {
    let bytes = html.as_bytes();
    let mut stack: Vec<&str> = Vec::new();
    let mut max = 0;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }
        let rest = &html[i + 1..];
        if let Some(after) = rest.strip_prefix("!--") {
            i += 1 + 3 + after.find("-->").map_or(after.len(), |e| e + 3);
            continue;
        }
        let closing = rest.starts_with('/');
        let name_start = i + 1 + usize::from(closing);
        let name_len = html[name_start..]
            .bytes()
            .take_while(|b| b.is_ascii_alphanumeric() || *b == b'-' || *b == b':')
            .count();
        if name_len == 0 || !bytes[name_start].is_ascii_alphabetic() {
            // `<!doctype`, `<?xml`, a stray `<`.
            i += 1;
            continue;
        }
        let name = &html[name_start..name_start + name_len];
        let end = tag_end(bytes, name_start + name_len);
        let self_closing = end >= 2 && bytes[end - 2] == b'/';
        i = end;
        if closing {
            if let Some(at) = stack.iter().rposition(|n| n.eq_ignore_ascii_case(name)) {
                stack.truncate(at);
            }
            continue;
        }
        if VOID.iter().any(|v| v.eq_ignore_ascii_case(name)) {
            continue;
        }
        if RAW_TEXT.iter().any(|v| v.eq_ignore_ascii_case(name)) {
            i = skip_raw_text(html, i, name);
            continue;
        }
        // `/>` only closes an element in SVG and MathML content; on an HTML
        // element the parser ignores the slash and the element stays open.
        if self_closing
            && stack
                .iter()
                .any(|n| n.eq_ignore_ascii_case("svg") || n.eq_ignore_ascii_case("math"))
        {
            continue;
        }
        close_implied(&mut stack, name);
        stack.push(name);
        max = max.max(stack.len());
        if max > limit {
            return max;
        }
    }
    max
}

/// Index just past the `>` that ends the tag whose name ends at `from`,
/// honouring quoted attribute values.
fn tag_end(bytes: &[u8], from: usize) -> usize {
    let mut quote = None;
    let mut i = from;
    while i < bytes.len() {
        match (quote, bytes[i]) {
            (None, b'>') => return i + 1,
            (None, q @ (b'"' | b'\'')) => quote = Some(q),
            (Some(q), c) if c == q => quote = None,
            _ => {}
        }
        i += 1;
    }
    bytes.len()
}

/// Index just past the end tag of the raw-text element `name`.
fn skip_raw_text(html: &str, from: usize, name: &str) -> usize {
    let mut at = from;
    while let Some(found) = html[at..].find("</") {
        let start = at + found + 2;
        let candidate = html.as_bytes().get(start..start + name.len());
        if candidate.is_some_and(|c| c.eq_ignore_ascii_case(name.as_bytes())) {
            return tag_end(html.as_bytes(), start + name.len());
        }
        at = start;
    }
    html.len()
}

/// Pops the open elements that a start tag of `name` closes implicitly.
fn close_implied(stack: &mut Vec<&str>, name: &str) {
    let top_is = |stack: &Vec<&str>, names: &[&str]| {
        stack
            .last()
            .is_some_and(|t| names.iter().any(|n| n.eq_ignore_ascii_case(t)))
    };
    let lower = name.to_ascii_lowercase();
    match lower.as_str() {
        "li" if top_is(stack, &["li"]) => {
            stack.pop();
        }
        "p" if top_is(stack, &["p"]) => {
            stack.pop();
        }
        "dt" | "dd" if top_is(stack, &["dt", "dd"]) => {
            stack.pop();
        }
        "option" if top_is(stack, &["option"]) => {
            stack.pop();
        }
        "td" | "th" if top_is(stack, &["td", "th"]) => {
            stack.pop();
        }
        "tr" => {
            if top_is(stack, &["td", "th"]) {
                stack.pop();
            }
            if top_is(stack, &["tr"]) {
                stack.pop();
            }
        }
        "tbody" | "thead" | "tfoot" => {
            if top_is(stack, &["td", "th"]) {
                stack.pop();
            }
            if top_is(stack, &["tr"]) {
                stack.pop();
            }
            if top_is(stack, &["tbody", "thead", "tfoot"]) {
                stack.pop();
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_real_nesting() {
        assert_eq!(
            max_depth("<div><p>a</p><span><b>x</b></span></div>", 100),
            3
        );
        assert_eq!(max_depth(&"<div>".repeat(300), 100), 101);
    }

    #[test]
    fn optional_end_tags_do_not_pile_up() {
        let flat = "<ul>".to_string() + &"<li>item".repeat(5000) + "</ul>";
        assert!(max_depth(&flat, 100) <= 3);
        let paras = "<p>word ".repeat(5000);
        assert!(max_depth(&paras, 100) <= 2);
        let table = "<table>".to_string() + &"<tr><td>a<td>b".repeat(2000) + "</table>";
        assert!(max_depth(&table, 100) <= 4);
        let nested = "<ul>".to_string() + &"<li>item<ul>".repeat(500);
        assert!(max_depth(&nested, 10_000) >= 1000);
    }

    #[test]
    fn self_closing_counts_only_in_svg_and_math() {
        let svg = "<svg>".to_string() + &"<path d='M0 0'/>".repeat(5000) + "</svg>";
        assert_eq!(max_depth(&svg, 100), 1);
        let divs = "<div/>".repeat(300);
        assert_eq!(max_depth(&divs, 100), 101);
    }

    #[test]
    fn void_elements_comments_and_raw_text_add_nothing() {
        let html = "<div><br><img src=a><!-- <b><b><b> --><script>if (a<b) { '<div><div>' }</script>\
<input value='>'><span>text</span></div>";
        assert_eq!(max_depth(html, 100), 2);
    }
}
