//! # clench-ffi
//!
//! Native bindings for clench, generated with [UniFFI]. The Apple app
//! (`apps/apple`) uses them through the Swift module `ClenchFFI`; any other
//! UniFFI language works the same way. All reading logic stays in the Rust
//! engine, and the surface mirrors `clench-wasm`:
//!
//! * [`extract`] turns fetched bytes into a [`Document`].
//! * [`Session`], built from a `Document` and optional settings, owns a
//!   player and mirrors its controls. It never reads a clock: the host
//!   reports elapsed time with [`Session::advance`] and asks
//!   [`Session::next_change_in`] when to wake up.
//! * [`Session::outline`], [`Session::terrain`], [`Session::position`] and
//!   [`Session::sentence`] describe the document map, sections and the
//!   sentence being read; [`reading_ms`] turns a stored [`Timing`] into a
//!   duration at any speed.
//! * [`version`] is the engine version.
//!
//! Settings are the same JSON object the WebAssembly `Session` accepts (see
//! [`clench_core::session::Settings`]), so a stored preference means the
//! same thing in every host.
//!
//! ## Threads and crashes
//!
//! A `Session` is safe to share between threads; calls are serialized by
//! an internal lock. [`extract`] runs on a dedicated thread with a large
//! stack, so it is safe to call from a thread with a small stack (an app
//! extension, a Swift task). A panic during extraction is reported as
//! [`ClenchError::Crashed`]. Build with `panic = "unwind"` (the workspace's
//! `apple` profile does) so that a panic elsewhere surfaces as a UniFFI
//! internal error rather than terminating the process.
//!
//! [UniFFI]: https://mozilla.github.io/uniffi-rs/latest/

use std::sync::{Mutex, MutexGuard};
use std::thread;

use clench_core::session::{ContextView, Settings, build_outline, build_player, word_count};
pub use clench_core::{BlockKind, PlayState, Source};
use clench_core::{Pacing, Player};

uniffi::setup_scaffolding!();

/// Stack for the extraction thread. HTML parsing recurses per nesting
/// level; the extractor refuses absurd nesting, and this leaves ample room
/// below that limit on any platform.
const EXTRACT_STACK_BYTES: usize = 64 * 1024 * 1024;

/// Structural role of a block.
#[uniffi::remote(Enum)]
pub enum BlockKind {
    Heading,
    Paragraph,
    ListItem,
    Blockquote,
    Code,
    Table,
    Figure,
    Caption,
    Footnote,
    Rule,
}

/// Where a document's content came from.
#[uniffi::remote(Enum)]
pub enum Source {
    Readability,
    JsonLd,
    MetaDescription,
    CitationAbstract,
    PlainText,
    Markdown,
    Pdf,
    None,
}

/// Playback state.
#[uniffi::remote(Enum)]
pub enum PlayState {
    Paused,
    Playing,
    Finished,
}

/// Why a call failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum ClenchError {
    /// The input was readable but held no text to read.
    #[error("no readable text found")]
    NoContent,
    /// The content type cannot be read as text.
    #[error("unsupported content type: {content_type}")]
    UnsupportedType {
        /// The content type or the sniffed kind.
        content_type: String,
    },
    /// The format is recognised but this build leaves it out.
    #[error("support for this format needs the `{feature}` feature")]
    FeatureDisabled {
        /// The Cargo feature that would enable it.
        feature: String,
    },
    /// The PDF could not be parsed or has no text layer.
    #[error("could not read PDF: {reason}")]
    Pdf {
        /// Why.
        reason: String,
    },
    /// The page nests elements deeper than the extractor accepts.
    #[error("the page nests elements more than {limit} levels deep")]
    TooDeeplyNested {
        /// The deepest nesting accepted.
        limit: u64,
    },
    /// The settings JSON is malformed or has unknown fields.
    #[error("invalid settings: {reason}")]
    InvalidSettings {
        /// The parser's message.
        reason: String,
    },
    /// The engine panicked. The input that caused it should be reported.
    #[error("clench engine crashed: {reason}")]
    Crashed {
        /// The panic message, when there is one.
        reason: String,
    },
}

impl From<clench_extract::ExtractError> for ClenchError {
    fn from(err: clench_extract::ExtractError) -> ClenchError {
        use clench_extract::ExtractError as E;
        match err {
            E::NoContent => ClenchError::NoContent,
            E::UnsupportedType(content_type) => ClenchError::UnsupportedType { content_type },
            E::FeatureDisabled(feature) => ClenchError::FeatureDisabled {
                feature: feature.to_string(),
            },
            E::Pdf(reason) => ClenchError::Pdf { reason },
            E::TooDeeplyNested { limit } => ClenchError::TooDeeplyNested {
                limit: limit as u64,
            },
        }
    }
}

