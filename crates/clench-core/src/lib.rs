//! # clench-core
//!
//! A portable, sans-I/O engine for RSVP (rapid serial visual presentation)
//! speed reading: words are shown one at a time at a fixed point, each
//! aligned on its pivot letter, with timing that slows at punctuation and
//! paragraph breaks.
//!
//! The engine never alters the author's words. It splits, times, skips and
//! annotates; every word token is a byte slice of the original block text.
//!
//! The crate is `#![no_std]` (it needs `alloc`), reads no clock, and uses
//! only integer arithmetic for timing, so native, WebAssembly and embedded
//! builds produce identical schedules.
//!
//! ## Flow
//!
//! 1. Build or deserialize a [`Document`] (typed [`Block`]s).
//! 2. Turn it into a [`Script`] of [`Token`]s with [`Script::new`] or
//!    [`Script::with_options`].
//! 3. Hand the script to a [`Player`] with a [`Pacing`], and drive it from
//!    the host's clock.
//! 4. For a document map, section list or time estimates, build an
//!    [`Outline`] of the script.
//!
//! ```
//! use clench_core::{Document, Pacing, PlayState, Player, Script};
//!
//! let doc = Document::from_plain_text("Read the post yourself.\n\nIt is short.");
//! let script = Script::new(&doc);
//! let mut player = Player::new(script, Pacing { wpm: 400, ..Pacing::default() });
//!
//! player.play();
//! let mut shown = Vec::new();
//! while player.state() == PlayState::Playing {
//!     let frame = player.frame();
//!     shown.push(format!("{}[{}]{}", frame.before, frame.pivot, frame.after));
//!     // A real host would sleep or set a timer; any dt works because the
//!     // player carries leftover time into the next token.
//!     let wait = player.next_change_in().unwrap();
//!     player.advance(wait);
//! }
//! assert_eq!(shown, ["R[e]ad", "t[h]e", "p[o]st", "yo[u]rself.", "I[t]", "i[s]", "s[h]ort."]);
//! assert_eq!(player.state(), PlayState::Finished);
//! ```
//!
//! ## Tokenization rules
//!
//! * Words are runs of non-whitespace. Unicode whitespace separates words,
//!   except the no-break spaces U+00A0, U+2007 and U+202F. Punctuation stays
//!   attached to its word exactly as written.
//! * **Long words.** A word longer than [`ScriptOptions::max_width`] grapheme
//!   clusters (default 16) is split into contiguous pieces. Pieces break just
//!   after one of `/ \ - ‐ – — _ . , ; : ? & = # + | ~ @` and are packed
//!   greedily, so each piece holds as many break-delimited segments as fit.
//!   A segment that alone is too long is cut every `max_width` clusters.
//!   Every character is kept; later pieces have [`Token::joined`] set so a
//!   host can tell they continue the previous token without a space.
//! * **Blocks.** [`ReadPolicy`] decides per [`BlockKind`] whether a block is
//!   read, replaced by one marker token, or skipped silently. By default code
//!   and tables become markers (`[code]`, `[table]`); figures, captions,
//!   footnotes and rules are skipped. Marker tokens have
//!   [`TokenKind::Marker`] and their byte range covers the whole block, so
//!   a host can show what was skipped. [`Script::omitted`] totals, per block
//!   kind, how many blocks were marked or skipped and how many words they
//!   held.
//! * **Pivot.** Over a token's core (first to last alphanumeric grapheme
//!   cluster), the pivot is the 1st cluster for length 1, the 2nd for 2–5,
//!   3rd for 6–9, 4th for 10–13 and 5th for 14+. A token without
//!   alphanumerics uses all its clusters as the core.
//! * **Sentences.** A word ends a sentence when it is the last of its block,
//!   or when it ends in `.` `!` `?` `…` (before any closing quotes or
//!   brackets) and the next word does not start with a lowercase letter.
//!   Exceptions: title abbreviations (`Dr.`, `Mr.`, `vs.`, `Sen.` ...);
//!   reference abbreviations that are also everyday words (`No.`, `Fig.`,
//!   `p.`, `Sec.`, `Vol.` ...) only when a number follows (`No. 5`, `Vol. IV`),
//!   so "The answer was no. We left." ends after "no."; initialisms (`U.S.`,
//!   `e.g.`, `a.m.`, a single capital initial like `J.`, but not the
//!   pronoun `I.`); and a bare list number like `1.` at the start of a block.
//!   Decimals such as `3.14` never end a sentence because the dot is inside
//!   the word.
//!
//! ## Known limitations
//!
//! * Sentence detection is heuristic. It misses a sentence that ends with an
//!   initialism or listed abbreviation (`...in the U.S. The next...`), and
//!   it ends a sentence after other abbreviations followed by a capitalized
//!   word (`Acme Inc. Today`). Lowercase sentence starts are not detected.
//! * Scripts written without spaces between words (Chinese, Japanese, Thai)
//!   are not segmented into words: each run becomes one word, cut every
//!   `max_width` clusters. Widths are counted in grapheme clusters, not
//!   terminal columns, so wide characters take more room than counted.
//! * Right-to-left text is handled as a sequence of bytes like any other;
//!   bidirectional display is the host's job.
#![no_std]
#![warn(missing_docs)]

extern crate alloc;

mod document;
pub mod outline;
mod pacing;
mod player;
mod script;
pub mod session;
mod text;

pub use document::{Block, BlockKind, Document, Source};
pub use outline::{Outline, OutlineBlock, Section, Tick, Timing};
pub use pacing::Pacing;
pub use player::{Frame, PlayState, Player};
pub use script::{
    BlockAction, Context, Omitted, Parts, Pause, ReadPolicy, Script, ScriptBlock, ScriptOptions,
    SentenceView, Token, TokenKind,
};
