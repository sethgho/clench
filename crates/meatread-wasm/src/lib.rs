//! # meatread-wasm
//!
//! WebAssembly bindings for meatread, built with `wasm-bindgen`. The same
//! module serves every JavaScript host (terminal, web page, browser
//! extension); all reading logic stays in the Rust engine.
//!
//! The JavaScript surface is two things:
//!
//! * `extract(bytes, contentType, url)` turns fetched bytes into a
//!   `Document` (the core document model as plain data).
//! * `Session`, built from a `Document` and optional `Settings`, owns a
//!   player and mirrors its controls. It never reads a clock: the host
//!   reports elapsed time with `advance(ms)` and asks `nextChangeIn()` when
//!   to wake up. `frame()` and `context()` return plain objects.
//!
//! Data objects use the core crate's serde shape (snake_case field names),
//! so a `Document` here is identical to the JSON the native tools print.
//! Errors are thrown as JavaScript `Error`s with readable messages.
//!
//! ## Crashes
//!
//! Release builds abort on panic, which traps the WebAssembly instance and
//! leaves its memory in an unknown state. A panic hook turns the panic into
//! a thrown `Error` whose message starts with `meatread engine crashed:` and
//! marks the instance as crashed; afterwards `extract` and `new Session`
//! throw at once. A host that sees such an error (or a
//! `WebAssembly.RuntimeError`) must not keep using that instance and should
//! load a fresh one if it needs to continue.
//!
//! ## Features
//!
//! `markdown` and `pdf` (both default) mirror the extractor's features, so a
//! size-sensitive build can leave PDF support out.

mod session;

use std::sync::Once;
use std::sync::atomic::{AtomicBool, Ordering};

use meatread_core::{Document, Player};
use serde::Serialize;
use serde::de::DeserializeOwned;
use wasm_bindgen::prelude::*;

pub use session::{ContextView, Settings, build_player, word_count};

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &'static str = r#"
/** Structural role of a block. */
export type BlockKind =
  | "heading" | "paragraph" | "list_item" | "blockquote" | "code"
  | "table" | "figure" | "caption" | "footnote" | "rule";

/** Where a document's content came from. */
export type Source =
  | "readability" | "json_ld" | "meta_description" | "citation_abstract"
  | "plain_text" | "markdown" | "pdf" | "none";

/** One structural unit of a document. The text is the author's, verbatim. */
export interface Block {
  kind: BlockKind;
  text: string;
  level: number | null;
  depth: number | null;
  ordered: boolean | null;
  quote: number;
  lang: string | null;
  skip: boolean;
}

/** A readable document: metadata plus blocks in reading order. */
export interface Document {
  url: string;
  title: string;
  byline: string | null;
  site_name: string | null;
  lang: string | null;
  excerpt: string | null;
  source: Source;
  /** The content is known or suspected to be incomplete. */
  partial: boolean;
  /** The page looked paywalled. */
  paywall_hint: boolean;
  /** Whitespace-delimited word count of all blocks. */
  word_count: number;
  blocks: Block[];
}

/** Playback state. */
export type PlayState = "paused" | "playing" | "finished";

/** What the script builder does with a block kind. */
export type BlockAction = "read" | "mark" | "skip";

/** Per-kind block actions. See the core crate's `ReadPolicy`. */
export interface ReadPolicy {
  heading: BlockAction;
  paragraph: BlockAction;
  list_item: BlockAction;
  blockquote: BlockAction;
  code: BlockAction;
  table: BlockAction;
  figure: BlockAction;
  caption: BlockAction;
  footnote: BlockAction;
  rule: BlockAction;
  honor_skip_hint: boolean;
}

/** Timing parameters. See the core crate's `Pacing` for the formula. */
export interface Pacing {
  wpm: number;
  min_wpm: number;
  max_wpm: number;
  long_word_letters: number;
  per_extra_letter: number;
  max_length_extra: number;
  digits: number;
  clause_pause: number;
  sentence_pause: number;
  paragraph_pause: number;
  heading_word: number;
  heading_pause: number;
  marker_ms: number;
  ease_in_words: number;
  ease_in_start: number;
}

/** Session settings. Every field is optional; unknown fields are rejected. */
export interface Settings {
  /** Words per minute; overrides `pacing.wpm`. */
  wpm?: number;
  pacing?: Partial<Pacing>;
  script?: { policy?: Partial<ReadPolicy>; max_width?: number };
  /** Resuming from pause rewinds to the start of the sentence. */
  rewind_on_resume?: boolean;
}

