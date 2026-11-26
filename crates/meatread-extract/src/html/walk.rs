//! Walks Readability's cleaned DOM into typed blocks.
//!
//! Text is only ever kept or dropped. The one transformation is collapsing
//! HTML white space, except inside `<pre>`, which is kept verbatim.

use std::collections::HashSet;

use dom_smoothie::dom_query::{NodeRef, Selection};
use meatread_core::{Block, BlockKind};

use crate::text::{block, collapse};

/// Marks a paragraph boundary inside buffered inline text (`<br><br>`).
const PARA_BREAK: char = '\u{1}';

/// The blocks of a walked DOM, plus the texts of paragraphs that look like an
/// author or syndication note: set entirely in italics and containing a link.
pub(super) struct Walked {
    pub blocks: Vec<Block>,
    pub closing_notes: HashSet<String>,
}

/// Walks the children of `root` into blocks.
pub(super) fn blocks(root: &NodeRef) -> Walked {
    let mut w = Walker::default();
    w.walk(root, Ctx::default());
    w.flush(Ctx::default());
    Walked {
        blocks: w.out,
        closing_notes: w.notes,
    }
}

#[derive(Clone, Copy, Default)]
struct Ctx {
    quote: u8,
    list_depth: u8,
    ordered: bool,
    footnotes: bool,
    /// Inside `<em>` or `<i>`.
    italic: bool,
}

fn styled(kind: BlockKind, text: String, ctx: &Ctx) -> Block {
    Block {
        quote: ctx.quote,
        ..block(kind, text)
    }
}

fn sel<'a>(n: &NodeRef<'a>, css: &str) -> Selection<'a> {
    Selection::from(*n).select(css)
}

#[derive(Default)]
struct Walker {
    out: Vec<Block>,
    buf: String,
    notes: HashSet<String>,
    /// The buffered text holds some text outside italics.
    has_roman: bool,
    /// The buffered text holds a link.
    has_link: bool,
}

impl Walker {
    /// Emits buffered inline text as text blocks typed by the context.
    fn flush(&mut self, ctx: Ctx) {
        let buf = std::mem::take(&mut self.buf);
        let note = !self.has_roman && self.has_link;
        self.has_roman = false;
        self.has_link = false;
        for part in buf.split(PARA_BREAK) {
            let text = collapse(part);
            if text.is_empty() {
                continue;
            }
            let kind = if ctx.footnotes {
                BlockKind::Footnote
            } else if ctx.list_depth > 0 {
                BlockKind::ListItem
            } else if ctx.quote > 0 {
                BlockKind::Blockquote
            } else {
                BlockKind::Paragraph
            };
            if note && kind == BlockKind::Paragraph {
                self.notes.insert(text.clone());
            }
            let mut b = styled(kind, text, &ctx);
            if kind == BlockKind::ListItem {
                b.depth = Some(ctx.list_depth);
                b.ordered = Some(ctx.ordered);
            }
            self.out.push(b);
        }
    }

    fn push(&mut self, kind: BlockKind, text: String, ctx: &Ctx) -> Option<&mut Block> {
        if text.is_empty() {
            return None;
        }
        self.out.push(styled(kind, text, ctx));
        self.out.last_mut()
    }

    fn walk(&mut self, node: &NodeRef, ctx: Ctx) {
        for child in node.children() {
            if child.is_text() {
                let text = child.text();
                if !ctx.italic && !text.trim().is_empty() {
                    self.has_roman = true;
                }
                self.buf.push_str(&text);
                continue;
            }
            if !child.is_element() {
                continue;
            }
            let name = child
                .node_name()
                .map(|n| n.to_ascii_lowercase())
                .unwrap_or_default();
            if is_dropped(&child, &name) {
                continue;
            }
            if !ctx.footnotes && is_footnote_section(&child) {
                self.flush(ctx);
                let inner = Ctx {
                    footnotes: true,
                    ..ctx
                };
                self.walk(&child, inner);
                self.flush(inner);
                continue;
            }
            self.element(&child, &name, ctx);
        }
    }