/// One structural unit of a document. The text is the author's, verbatim.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Block {
    /// Structural role.
    pub kind: BlockKind,
    /// The author's text.
    pub text: String,
    /// Heading level, 1 to 6.
    pub level: Option<u8>,
    /// List nesting depth, from 1.
    pub depth: Option<u8>,
    /// Whether a list item belongs to an ordered list.
    pub ordered: Option<bool>,
    /// Blockquote nesting depth; 0 outside quotes.
    pub quote: u8,
    /// Language tag (for code, the programming language).
    pub lang: Option<String>,
    /// The extractor suggests skipping this block.
    pub skip: bool,
}

/// A readable document: metadata plus blocks in reading order.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Document {
    /// The document's address; may be empty.
    pub url: String,
    /// Title; may be empty.
    pub title: String,
    /// Author line.
    pub byline: Option<String>,
    /// Publication or site name.
    pub site_name: Option<String>,
    /// Language tag.
    pub lang: Option<String>,
    /// Short summary.
    pub excerpt: Option<String>,
    /// Where the content came from.
    pub source: Source,
    /// The content is known or suspected to be incomplete.
    pub partial: bool,
    /// The page looked paywalled.
    pub paywall_hint: bool,
    /// Whitespace-delimited word count of the blocks a default script reads.
    pub word_count: u64,
    /// Blocks in reading order.
    pub blocks: Vec<Block>,
}

impl From<clench_core::Block> for Block {
    fn from(b: clench_core::Block) -> Block {
        Block {
            kind: b.kind,
            text: b.text,
            level: b.level,
            depth: b.depth,
            ordered: b.ordered,
            quote: b.quote,
            lang: b.lang,
            skip: b.skip,
        }
    }
}

impl From<Block> for clench_core::Block {
    fn from(b: Block) -> clench_core::Block {
        let mut out = clench_core::Block::new(b.kind, b.text);
        out.level = b.level;
        out.depth = b.depth;
        out.ordered = b.ordered;
        out.quote = b.quote;
        out.lang = b.lang;
        out.skip = b.skip;
        out
    }
}

impl From<clench_core::Document> for Document {
    fn from(d: clench_core::Document) -> Document {
        Document {
            url: d.url,
            title: d.title,
            byline: d.byline,
            site_name: d.site_name,
            lang: d.lang,
            excerpt: d.excerpt,
            source: d.source,
            partial: d.partial,
            paywall_hint: d.paywall_hint,
            word_count: d.word_count as u64,
            blocks: d.blocks.into_iter().map(Block::from).collect(),
        }
    }
}

impl From<Document> for clench_core::Document {
    fn from(d: Document) -> clench_core::Document {
        let mut out = clench_core::Document::from_blocks(
            d.blocks.into_iter().map(clench_core::Block::from).collect(),
        );
        out.url = d.url;
        out.title = d.title;
        out.byline = d.byline;
        out.site_name = d.site_name;
        out.lang = d.lang;
        out.excerpt = d.excerpt;
        out.source = d.source;
        out.partial = d.partial;
        out.paywall_hint = d.paywall_hint;
        out.word_count = usize::try_from(d.word_count).unwrap_or(usize::MAX);
        out
    }
}

/// Everything needed to draw the current word.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Frame {
    /// Text before the pivot.
    pub before: String,
    /// The pivot grapheme cluster, drawn at the fixation point.
    pub pivot: String,
    /// Text after the pivot.
    pub after: String,
    /// Index of the token.
    pub index: u64,
    /// Number of tokens.
    pub total: u64,
    /// Index of the token's block in the document.
    pub block: u64,
    /// Kind of that block.
    pub block_kind: BlockKind,
    /// The token is an engine marker such as `[code]`, not author text.
    pub is_marker: bool,
    /// The token is the first one shown from its block.
    pub block_start: bool,
    /// Fraction of the script before this token, 0 to 1.
    pub progress: f32,
    /// Estimated time left at the current WPM, ms.
    pub remaining_ms: u64,
    /// Playback state.
    pub state: PlayState,
    /// Current words per minute.
    pub wpm: u32,
}

/// The current token's block, split around the token.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct Context {
    /// Index of the block in the document.
    pub block: u64,
    /// The block's kind.
    pub kind: BlockKind,
    /// Block text before the current token.
    pub before: String,
    /// The current token; for a marker, the whole skipped block.
    pub token: String,
    /// Block text after the current token.
    pub after: String,
}