/** Everything needed to draw the current word. */
export interface Frame {
  /** Text before the pivot. */
  before: string;
  /** The pivot grapheme cluster, drawn at the fixation point. */
  pivot: string;
  /** Text after the pivot. */
  after: string;
  index: number;
  total: number;
  block: number;
  block_kind: BlockKind;
  /** The token is an engine marker such as `[code]`, not author text. */
  is_marker: boolean;
  progress: number;
  remaining_ms: number;
  state: PlayState;
  wpm: number;
}

/** The current token's block, split around the token. */
export interface Context {
  block: number;
  kind: BlockKind;
  before: string;
  /** The current token; for a marker, the whole skipped block. */
  token: string;
  after: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    /// A `Document` object coming from JavaScript.
    #[wasm_bindgen(typescript_type = "Document")]
    pub type DocumentInput;

    /// A `Settings` object coming from JavaScript.
    #[wasm_bindgen(typescript_type = "Settings")]
    pub type SettingsInput;
}

static CRASHED: AtomicBool = AtomicBool::new(false);
static PANIC_HOOK: Once = Once::new();

const CRASH_PREFIX: &str = "meatread engine crashed:";

/// Installs the crash hook (once) and refuses to run on a crashed instance.
/// Every entry point that creates new state calls this first.
fn ensure_usable() -> Result<(), JsError> {
    PANIC_HOOK.call_once(|| {
        std::panic::set_hook(Box::new(|info| {
            CRASHED.store(true, Ordering::SeqCst);
            wasm_bindgen::throw_str(&format!("{CRASH_PREFIX} {info}"));
        }));
    });
    if CRASHED.load(Ordering::SeqCst) {
        Err(JsError::new(&format!(
            "{CRASH_PREFIX} an earlier call failed; load a fresh instance"
        )))
    } else {
        Ok(())
    }
}

fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsError> {
    let json = serde_json::to_string(value)?;
    js_sys::JSON::parse(&json).map_err(|_| JsError::new("could not convert a value to JavaScript"))
}

fn from_js<T: DeserializeOwned>(value: &JsValue, what: &str) -> Result<T, JsError> {
    let json = js_sys::JSON::stringify(value)
        .ok()
        .and_then(|s| s.as_string())
        .ok_or_else(|| JsError::new(&format!("{what} is not a plain object")))?;
    serde_json::from_str(&json).map_err(|e| JsError::new(&format!("invalid {what}: {e}")))
}

/// The version of the meatread engine.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Extracts a readable document from fetched bytes. `contentType` is the raw
/// Content-Type header (it may carry a charset); `url` is the document's
/// address, used to record it and to resolve links. Throws when the input
/// holds no readable text or its type is unsupported.
#[wasm_bindgen(unchecked_return_type = "Document")]
pub fn extract(
    bytes: &[u8],
    #[wasm_bindgen(js_name = contentType)] content_type: Option<String>,
    url: Option<String>,
) -> Result<JsValue, JsError> {
    ensure_usable()?;
    let doc =
        meatread_extract::extract(bytes, content_type.as_deref(), url.as_deref().unwrap_or(""))
            .map_err(|e| JsError::new(&e.to_string()))?;
    to_js(&doc)
}

/// A reading session: a document turned into a timed script and a player
/// positioned in it. Starts paused at the first word (finished if the
/// document has nothing to read).
#[wasm_bindgen]
pub struct Session {
    player: Player,
    words: usize,
}

#[wasm_bindgen]
impl Session {
    /// Builds a session for `doc` (as returned by `extract`). Throws if the
    /// document or the settings are malformed.
    #[wasm_bindgen(constructor)]
    pub fn new(doc: DocumentInput, settings: Option<SettingsInput>) -> Result<Session, JsError> {
        ensure_usable()?;
        let doc: Document = from_js(&doc, "document")?;
        let settings: Settings = match settings {
            Some(s) => from_js(&s, "settings")?,
            None => Settings::default(),
        };
        let player = build_player(&doc, &settings);
        let words = word_count(player.script());
        Ok(Session { player, words })
    }

    /// Starts or resumes playback (with ease-in). From finished, restarts.
    pub fn play(&mut self) {
        self.player.play();
    }

    /// Pauses on the current word.
    pub fn pause(&mut self) {
        self.player.pause();
    }

    /// Pauses if playing, otherwise plays.
    pub fn toggle(&mut self) {
        self.player.toggle();
    }

    /// Returns to the first word, keeping play or pause.
    pub fn restart(&mut self) {
        self.player.restart();
    }