    fn element(&mut self, child: &NodeRef, name: &str, ctx: Ctx) {
        match name {
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                self.flush(ctx);
                let level = name.as_bytes()[1] - b'0';
                let text = inline_text(child);
                if let Some(b) = self.push(BlockKind::Heading, text, &ctx) {
                    b.level = Some(level);
                }
            }
            "pre" => {
                self.flush(ctx);
                let raw = child.text().to_string();
                let text = raw.trim_end_matches(['\n', '\r']).to_string();
                let lang = code_lang(child);
                if text.trim().is_empty() {
                    return;
                }
                if let Some(b) = self.push(BlockKind::Code, text, &ctx) {
                    b.lang = lang;
                }
            }
            "ul" | "ol" | "menu" => {
                self.flush(ctx);
                let inner = Ctx {
                    list_depth: ctx.list_depth.saturating_add(1),
                    ordered: name == "ol",
                    ..ctx
                };
                for item in child.children() {
                    if item.is_element() && item.has_name("li") {
                        self.walk(&item, inner);
                        self.flush(inner);
                    } else if item.is_element() {
                        self.walk(&item, inner);
                    }
                }
                self.flush(inner);
            }
            "blockquote" => {
                self.flush(ctx);
                let inner = Ctx {
                    quote: ctx.quote.saturating_add(1),
                    ..ctx
                };
                self.walk(child, inner);
                self.flush(inner);
            }
            "table" => {
                self.flush(ctx);
                if is_data_table(child) {
                    self.push(BlockKind::Table, table_text(child), &ctx);
                } else {
                    self.walk(child, ctx);
                    self.flush(ctx);
                }
            }
            "figure" if sel(child, "pre, blockquote, table, ul, ol").exists() => {
                // Figures used as wrappers (code listings, pull quotes).
                self.flush(ctx);
                self.walk(child, ctx);
                self.flush(ctx);
            }
            "figure" | "picture" => {
                self.flush(ctx);
                let alt = sel(child, "img")
                    .attr("alt")
                    .map(|a| collapse(&a))
                    .unwrap_or_default();
                self.push(BlockKind::Figure, alt, &ctx);
                let caption = sel(child, "figcaption");
                if caption.exists() {
                    let text = collapse(&caption.text());
                    self.push(BlockKind::Caption, text, &ctx);
                }
            }
            "p" | "div" | "span" if is_credit(child) => {
                self.flush(ctx);
                let text = inline_text(child);
                self.push(BlockKind::Caption, text, &ctx);
            }
            "figcaption" | "caption" => {
                self.flush(ctx);
                let text = inline_text(child);
                self.push(BlockKind::Caption, text, &ctx);
            }
            "hr" => {
                self.flush(ctx);
                self.out.push(styled(BlockKind::Rule, String::new(), &ctx));
            }
            "br" => {
                // A second consecutive <br> is a paragraph break (legacy layouts).
                if self.buf.trim_end_matches([' ', '\t', '\r']).ends_with('\n') {
                    self.buf.push(PARA_BREAK);
                } else {
                    self.buf.push('\n');
                }
            }
            "p" | "div" | "section" | "article" | "main" | "header" | "footer" | "aside"
            | "details" | "summary" | "dl" | "dt" | "dd" | "center" | "address" | "li"
            | "hgroup" | "fieldset" | "tr" | "td" | "th" | "tbody" | "thead" | "tfoot"
            | "legend" => {
                self.flush(ctx);
                self.walk(child, ctx);
                self.flush(ctx);
            }
            "em" | "i" => {
                let inner = Ctx {
                    italic: true,
                    ..ctx
                };
                self.walk(child, inner);
            }
            "a" => {
                self.has_link = true;
                self.walk(child, ctx);
            }
            // Inline elements: strong, code, span, font, ...
            _ => self.walk(child, ctx),
        }
    }
}

