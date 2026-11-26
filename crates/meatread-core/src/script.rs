//! Turning a [`Document`] into an immutable playback [`Script`].

use alloc::string::String;
use alloc::vec::Vec;

use crate::document::{Block, BlockKind, Document};
use crate::text::{self, Ending};

/// What the script builder does with a block of a given kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum BlockAction {
    /// Read the block word by word.
    Read,
    /// Replace the block with a single marker token.
    Mark,
    /// Leave the block out silently.
    Skip,
}

/// Which block kinds are read, marked or skipped.
///
/// The default reads headings, paragraphs, list items and blockquotes; marks
/// code blocks and tables; and silently skips figures, captions, footnotes
/// and rules.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct ReadPolicy {
    /// Action for [`BlockKind::Heading`].
    pub heading: BlockAction,
    /// Action for [`BlockKind::Paragraph`].
    pub paragraph: BlockAction,
    /// Action for [`BlockKind::ListItem`].
    pub list_item: BlockAction,
    /// Action for [`BlockKind::Blockquote`].
    pub blockquote: BlockAction,
    /// Action for [`BlockKind::Code`].
    pub code: BlockAction,
    /// Action for [`BlockKind::Table`].
    pub table: BlockAction,
    /// Action for [`BlockKind::Figure`].
    pub figure: BlockAction,
    /// Action for [`BlockKind::Caption`].
    pub caption: BlockAction,
    /// Action for [`BlockKind::Footnote`].
    pub footnote: BlockAction,
    /// Action for [`BlockKind::Rule`].
    pub rule: BlockAction,
    /// When true, a block with [`Block::skip`](crate::Block::skip) set whose
    /// action would be [`BlockAction::Read`] is skipped silently instead.
    /// Marked and skipped kinds are unaffected.
    pub honor_skip_hint: bool,
}

impl Default for ReadPolicy {
    fn default() -> ReadPolicy {
        use BlockAction::*;
        ReadPolicy {
            heading: Read,
            paragraph: Read,
            list_item: Read,
            blockquote: Read,
            code: Mark,
            table: Mark,
            figure: Skip,
            caption: Skip,
            footnote: Skip,
            rule: Skip,
            honor_skip_hint: true,
        }
    }
}

impl ReadPolicy {
    /// What happens to `block`: its kind's action, except that a block the
    /// extractor flagged [`Block::skip`] is skipped instead of read when
    /// [`ReadPolicy::honor_skip_hint`] is set.
    pub fn resolve(&self, block: &Block) -> BlockAction {
        match self.action(block.kind) {
            BlockAction::Read if block.skip && self.honor_skip_hint => BlockAction::Skip,
            action => action,
        }
    }

    /// Number of words a script built with this policy reads from `blocks`:
    /// the words of every block that resolves to [`BlockAction::Read`].
    /// Marked and skipped blocks contribute none. Counts words as the
    /// tokenizer splits them, so it equals the script's number of word
    /// tokens that do not continue a split long word.
    pub fn word_count(&self, blocks: &[Block]) -> usize {
        blocks
            .iter()
            .filter(|b| self.resolve(b) == BlockAction::Read)
            .map(|b| text::words(&b.text).count())
            .sum()
    }

    /// The configured action for `kind`.
    pub fn action(&self, kind: BlockKind) -> BlockAction {
        *self.slot(kind)
    }

    /// Sets the action for `kind`.
    pub fn set(&mut self, kind: BlockKind, action: BlockAction) {
        *self.slot_mut(kind) = action;
    }

    fn slot(&self, kind: BlockKind) -> &BlockAction {
        match kind {
            BlockKind::Heading => &self.heading,
            BlockKind::Paragraph => &self.paragraph,
            BlockKind::ListItem => &self.list_item,
            BlockKind::Blockquote => &self.blockquote,
            BlockKind::Code => &self.code,
            BlockKind::Table => &self.table,
            BlockKind::Figure => &self.figure,
            BlockKind::Caption => &self.caption,
            BlockKind::Footnote => &self.footnote,
            BlockKind::Rule => &self.rule,
        }
    }

    fn slot_mut(&mut self, kind: BlockKind) -> &mut BlockAction {
        match kind {
            BlockKind::Heading => &mut self.heading,
            BlockKind::Paragraph => &mut self.paragraph,
            BlockKind::ListItem => &mut self.list_item,
            BlockKind::Blockquote => &mut self.blockquote,
            BlockKind::Code => &mut self.code,
            BlockKind::Table => &mut self.table,
            BlockKind::Figure => &mut self.figure,
            BlockKind::Caption => &mut self.caption,
            BlockKind::Footnote => &mut self.footnote,
            BlockKind::Rule => &mut self.rule,
        }
    }
}

