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

/// Typographic and structural evidence about paragraphs, keyed by their
/// text. Edge-trimming rules combine it with wording; wording alone never
/// drops text.
#[derive(Default)]
pub(super) struct Evidence {
    /// Paragraphs that look like an author or syndication note: set entirely
    /// in italics and containing a link.
    pub closing_notes: HashSet<String>,
    /// Paragraphs whose whole text is a link or a `<time>`, the way page
    /// furniture such as comment counts and relative timestamps is marked up.
    pub link_only: HashSet<String>,
    /// Paragraphs inside an element whose class or id names an
    /// advertisement or sponsorship slot.
    pub in_ad: HashSet<String>,
}

/// The blocks of a walked DOM with their evidence.
pub(super) struct Walked {
    pub blocks: Vec<Block>,
    pub evidence: Evidence,
}

/// Walks the children of `root` into blocks.
pub(super) fn blocks(root: &NodeRef) -> Walked {
    let mut w = Walker::default();
    w.walk(root, Ctx::default());
    w.flush(Ctx::default());
    Walked {
        blocks: w.out,
        evidence: w.evidence,
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
    /// Inside `<a>` or `<time>`.
    linked: bool,
    /// Inside an advertisement or sponsorship slot.
    in_ad: bool,
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

/// Elements nested deeper than this are read as flat text instead of being
/// walked, which bounds the walker's recursion however deep the tree is.
/// Real pages stay far below it.
const MAX_WALK_DEPTH: usize = 256;

#[derive(Default)]
struct Walker {
    depth: usize,
    out: Vec<Block>,
    buf: String,
    evidence: Evidence,
    /// The buffered text holds some text outside italics.
    has_roman: bool,
    /// The buffered text holds some text outside links.
    has_unlinked: bool,
    /// The buffered text holds a link.
    has_link: bool,
}

impl Walker {
    /// Emits buffered inline text as text blocks typed by the context.
    fn flush(&mut self, ctx: Ctx) {
        let buf = std::mem::take(&mut self.buf);
        let note = !self.has_roman && self.has_link;
        let link_only = !self.has_unlinked && self.has_link;
        self.has_unlinked = false;
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
                self.evidence.closing_notes.insert(text.clone());
            }
            if ctx.in_ad && kind == BlockKind::Paragraph {
                self.evidence.in_ad.insert(text.clone());
            }
            if link_only && kind == BlockKind::Paragraph {
                self.evidence.link_only.insert(text.clone());
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
        if self.depth >= MAX_WALK_DEPTH {
            let text = node.text();
            if !text.trim().is_empty() {
                self.has_roman |= !ctx.italic;
                self.has_unlinked |= !ctx.linked;
                self.buf.push_str(&text);
            }
            return;
        }
        self.depth += 1;
        self.walk_children(node, ctx);
        self.depth -= 1;
    }

    fn walk_children(&mut self, node: &NodeRef, ctx: Ctx) {
        for child in node.children() {
            if child.is_text() {
                let text = child.text();
                if !text.trim().is_empty() {
                    self.has_roman |= !ctx.italic;
                    self.has_unlinked |= !ctx.linked;
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
        let ctx = Ctx {
            in_ad: ctx.in_ad || super::chrome::names_an_ad_slot(child),
            ..ctx
        };
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
            "figure" if is_wrapper_figure(child) => {
                // Figures used as wrappers (code listings, pull quotes, poems).
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
            "a" | "time" => {
                self.has_link = true;
                let inner = Ctx {
                    linked: true,
                    ..ctx
                };
                self.walk(child, inner);
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
        "script" | "style" | "noscript" | "template" | "input" | "select" | "textarea" | "nav"
        | "img" | "video" | "audio" | "iframe" | "svg" | "canvas" | "object" | "embed" | "wbr" => {
            true
        }
        // Controls are interface unless they sit inside a sentence
        // (link-styled buttons) or a form wraps article paragraphs (legacy
        // page-wide forms).
        "button" => !super::chrome::has_adjacent_text(n),
        "form" => !super::chrome::has_long_paragraph(n, FORM_MAX_PARAGRAPH_WORDS),
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
    super::shield::class_value(n)
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

/// A form holding a paragraph at least this long is content, not a control.
const FORM_MAX_PARAGRAPH_WORDS: usize = 40;

/// A footnote marker is at most this many characters ("12", "[a]", "†").
const FOOTNOTE_MARKER_MAX_CHARS: usize = 5;

/// A caption or credit has at most this many words.
const CAPTION_MAX_WORDS: usize = 40;

/// Image credits and captions rendered outside a `<figure>`.
///
/// A class naming a caption or credit is not enough on its own (utility
/// classes such as `has-caption-styles` sit on whole article containers), so
/// the element must also be short, hold no block-level children and be the
/// sole text of its block.
fn is_credit(n: &NodeRef) -> bool {
    has_class(n, |c| {
        !["has-", "is-", "with-", "no-"]
            .iter()
            .any(|p| c.starts_with(p))
            && c.split(['-', '_'])
                .any(|part| matches!(part, "credit" | "credits" | "caption"))
    }) && super::chrome::is_short_leaf(n, CAPTION_MAX_WORDS)
        && super::chrome::owns_its_block(n)
}

fn is_footnote_ref(sup: &NodeRef) -> bool {
    let a = sel(sup, "a");
    let href = a.attr("href").unwrap_or_default();
    let text = collapse(&sup.text());
    a.length() == 1
        && href.starts_with('#')
        && text == collapse(&a.text())
        && text.chars().count() <= FOOTNOTE_MARKER_MAX_CHARS
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

/// A figure that holds text structure of its own outside its caption. A
/// paragraph that wraps the picture (which Readability makes out of picture
/// containers) is part of the picture, not text.
fn is_wrapper_figure(figure: &NodeRef) -> bool {
    sel(
        figure,
        "pre, blockquote, table, ul, ol, p, h1, h2, h3, h4, h5, h6, dl",
    )
    .nodes()
    .iter()
    .any(|n| {
        !n.ancestors_it(None).any(|a| a.has_name("figcaption")) && !sel(n, "img, picture").exists()
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