/// Elements that carry no readable text of their own, or are interface or
/// reference-marker furniture inside the article.
fn is_dropped(n: &NodeRef, name: &str) -> bool {
    match name {
        "script" | "style" | "noscript" | "template" | "button" | "input" | "select"
        | "textarea" | "form" | "nav" | "img" | "video" | "audio" | "iframe" | "svg" | "canvas"
        | "object" | "embed" | "wbr" => true,
        "sup" if is_footnote_ref(n) => true,
        _ => {
            is_backlink(n) || super::chrome::is_furniture(n) || super::chrome::is_standalone_time(n)
        }
    }
}

/// A link from a footnote back to the place it was cited from.
fn is_backlink(n: &NodeRef) -> bool {
    if n.attr("role").is_some_and(|r| &*r == "doc-backlink") {
        return true;
    }
    has_class(n, |c| {
        matches!(
            c,
            "mw-cite-backlink" | "footnote-backref" | "footnote-back-link" | "reversefootnote"
        )
    })
}

fn has_class(n: &NodeRef, pred: impl Fn(&str) -> bool) -> bool {
    n.attr("class")
        .is_some_and(|c| c.split_whitespace().any(|t| pred(&t.to_ascii_lowercase())))
}

/// Text of an element treated as a single inline run (headings, captions).
fn inline_text(node: &NodeRef) -> String {
    let mut w = Walker::default();
    w.walk(node, Ctx::default());
    w.flush(Ctx::default());
    w.out
        .into_iter()
        .map(|b| b.text)
        .collect::<Vec<_>>()
        .join(" ")
}

fn code_lang(pre: &NodeRef) -> Option<String> {
    let classes = [pre.attr("class"), sel(pre, "code").attr("class")];
    for class in classes.into_iter().flatten() {
        let mut tokens = class.split_whitespace();
        while let Some(token) = tokens.next() {
            if let Some(lang) = token
                .strip_prefix("language-")
                .or_else(|| token.strip_prefix("lang-"))
            {
                return Some(lang.to_string());
            }
            // MDN style: `brush: js`.
            if token == "brush:" {
                if let Some(lang) = tokens.next() {
                    return Some(lang.to_string());
                }
            }
        }
    }
    pre.attr("data-lang").map(|s| s.to_string())
}

/// Image credits and captions rendered outside a `<figure>`.
fn is_credit(n: &NodeRef) -> bool {
    has_class(n, |c| {
        c.split(['-', '_'])
            .any(|part| matches!(part, "credit" | "credits" | "caption"))
    })
}

fn is_footnote_ref(sup: &NodeRef) -> bool {
    let a = sel(sup, "a");
    let href = a.attr("href").unwrap_or_default();
    a.length() == 1 && href.starts_with('#') && collapse(&a.text()) == collapse(&sup.text())
}

fn is_footnote_section(n: &NodeRef) -> bool {
    n.attr("role").is_some_and(|r| &*r == "doc-endnotes")
        || has_class(n, |c| {
            matches!(
                c,
                "footnotes" | "footnote-definitions" | "references" | "reflist"
            )
        })
}

/// Distinguishes data tables from layout tables, as Readability does.
fn is_data_table(t: &NodeRef) -> bool {
    if t.attr("role").is_some_and(|r| &*r == "presentation") {
        return false;
    }
    if sel(t, "caption, thead, th").exists() {
        return true;
    }
    if sel(
        t,
        "td p, td div, td table, td ul, td pre, td h1, td h2, td h3",
    )
    .exists()
    {
        return false;
    }
    let rows = sel(t, "tr");
    let max_cols = rows
        .nodes()
        .iter()
        .map(|r| sel(r, "td").length())
        .max()
        .unwrap_or(0);
    rows.length() >= 2 && max_cols >= 2
}

/// A table as text: cells separated by tabs, rows by newlines.
fn table_text(t: &NodeRef) -> String {
    sel(t, "tr")
        .nodes()
        .iter()
        .map(|row| {
            sel(row, "th, td")
                .nodes()
                .iter()
                .map(|cell| inline_text(cell))
                .collect::<Vec<_>>()
                .join("\t")
        })
        .collect::<Vec<_>>()
        .join("\n")
}