/// Options for [`Script::with_options`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct ScriptOptions {
    /// Which blocks are read, marked or skipped.
    pub policy: ReadPolicy,
    /// Words longer than this many grapheme clusters are split into pieces
    /// (see the crate docs for the rule). `0` disables splitting. Default 16.
    pub max_width: usize,
}

impl Default for ScriptOptions {
    fn default() -> ScriptOptions {
        ScriptOptions {
            policy: ReadPolicy::default(),
            max_width: 16,
        }
    }
}

/// Whether a token carries author text or stands in for a skipped block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum TokenKind {
    /// A word (or piece of a long word) from the author's text.
    Word,
    /// An engine-generated marker for a block that was not read. Its text is
    /// [`BlockKind::marker_label`] of the block's kind.
    Marker,
}

/// The pause a token's position implies, strongest applicable class only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Pause {
    /// No pause.
    None,
    /// Clause punctuation: `,` `;` `:` em/en dash, closing paren or bracket,
    /// or a free-standing hyphen run such as `--`.
    Clause,
    /// The token ends a sentence.
    Sentence,
    /// The token ends its block.
    Paragraph,
}

/// One unit of playback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Token {
    /// Author word or engine marker.
    pub kind: TokenKind,
    /// Kind of the source block.
    pub block_kind: BlockKind,
    /// Index of the source block in [`Document::blocks`].
    pub block: usize,
    /// Index of the token's block in [`Script::blocks`].
    pub paragraph: usize,
    /// Ordinal of the token's sentence across the whole script.
    pub sentence: usize,
    /// Start of the token's byte range in the block text. For a marker the
    /// range covers the whole block.
    pub start: usize,
    /// End (exclusive) of the token's byte range in the block text.
    pub end: usize,
    /// Start of the pivot's byte range within [`Script::text`] of this token.
    pub pivot_start: usize,
    /// End (exclusive) of the pivot's byte range within the token text.
    pub pivot_end: usize,
    /// Length of the token's core in grapheme clusters (leading and trailing
    /// punctuation excluded); drives pivot choice and length pacing.
    pub letters: usize,
    /// The token contains a numeric character.
    pub has_digits: bool,
    /// This token continues the previous one with no whitespace between them
    /// (it is a later piece of a split long word).
    pub joined: bool,
    /// The pause implied after this token.
    pub pause: Pause,
}

impl Token {
    /// Whether this token is an engine marker rather than author text.
    pub fn is_marker(&self) -> bool {
        self.kind == TokenKind::Marker
    }
}

/// A block that contributed tokens to a script, with its text retained so
/// hosts can show context.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ScriptBlock {
    /// Index of the block in [`Document::blocks`].
    pub index: usize,
    /// The block's kind.
    pub kind: BlockKind,
    /// The block's text, verbatim.
    pub text: String,
    /// Index of the block's first token.
    pub token_start: usize,
    /// Index one past the block's last token.
    pub token_end: usize,
}

/// The highlighted surroundings of a token: its block's full text and the
/// token's byte range within it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Context<'a> {
    /// Index of the block in [`Document::blocks`].
    pub block: usize,
    /// The block's kind.
    pub kind: BlockKind,
    /// The block's full text.
    pub text: &'a str,
    /// Start of the token's byte range in `text`.
    pub start: usize,
    /// End (exclusive) of the token's byte range in `text`.
    pub end: usize,
}

/// A token's display text split around its pivot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Parts<'a> {
    /// Text before the pivot.
    pub before: &'a str,
    /// The pivot grapheme cluster, to be drawn at the fixation point.
    pub pivot: &'a str,
    /// Text after the pivot.
    pub after: &'a str,
}

/// An immutable sequence of tokens built from a [`Document`].
///
/// With the `serde` feature a script can be serialized for inspection; it is
/// not deserializable, so its internal indices are always consistent.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Script {
    blocks: Vec<ScriptBlock>,
    tokens: Vec<Token>,
    sentence_starts: Vec<usize>,
}

impl Script {
    /// Builds a script from `doc` with [`ScriptOptions::default`].
    pub fn new(doc: &Document) -> Script {
        Script::with_options(doc, &ScriptOptions::default())
    }

    /// Builds a script from `doc` with `options`.
    pub fn with_options(doc: &Document, options: &ScriptOptions) -> Script {
        let mut script = Script::default();
        for (index, block) in doc.blocks.iter().enumerate() {
            match options.policy.resolve(block) {
                BlockAction::Skip => {}
                BlockAction::Mark => script.push_marker(index, block.kind, &block.text),
                BlockAction::Read => {
                    script.push_read(index, block.kind, &block.text, options.max_width)
                }
            }
        }
        script
    }

