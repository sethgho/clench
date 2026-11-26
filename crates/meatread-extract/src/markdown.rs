//! Markdown extraction (feature `markdown`).

use meatread_core::{Block, BlockKind, Document, Source};
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::html::fragment_blocks;

use crate::ExtractError;
use crate::text::{block, collapse, readable_words};

/// Extracts a document from Markdown (CommonMark plus tables, footnotes,
/// strikethrough, task lists and YAML front matter).
///
/// Markup syntax is consumed and the words are kept as written: emphasis
/// markers, link destinations, list bullets and heading hashes disappear;
/// `\*` reads as `*`; entities are decoded. Smart punctuation is not applied.
/// Headings, list items (with depth and order), blockquotes (with nesting),
/// fenced and indented code (with the first word of the info string as the
/// language), tables, rules and footnote definitions become typed blocks.
/// A raw HTML block is read as a whole: its visible text becomes blocks (a
/// heading element is a heading, an image's alt text a figure) and its tags
/// and attributes are never read; inline HTML is dropped except `<br>`.
/// `title` and `author` keys of YAML front matter set the title and byline;
/// otherwise the title is the first level-1 heading, else the first heading
/// of any level (including one inside an HTML block), else the file name of
/// `url`.
///
/// # Errors
///
/// [`ExtractError::NoContent`] if the input has no words.
pub fn extract_markdown(md: &str, url: &str) -> Result<Document, ExtractError> {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS;
    let mut conv = Converter::default();
    for event in Parser::new_ext(md, options) {
        conv.event(event);
    }
    conv.flush();
    if let Some(e) = conv.error.take() {
        return Err(e);
    }

    let mut blocks = conv.out;
    blocks.retain(|b| b.kind == BlockKind::Rule || !b.text.trim().is_empty());
    let words = readable_words(&blocks);
    if words == 0 {
        return Err(ExtractError::NoContent);
    }
    let title = conv.front_title.unwrap_or_else(|| {
        let headings = || blocks.iter().filter(|b| b.kind == BlockKind::Heading);
        headings()
            .find(|b| b.level == Some(1))
            .or_else(|| headings().next())
            .map(|b| b.text.clone())
            .or_else(|| file_name_of(url))
            .unwrap_or_default()
    });
    Ok(Document {
        url: url.to_string(),
        title,
        byline: conv.front_author,
        source: Source::Markdown,
        word_count: words,
        blocks,
        ..Document::default()
    })
}

#[derive(Default)]
struct Converter {
    out: Vec<Block>,
    buf: String,
    /// One entry per open list: whether it is ordered.
    lists: Vec<bool>,
    quote: u8,
    footnote: bool,
    heading: Option<u8>,
    code: Option<Option<String>>,
    code_text: String,
    image_depth: u32,
    /// The raw HTML block being collected, delivered line by line.
    html_block: Option<String>,
    /// Set when a raw HTML block was too deeply nested to read.
    error: Option<ExtractError>,
    in_metadata: bool,
    metadata: String,
    front_title: Option<String>,
    front_author: Option<String>,
    table: Option<Vec<Vec<String>>>,
    row: Vec<String>,
}

impl Converter {
    fn text_kind(&self) -> BlockKind {
        if self.footnote {
            BlockKind::Footnote
        } else if !self.lists.is_empty() {
            BlockKind::ListItem
        } else if self.quote > 0 {
            BlockKind::Blockquote
        } else {
            BlockKind::Paragraph
        }
    }

    /// Emits the buffered inline text as a block typed by the context.
    fn flush(&mut self) {
        let text = collapse(&std::mem::take(&mut self.buf));
        if text.is_empty() {
            return;
        }
        let kind = self.text_kind();
        let mut b = block(kind, text);
        b.quote = self.quote;
        if kind == BlockKind::ListItem {
            b.depth = Some(self.lists.len().min(255) as u8);
            b.ordered = self.lists.last().copied();
        }
        self.out.push(b);
    }