    /// Moves `delta` tokens forward (negative: back).
    pub fn step(&mut self, delta: i32) {
        self.player.step(delta as isize);
    }

    /// Jumps to the start of the next sentence.
    #[wasm_bindgen(js_name = nextSentence)]
    pub fn next_sentence(&mut self) {
        self.player.next_sentence();
    }

    /// Jumps to the start of this sentence, or the previous one if already
    /// at its start.
    #[wasm_bindgen(js_name = prevSentence)]
    pub fn prev_sentence(&mut self) {
        self.player.prev_sentence();
    }

    /// Jumps to the first word of the next block.
    #[wasm_bindgen(js_name = nextParagraph)]
    pub fn next_paragraph(&mut self) {
        self.player.next_paragraph();
    }

    /// Jumps to the first word of this block, or the previous one if
    /// already at its start.
    #[wasm_bindgen(js_name = prevParagraph)]
    pub fn prev_paragraph(&mut self) {
        self.player.prev_paragraph();
    }

    /// Jumps to token `index`, clamped.
    pub fn seek(&mut self, index: usize) {
        self.player.seek(index);
    }

    /// Jumps to the token `fraction` (0 to 1) of the way through.
    #[wasm_bindgen(js_name = seekFraction)]
    pub fn seek_fraction(&mut self, fraction: f32) {
        self.player.seek_fraction(fraction);
    }

    /// Sets the words per minute (clamped), keeping the position.
    #[wasm_bindgen(js_name = setWpm)]
    pub fn set_wpm(&mut self, wpm: u32) {
        self.player.set_wpm(wpm);
    }

    /// Changes the words per minute by `delta` (clamped).
    #[wasm_bindgen(js_name = nudgeWpm)]
    pub fn nudge_wpm(&mut self, delta: i32) {
        self.player.nudge_wpm(delta);
    }

    /// Reports `ms` of elapsed time (whole milliseconds). Returns whether
    /// the current word or the state changed.
    pub fn advance(&mut self, ms: u32) -> bool {
        self.player.advance(ms)
    }

    /// Milliseconds until the current word changes, or `undefined` unless
    /// playing.
    #[wasm_bindgen(js_name = nextChangeIn)]
    pub fn next_change_in(&self) -> Option<u32> {
        self.player.next_change_in()
    }

    /// The current word and playback status.
    #[wasm_bindgen(unchecked_return_type = "Frame")]
    pub fn frame(&self) -> Result<JsValue, JsError> {
        to_js(&self.player.frame())
    }

    /// The current word's block, for a pause view; `undefined` if the
    /// document has nothing to read.
    #[wasm_bindgen(unchecked_return_type = "Context | undefined")]
    pub fn context(&self) -> Result<JsValue, JsError> {
        match self.player.context() {
            Some(ctx) => to_js(&ContextView::from(ctx)),
            None => Ok(JsValue::UNDEFINED),
        }
    }

    /// Playback state.
    #[wasm_bindgen(getter, unchecked_return_type = "PlayState")]
    pub fn state(&self) -> JsValue {
        let name = match self.player.state() {
            meatread_core::PlayState::Paused => "paused",
            meatread_core::PlayState::Playing => "playing",
            meatread_core::PlayState::Finished => "finished",
        };
        JsValue::from_str(name)
    }

    /// Current words per minute.
    #[wasm_bindgen(getter)]
    pub fn wpm(&self) -> u32 {
        self.player.wpm()
    }

    /// Index of the current token.
    #[wasm_bindgen(getter)]
    pub fn index(&self) -> usize {
        self.player.index()
    }

    /// Number of tokens (words, long-word pieces and markers).
    #[wasm_bindgen(getter, js_name = tokenCount)]
    pub fn token_count(&self) -> usize {
        self.player.script().len()
    }

    /// Number of author words that will be read (markers and the later
    /// pieces of split long words are not counted).
    #[wasm_bindgen(getter, js_name = wordCount)]
    pub fn word_count(&self) -> usize {
        self.words
    }

    /// Estimated reading time of the whole script at the current WPM, ms,
    /// excluding ease-in.
    #[wasm_bindgen(getter, js_name = totalMs)]
    pub fn total_ms(&self) -> f64 {
        self.player.total_ms() as f64
    }

    /// Estimated time left at the current WPM, ms, excluding ease-in.
    #[wasm_bindgen(getter, js_name = remainingMs)]
    pub fn remaining_ms(&self) -> f64 {
        self.player.remaining_ms() as f64
    }
}