/// What a session leaves out for one block kind.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct Omitted {
    /// The block kind.
    pub kind: BlockKind,
    /// Blocks shown as a single marker such as `[code]`.
    pub marked: u64,
    /// Words inside the marked blocks.
    pub marked_words: u64,
    /// Blocks left out without a trace.
    pub skipped: u64,
    /// Words inside the skipped blocks.
    pub skipped_words: u64,
}

/// Reading time that does not depend on the speed; turn it into
/// milliseconds with [`reading_ms`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, uniffi::Record)]
pub struct Timing {
    /// Summed word durations in thousandths of the base duration
    /// `60000 / wpm` ms.
    pub beats: u64,
    /// Time that does not scale with the speed (markers), ms.
    pub fixed_ms: u64,
}

impl From<clench_core::Timing> for Timing {
    fn from(t: clench_core::Timing) -> Timing {
        Timing {
            beats: t.beats,
            fixed_ms: t.fixed_ms,
        }
    }
}

impl From<Timing> for clench_core::Timing {
    fn from(t: Timing) -> clench_core::Timing {
        clench_core::Timing {
            beats: t.beats,
            fixed_ms: t.fixed_ms,
        }
    }
}

/// A run of blocks starting at a section heading, or the text before the
/// first one.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct OutlineSection {
    /// The heading text; `None` for text before the first section heading.
    pub title: Option<String>,
    /// The heading level.
    pub level: Option<u8>,
    /// First token of the section.
    pub token_start: u64,
    /// One past the section's last token.
    pub token_end: u64,
    /// Number of blocks in the section, heading included.
    pub blocks: u64,
    /// Author words in the section.
    pub words: u64,
    /// Reading time of the section.
    pub timing: Timing,
}

/// The sections and totals of a session's script.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct Outline {
    /// Sections in order; every token belongs to exactly one.
    pub sections: Vec<OutlineSection>,
    /// Number of blocks.
    pub blocks: u64,
    /// Author words read.
    pub words: u64,
    /// Number of tokens.
    pub tokens: u64,
    /// Number of sentences.
    pub sentences: u64,
    /// Reading time of the whole script.
    pub timing: Timing,
}

/// One mark on a document map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct TerrainTick {
    /// The tick is a section heading.
    pub heading: bool,
    /// Kind of the tick's first block.
    pub kind: BlockKind,
    /// Index of the tick's section in [`Outline::sections`].
    pub section: u64,
    /// First token the tick covers; seek here to jump to it.
    pub token_start: u64,
    /// One past the last token the tick covers.
    pub token_end: u64,
    /// Author words the tick covers.
    pub words: u64,
    /// Size relative to the largest non-heading tick, 0 to 1000; headings
    /// are 1000.
    pub weight: u16,
}

impl From<clench_core::Tick> for TerrainTick {
    fn from(t: clench_core::Tick) -> TerrainTick {
        TerrainTick {
            heading: t.heading,
            kind: t.kind,
            section: t.section as u64,
            token_start: t.token_start as u64,
            token_end: t.token_end as u64,
            words: t.words as u64,
            weight: t.weight,
        }
    }
}

/// Where the session is, in outline terms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct ReadingPosition {
    /// Index of the current token.
    pub token: u64,
    /// Index of the current section in [`Outline::sections`]; `None` when
    /// finished.
    pub section: Option<u64>,
    /// Index of the current tick in a terrain of the size asked for; `None`
    /// when finished.
    pub tick: Option<u64>,
    /// Reading time from the current word to the end, current word
    /// included.
    pub remaining: Timing,
    /// Reading time from the current word to the end of its section.
    pub section_remaining: Timing,
}

/// The sentence around the current word.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SentenceView {
    /// Sentence text before the current word.
    pub before: String,
    /// The current word (for a marker, its label).
    pub token: String,
    /// Sentence text after the current word.
    pub after: String,
    /// Ordinal of the sentence in the whole script.
    pub sentence: u64,
    /// Ordinal of the sentence in its block.
    pub index_in_block: u64,
    /// Words of each sentence in the block; its length is the number of
    /// sentences in the block.
    pub words: Vec<u64>,
}

/// Milliseconds that `timing` takes at `wpm`, clamped to the engine's
/// default speed range (100 to 1500).
#[uniffi::export]
pub fn reading_ms(timing: Timing, wpm: u32) -> u64 {
    clench_core::Timing::from(timing).ms_with(&Pacing::default(), wpm)
}

