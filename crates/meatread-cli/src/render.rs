//! Text renderings of documents and scripts.

use std::fmt::Write;

use meatread_core::{Block, BlockKind, Document, Pacing, Pause, Player, Script};

const PREVIEW_CHARS: usize = 90;

fn kind_name(kind: BlockKind) -> &'static str {
    match kind {
        BlockKind::Heading => "heading",
        BlockKind::Paragraph => "paragraph",
        BlockKind::ListItem => "list_item",
        BlockKind::Blockquote => "blockquote",
        BlockKind::Code => "code",
        BlockKind::Table => "table",
        BlockKind::Figure => "figure",
        BlockKind::Caption => "caption",
        BlockKind::Footnote => "footnote",
        BlockKind::Rule => "rule",
    }
}

fn flags(b: &Block) -> String {
    let mut f = Vec::new();
    if let Some(level) = b.level {
        f.push(format!("h{level}"));
    }
    if let Some(depth) = b.depth {
        let marker = if b.ordered == Some(true) { "ol" } else { "ul" };
        f.push(format!("{marker}{depth}"));
    }
    if b.quote > 0 {
        f.push(format!("q{}", b.quote));
    }
    if let Some(lang) = &b.lang {
        f.push(format!("lang={lang}"));
    }
    if b.skip {
        f.push("skip".to_string());
    }
    f.join(",")
}

fn preview(text: &str) -> String {
    let one_line: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if one_line.chars().count() <= PREVIEW_CHARS {
        one_line
    } else {
        let cut: String = one_line.chars().take(PREVIEW_CHARS).collect();
        format!("{cut}...")
    }
}

/// A readable block listing with a metadata header.
pub fn listing(doc: &Document) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "title:    {}", doc.title);
    if let Some(v) = &doc.byline {
        let _ = writeln!(out, "byline:   {v}");
    }
    if let Some(v) = &doc.site_name {
        let _ = writeln!(out, "site:     {v}");
    }
    if let Some(v) = &doc.lang {
        let _ = writeln!(out, "lang:     {v}");
    }
    let _ = writeln!(
        out,
        "source:   {:?}   words: {}   partial: {}   paywall_hint: {}",
        doc.source, doc.word_count, doc.partial, doc.paywall_hint
    );
    let _ = writeln!(out, "blocks:   {}", doc.blocks.len());
    out.push('\n');
    for (i, b) in doc.blocks.iter().enumerate() {
        let _ = writeln!(
            out,
            "{i:>4}  {:<10} {:<12} {}",
            kind_name(b.kind),
            flags(b),
            preview(&b.text)
        );
    }
    out
}

/// The text a reader plays by default: one block per line, skipped blocks
/// left out.
pub fn reading_text(doc: &Document) -> String {
    let mut out = String::new();
    for b in doc.blocks.iter().filter(|b| !b.skip) {
        let one_line: String = b.text.split_whitespace().collect::<Vec<_>>().join(" ");
        let _ = writeln!(out, "{one_line}");
    }
    out
}

fn pause_name(p: Pause) -> &'static str {
    match p {
        Pause::None => "-",
        Pause::Clause => "clause",
        Pause::Sentence => "sentence",
        Pause::Paragraph => "paragraph",
    }
}

/// One line per token: index, text, duration and pause kind.
pub fn schedule(script: &Script, pacing: &Pacing) -> String {
    let mut out = String::new();
    for (i, token) in script.tokens().iter().enumerate() {
        let _ = writeln!(
            out,
            "{i:>6}  {:<24} {:>5} ms  {}",
            script.text(i),
            pacing.token_ms(token),
            pause_name(token.pause)
        );
    }
    out
}

/// The closing summary line of `script`: size, estimated time, and what the
/// script leaves out ("not read: 12 code marked (310 words), 5 caption
/// skipped (60 words)").
pub fn summary(player: &Player) -> String {
    let total = player.total_ms();
    let secs = total / 1000;
    let mut line = format!(
        "{} tokens at {} wpm: estimated reading time {}m{:02}s",
        player.script().len(),
        player.wpm(),
        secs / 60,
        secs % 60
    );
    let mut parts = Vec::new();
    for o in player.script().omitted() {
        let name = kind_name(o.kind);
        if o.marked > 0 {
            parts.push(format!(
                "{} {name} marked ({} words)",
                o.marked, o.marked_words
            ));
        }
        if o.skipped > 0 {
            parts.push(format!(
                "{} {name} skipped ({} words)",
                o.skipped, o.skipped_words
            ));
        }
    }
    if !parts.is_empty() {
        let _ = write!(line, "; not read: {}", parts.join(", "));
    }
    line
}