    fn event(&mut self, event: Event) {
        if self.in_metadata {
            match event {
                Event::Text(t) => self.metadata.push_str(&t),
                Event::End(TagEnd::MetadataBlock(_)) => {
                    self.in_metadata = false;
                    self.read_front_matter();
                }
                _ => {}
            }
            return;
        }
        if self.image_depth > 0 {
            match event {
                Event::Start(Tag::Image { .. }) => self.image_depth += 1,
                Event::End(TagEnd::Image) => {
                    self.image_depth -= 1;
                    if self.image_depth == 0 {
                        let alt = collapse(&std::mem::take(&mut self.code_text));
                        if !alt.is_empty() {
                            self.out.push(block(BlockKind::Figure, alt));
                        }
                    }
                }
                Event::Text(t) | Event::Code(t) => self.code_text.push_str(&t),
                _ => {}
            }
            return;
        }
        if let Some(raw) = self.html_block.as_mut() {
            match event {
                Event::Html(h) | Event::Text(h) => raw.push_str(&h),
                Event::End(TagEnd::HtmlBlock) => {
                    let raw = self.html_block.take().unwrap_or_default();
                    self.html_blocks(&raw);
                }
                _ => {}
            }
            return;
        }
        match event {
            Event::Start(Tag::HtmlBlock) => {
                self.flush();
                self.html_block = Some(String::new());
            }
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag),
            Event::Text(t) => {
                if self.code.is_some() {
                    self.code_text.push_str(&t);
                } else {
                    self.buf.push_str(&t);
                }
            }
            Event::Code(t) => self.buf.push_str(&t),
            Event::SoftBreak | Event::HardBreak => self.buf.push(' '),
            Event::Html(h) => self.html_blocks(&h),
            Event::InlineHtml(h) => self.inline_html(&h),
            Event::Rule => {
                self.flush();
                self.out.push(block(BlockKind::Rule, String::new()));
            }
            Event::FootnoteReference(_)
            | Event::TaskListMarker(_)
            | Event::InlineMath(_)
            | Event::DisplayMath(_) => {}
        }
    }

    fn start(&mut self, tag: Tag) {
        match tag {
            Tag::Paragraph | Tag::Item => self.flush(),
            Tag::Heading { level, .. } => {
                self.flush();
                self.heading = Some(heading_number(level));
            }
            Tag::BlockQuote(_) => {
                self.flush();
                self.quote = self.quote.saturating_add(1);
            }
            Tag::CodeBlock(kind) => {
                self.flush();
                let lang = match kind {
                    CodeBlockKind::Fenced(info) => info
                        .split_whitespace()
                        .next()
                        .filter(|l| !l.is_empty())
                        .map(str::to_string),
                    CodeBlockKind::Indented => None,
                };
                self.code = Some(lang);
                self.code_text.clear();
            }
            Tag::List(first) => {
                self.flush();
                self.lists.push(first.is_some());
            }
            Tag::FootnoteDefinition(_) => {
                self.flush();
                self.footnote = true;
            }
            Tag::Table(_) => {
                self.flush();
                self.table = Some(Vec::new());
            }
            Tag::TableHead | Tag::TableRow => self.row.clear(),
            Tag::TableCell => self.buf.clear(),
            Tag::Image { .. } => {
                self.image_depth = 1;
                self.code_text.clear();
            }
            Tag::MetadataBlock(_) => {
                self.in_metadata = true;
                self.metadata.clear();
            }
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph | TagEnd::Item => self.flush(),
            TagEnd::Heading(_) => {
                let text = collapse(&std::mem::take(&mut self.buf));
                if let Some(level) = self.heading.take() {
                    if !text.is_empty() {
                        let mut b = block(BlockKind::Heading, text);
                        b.level = Some(level);
                        b.quote = self.quote;
                        self.out.push(b);
                    }
                }
            }
            TagEnd::BlockQuote(_) => {
                self.flush();
                self.quote = self.quote.saturating_sub(1);
            }
            TagEnd::CodeBlock => {
                let lang = self.code.take().flatten();
                let text = std::mem::take(&mut self.code_text);
                let text = text.trim_end_matches(['\n', '\r']).to_string();
                if !text.trim().is_empty() {
                    let mut b = block(BlockKind::Code, text);
                    b.lang = lang;
                    b.quote = self.quote;
                    self.out.push(b);
                }
            }
            TagEnd::List(_) => {
                self.flush();
                self.lists.pop();
            }
            TagEnd::FootnoteDefinition => {
                self.flush();
                self.footnote = false;
            }
            TagEnd::TableCell => {
                let cell = collapse(&std::mem::take(&mut self.buf));
                self.row.push(cell);
            }
            TagEnd::TableHead | TagEnd::TableRow => {
                let row = std::mem::take(&mut self.row);
                if let Some(t) = self.table.as_mut() {
                    t.push(row);
                }
            }
            TagEnd::Table => {
                if let Some(rows) = self.table.take() {
                    let text = rows
                        .iter()
                        .map(|r| r.join("\t"))
                        .collect::<Vec<_>>()
                        .join("\n");
                    let mut b = block(BlockKind::Table, text);
                    b.quote = self.quote;
                    self.out.push(b);
                }
            }
            _ => {}
        }
    }

    /// A complete raw HTML block: its visible text as typed blocks.
    fn html_blocks(&mut self, raw: &str) {
        self.flush();
        let blocks = match fragment_blocks(raw) {
            Ok(blocks) => blocks,
            Err(e) => {
                self.error.get_or_insert(e);
                return;
            }
        };
        for mut b in blocks {
            b.quote = self.quote;
            if self.footnote && b.kind == BlockKind::Paragraph {
                b.kind = BlockKind::Footnote;
                b.skip = false;
            }
            self.out.push(b);
        }
    }

    /// An inline HTML tag: never text, except that `<br>` separates words.
    fn inline_html(&mut self, tag: &str) {
        let lower = tag.trim_start().to_ascii_lowercase();
        if lower.starts_with("<br") && !self.buf.is_empty() && !self.buf.ends_with(' ') {
            self.buf.push(' ');
        }
    }

    fn read_front_matter(&mut self) {
        for line in self.metadata.lines() {
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            let value = value.trim().trim_matches(['"', '\'']);
            let value = collapse(value);
            if value.is_empty() {
                continue;
            }
            match key.trim().to_ascii_lowercase().as_str() {
                "title" => self.front_title = Some(value),
                "author" => self.front_author = Some(value),
                _ => {}
            }
        }
        self.metadata.clear();
    }
}

/// The last path segment of `url` without its extension, if there is one.
fn file_name_of(url: &str) -> Option<String> {
    let end = url.find(['?', '#']).unwrap_or(url.len());
    let url = &url[..end];
    let path = match url.split_once("://") {
        Some((_, rest)) => rest.split_once('/')?.1,
        None => url,
    };
    let name = path.trim_end_matches('/').rsplit('/').next()?;
    let stem = match name.rsplit_once('.') {
        Some((stem, _)) if !stem.is_empty() => stem,
        _ => name,
    };
    (!stem.is_empty()).then(|| stem.to_string())
}

fn heading_number(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}