/// The version of the clench engine.
#[uniffi::export]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Extracts a readable document from fetched bytes. `content_type` is the
/// raw Content-Type header (it may carry a charset); `url` is the
/// document's address, used to record it and to resolve links.
///
/// # Errors
///
/// When the input holds no readable text, its type is unsupported, or the
/// engine panicked on it.
#[uniffi::export]
pub fn extract(
    bytes: Vec<u8>,
    content_type: Option<String>,
    url: Option<String>,
) -> Result<Document, ClenchError> {
    let worker = thread::Builder::new()
        .name("clench-extract".into())
        .stack_size(EXTRACT_STACK_BYTES)
        .spawn(move || {
            clench_extract::extract(
                &bytes,
                content_type.as_deref(),
                url.as_deref().unwrap_or(""),
            )
        })
        .map_err(|e| ClenchError::Crashed {
            reason: format!("could not start the extraction thread: {e}"),
        })?;
    match worker.join() {
        Ok(result) => Ok(result?.into()),
        Err(payload) => Err(ClenchError::Crashed {
            reason: panic_message(payload.as_ref()),
        }),
    }
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic".to_string()
    }
}

/// A reading session: a document turned into a timed script and a player
/// positioned in it. Starts paused at the first word (finished if the
/// document has nothing to read).
#[derive(uniffi::Object)]
pub struct Session {
    player: Mutex<Player>,
    outline: clench_core::Outline,
    words: u64,
}

impl Session {
    /// The player, recovering it if an earlier call panicked while holding
    /// the lock: every player method leaves it in a consistent state.
    fn player(&self) -> MutexGuard<'_, Player> {
        self.player.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[uniffi::export]
impl Session {
    /// Builds a session for `document`. `settings_json` is the settings
    /// object shared with the WebAssembly bindings, for example
    /// `{"wpm": 450}`; `None` uses the defaults.
    ///
    /// # Errors
    ///
    /// [`ClenchError::InvalidSettings`] when the settings do not parse.
    #[uniffi::constructor]
    pub fn new(document: Document, settings_json: Option<String>) -> Result<Session, ClenchError> {
        let settings: Settings = match settings_json.as_deref() {
            Some(json) => serde_json::from_str(json).map_err(|e| ClenchError::InvalidSettings {
                reason: e.to_string(),
            })?,
            None => Settings::default(),
        };
        let doc: clench_core::Document = document.into();
        let player = build_player(&doc, &settings);
        let outline = build_outline(&doc, &player);
        let words = word_count(player.script()) as u64;
        Ok(Session {
            player: Mutex::new(player),
            outline,
            words,
        })
    }

    /// Starts or resumes playback (with ease-in). From finished, restarts.
    pub fn play(&self) {
        self.player().play();
    }

    /// Pauses on the current word.
    pub fn pause(&self) {
        self.player().pause();
    }

    /// Pauses if playing, otherwise plays.
    pub fn toggle(&self) {
        self.player().toggle();
    }

    /// Returns to the first word, keeping play or pause.
    pub fn restart(&self) {
        self.player().restart();
    }

    /// Moves `delta` tokens forward (negative: back).
    pub fn step(&self, delta: i32) {
        self.player().step(delta as isize);
    }

    /// Jumps to the start of the next sentence.
    pub fn next_sentence(&self) {
        self.player().next_sentence();
    }

    /// Jumps to the start of this sentence, or the previous one if already
    /// at its start.
    pub fn prev_sentence(&self) {
        self.player().prev_sentence();
    }

    /// Jumps to the first word of the next block.
    pub fn next_paragraph(&self) {
        self.player().next_paragraph();
    }

    /// Jumps to the first word of this block, or the previous one if
    /// already at its start.
    pub fn prev_paragraph(&self) {
        self.player().prev_paragraph();
    }

    /// Jumps to token `index`, clamped.
    pub fn seek(&self, index: u64) {
        self.player()
            .seek(usize::try_from(index).unwrap_or(usize::MAX));
    }

    /// Jumps to the token `fraction` (0 to 1) of the way through.
    pub fn seek_fraction(&self, fraction: f32) {
        self.player().seek_fraction(fraction);
    }

    /// Sets the words per minute (clamped), keeping the position.
    pub fn set_wpm(&self, wpm: u32) {
        self.player().set_wpm(wpm);
    }

    /// Changes the words per minute by `delta` (clamped).
    pub fn nudge_wpm(&self, delta: i32) {
        self.player().nudge_wpm(delta);
    }

    /// Reports `ms` of elapsed time. Returns whether the current word or
    /// the state changed.
    pub fn advance(&self, ms: u32) -> bool {
        self.player().advance(ms)
    }

    /// Milliseconds until the current word changes, or `None` unless
    /// playing.
    pub fn next_change_in(&self) -> Option<u32> {
        self.player().next_change_in()
    }