    fn push_marker(&mut self, index: usize, kind: BlockKind, text: &str) {
        let paragraph = self.blocks.len();
        let label = kind.marker_label();
        let (pivot_start, pivot_end, letters) = text::pivot(label);
        let token_start = self.tokens.len();
        self.sentence_starts.push(token_start);
        self.tokens.push(Token {
            kind: TokenKind::Marker,
            block_kind: kind,
            block: index,
            paragraph,
            sentence: self.sentence_starts.len() - 1,
            start: 0,
            end: text.len(),
            pivot_start,
            pivot_end,
            letters,
            has_digits: false,
            joined: false,
            pause: Pause::Paragraph,
        });
        self.blocks.push(ScriptBlock {
            index,
            kind,
            text: String::from(text),
            token_start,
            token_end: token_start + 1,
        });
    }

    fn push_read(&mut self, index: usize, kind: BlockKind, text: &str, max_width: usize) {
        let words: Vec<(usize, usize)> = text::words(text).collect();
        if words.is_empty() {
            return;
        }
        let paragraph = self.blocks.len();
        let token_start = self.tokens.len();
        let mut sentence_open = false;
        for (w, &(ws, we)) in words.iter().enumerate() {
            let word = &text[ws..we];
            let next = words.get(w + 1).map(|&(s, e)| &text[s..e]);
            let pause = if next.is_none() {
                Pause::Paragraph
            } else if text::ends_sentence(word, next, w == 0) {
                Pause::Sentence
            } else if text::ending(word) == Ending::Clause {
                Pause::Clause
            } else {
                Pause::None
            };
            if !sentence_open {
                self.sentence_starts.push(self.tokens.len());
                sentence_open = true;
            }
            let sentence = self.sentence_starts.len() - 1;
            text::split_long(word, max_width, |ps, pe| {
                let piece = &word[ps..pe];
                let (pivot_start, pivot_end, letters) = text::pivot(piece);
                self.tokens.push(Token {
                    kind: TokenKind::Word,
                    block_kind: kind,
                    block: index,
                    paragraph,
                    sentence,
                    start: ws + ps,
                    end: ws + pe,
                    pivot_start,
                    pivot_end,
                    letters,
                    has_digits: piece.chars().any(char::is_numeric),
                    joined: ps > 0,
                    pause: Pause::None,
                });
            });
            if let Some(last) = self.tokens.last_mut() {
                last.pause = pause;
            }
            if pause >= Pause::Sentence {
                sentence_open = false;
            }
        }
        self.blocks.push(ScriptBlock {
            index,
            kind,
            text: String::from(text),
            token_start,
            token_end: self.tokens.len(),
        });
    }

    /// All tokens in playback order.
    pub fn tokens(&self) -> &[Token] {
        &self.tokens
    }

    /// The blocks that contributed tokens, in order. A token's
    /// [`Token::paragraph`] indexes this slice.
    pub fn blocks(&self) -> &[ScriptBlock] {
        &self.blocks
    }

    /// Number of tokens.
    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    /// Whether the script has no tokens.
    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    /// Number of sentences. A token's [`Token::sentence`] is below this.
    pub fn sentence_count(&self) -> usize {
        self.sentence_starts.len()
    }

    /// Index of the first token of `sentence`, or `None` if out of range.
    pub fn sentence_start(&self, sentence: usize) -> Option<usize> {
        self.sentence_starts.get(sentence).copied()
    }

    /// The text shown for token `index`: a slice of the author's block text
    /// for a word, or the marker label for a marker. Empty if out of range.
    pub fn text(&self, index: usize) -> &str {
        let Some(token) = self.tokens.get(index) else {
            return "";
        };
        match token.kind {
            TokenKind::Marker => token.block_kind.marker_label(),
            TokenKind::Word => &self.blocks[token.paragraph].text[token.start..token.end],
        }
    }

    /// The text of token `index` split around its pivot. All parts are empty
    /// if out of range.
    pub fn parts(&self, index: usize) -> Parts<'_> {
        let Some(token) = self.tokens.get(index) else {
            return Parts::default();
        };
        let text = self.text(index);
        Parts {
            before: &text[..token.pivot_start],
            pivot: &text[token.pivot_start..token.pivot_end],
            after: &text[token.pivot_end..],
        }
    }

    /// The block text around token `index` with the token's byte range, for a
    /// pause or context view. `None` if out of range.
    pub fn context(&self, index: usize) -> Option<Context<'_>> {
        let token = self.tokens.get(index)?;
        let block = &self.blocks[token.paragraph];
        Some(Context {
            block: block.index,
            kind: block.kind,
            text: &block.text,
            start: token.start,
            end: token.end,
        })
    }
}