    /// The current word and playback status.
    pub fn frame(&self) -> Frame {
        let player = self.player();
        let f = player.frame();
        Frame {
            before: f.before.to_string(),
            pivot: f.pivot.to_string(),
            after: f.after.to_string(),
            index: f.index as u64,
            total: f.total as u64,
            block: f.block as u64,
            block_kind: f.block_kind,
            is_marker: f.is_marker,
            block_start: f.block_start,
            progress: f.progress,
            remaining_ms: f.remaining_ms,
            state: f.state,
            wpm: f.wpm,
        }
    }

    /// The current word's block, for a pause view; `None` if the document
    /// has nothing to read.
    pub fn context(&self) -> Option<Context> {
        let player = self.player();
        player.context().map(|ctx| {
            let view = ContextView::from(ctx);
            Context {
                block: view.block as u64,
                kind: view.kind,
                before: view.before.to_string(),
                token: view.token.to_string(),
                after: view.after.to_string(),
            }
        })
    }

    /// What the session leaves out, one entry per block kind that had any
    /// marked or silently skipped blocks.
    pub fn omitted(&self) -> Vec<Omitted> {
        self.player()
            .script()
            .omitted()
            .iter()
            .map(|o| Omitted {
                kind: o.kind,
                marked: o.marked as u64,
                marked_words: o.marked_words as u64,
                skipped: o.skipped as u64,
                skipped_words: o.skipped_words as u64,
            })
            .collect()
    }

    /// Playback state.
    pub fn state(&self) -> PlayState {
        self.player().state()
    }

    /// Current words per minute.
    pub fn wpm(&self) -> u32 {
        self.player().wpm()
    }

    /// Index of the current token.
    pub fn index(&self) -> u64 {
        self.player().index() as u64
    }

    /// Number of tokens (words, long-word pieces and markers).
    pub fn token_count(&self) -> u64 {
        self.player().script().len() as u64
    }

    /// Number of author words that will be read (markers and the later
    /// pieces of split long words are not counted).
    pub fn word_count(&self) -> u64 {
        self.words
    }

    /// Estimated reading time of the whole script at the current WPM, ms,
    /// excluding ease-in.
    pub fn total_ms(&self) -> u64 {
        self.player().total_ms()
    }

    /// Estimated time left at the current WPM, ms, excluding ease-in.
    pub fn remaining_ms(&self) -> u64 {
        self.player().remaining_ms()
    }

    /// The script's sections and totals. The outline does not change during
    /// a session.
    pub fn outline(&self) -> Outline {
        let o = &self.outline;
        Outline {
            sections: o
                .sections
                .iter()
                .map(|s| OutlineSection {
                    title: s.title.clone(),
                    level: s.level,
                    token_start: s.token_start as u64,
                    token_end: s.token_end as u64,
                    blocks: (s.block_end - s.block_start) as u64,
                    words: s.words as u64,
                    timing: s.timing.into(),
                })
                .collect(),
            blocks: o.blocks.len() as u64,
            words: o.words as u64,
            tokens: o.tokens as u64,
            sentences: o.sentences as u64,
            timing: o.timing.into(),
        }
    }

    /// A document map of at most `max_ticks` ticks (or two per section if
    /// that is more; 0 means one per block): section headings and runs of
    /// the blocks between them, sized by word count.
    pub fn terrain(&self, max_ticks: u32) -> Vec<TerrainTick> {
        self.outline
            .terrain(max_ticks as usize)
            .into_iter()
            .map(TerrainTick::from)
            .collect()
    }

    /// Where the session is, with the tick index for a terrain of
    /// `max_ticks` (the same value passed to [`Session::terrain`]).
    pub fn position(&self, max_ticks: u32) -> ReadingPosition {
        let ticks = self.outline.terrain(max_ticks as usize);
        let player = self.player();
        let p = clench_core::session::position(&player, &self.outline, &ticks);
        ReadingPosition {
            token: p.token as u64,
            section: p.section.map(|s| s as u64),
            tick: p.tick.map(|t| t as u64),
            remaining: p.remaining.into(),
            section_remaining: p.section_remaining.into(),
        }
    }

    /// The sentence around the current word, for a sentence strip; `None`
    /// if the document has nothing to read.
    pub fn sentence(&self) -> Option<SentenceView> {
        let player = self.player();
        player
            .script()
            .sentence_view(player.index())
            .map(|v| SentenceView {
                before: v.before.to_string(),
                token: v.token.to_string(),
                after: v.after.to_string(),
                sentence: v.sentence as u64,
                index_in_block: v.index_in_block as u64,
                words: v.words.iter().map(|&w| w as u64).collect(),
            })
    }
}
