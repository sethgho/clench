//! How long each token stays on screen.

use crate::document::BlockKind;
use crate::script::{Pause, Token, TokenKind};

/// Timing parameters. All arithmetic is integer, so a given `Pacing` and
/// script produce the same schedule on every platform.
///
/// Fields named in *permille* are thousandths of the base duration
/// `60_000 / wpm` ms. A word's duration is
///
/// ```text
/// base × (1000 + length + digits + heading + pause) / 1000
/// ```
///
/// where each term is `0` unless it applies:
///
/// * `length`: `(letters - long_word_letters) × per_extra_letter`, capped at
///   `max_length_extra`, for words with more than `long_word_letters` letters;
/// * `digits`: `digits` if the token contains a numeric character;
/// * `heading`: `heading_word` for tokens of a heading block;
/// * `pause`: one of `clause_pause`, `sentence_pause`, `paragraph_pause`, or
///   `heading_pause` for the last token of a heading, by the token's
///   [`Pause`] class.
///
/// The result is truncated to whole milliseconds. Markers are shown for
/// `marker_ms` regardless of WPM. During ease-in (see [`Pacing::eased_ms`])
/// word durations are stretched further: over `ease_in_words` words after
/// starting or resuming, and over the shorter `jump_ease_words` after a jump
/// while playing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct Pacing {
    /// Target words per minute. Default 300.
    pub wpm: u32,
    /// Lowest WPM accepted by [`Pacing::clamp_wpm`]. Default 100.
    pub min_wpm: u32,
    /// Highest WPM accepted by [`Pacing::clamp_wpm`]. Default 1500.
    pub max_wpm: u32,
    /// Words with more letters than this get extra time. Default 6.
    pub long_word_letters: u32,
    /// Permille added per letter beyond `long_word_letters`. Default 40.
    pub per_extra_letter: u32,
    /// Cap on the length term, permille. Default 800.
    pub max_length_extra: u32,
    /// Permille added to tokens containing digits. Default 300.
    pub digits: u32,
    /// Permille added after clause punctuation. Default 500.
    pub clause_pause: u32,
    /// Permille added at a sentence end. Default 1200.
    pub sentence_pause: u32,
    /// Permille added at the end of a block. Default 2000.
    pub paragraph_pause: u32,
    /// Permille added to every token of a heading. Default 400.
    pub heading_word: u32,
    /// Permille added after the last token of a heading, in place of
    /// `paragraph_pause`. Default 2500.
    pub heading_pause: u32,
    /// Fixed display time of a marker token, ms. Default 1500.
    pub marker_ms: u32,
    /// Number of words that ramp up after starting or resuming playback.
    /// `0` disables ease-in altogether, including after jumps. Default 5.
    pub ease_in_words: u32,
    /// Number of words that ramp up after a jump (sentence, paragraph, step,
    /// seek) made while playing, so repeated jumps stay responsive. Never
    /// more than `ease_in_words`; `0` disables easing after jumps. Default 2.
    pub jump_ease_words: u32,
    /// Duration multiplier, permille, for the first eased word; later eased
    /// words step linearly down toward 1000. Default 2000 (half speed).
    pub ease_in_start: u32,
}

impl Default for Pacing {
    fn default() -> Pacing {
        Pacing {
            wpm: 300,
            min_wpm: 100,
            max_wpm: 1500,
            long_word_letters: 6,
            per_extra_letter: 40,
            max_length_extra: 800,
            digits: 300,
            clause_pause: 500,
            sentence_pause: 1200,
            paragraph_pause: 2000,
            heading_word: 400,
            heading_pause: 2500,
            marker_ms: 1500,
            ease_in_words: 5,
            jump_ease_words: 2,
            ease_in_start: 2000,
        }
    }
}

impl Pacing {
    /// `wpm` limited to `min_wpm..=max_wpm` (and never below 1).
    pub fn clamp_wpm(&self, wpm: u32) -> u32 {
        let lo = self.min_wpm.max(1);
        let hi = self.max_wpm.max(lo);
        wpm.clamp(lo, hi)
    }

    /// Base duration of one word at the current (clamped) WPM, ms.
    pub fn base_ms(&self) -> u32 {
        60_000 / self.clamp_wpm(self.wpm)
    }

    /// Duration of `token` at the current WPM, without ease-in, ms.
    pub fn token_ms(&self, token: &Token) -> u32 {
        if token.kind == TokenKind::Marker {
            return self.marker_ms;
        }
        let heading = token.block_kind == BlockKind::Heading;
        let mut permille: u64 = 1000;
        let letters = token.letters as u64;
        let threshold = u64::from(self.long_word_letters);
        if letters > threshold {
            let extra = (letters - threshold).saturating_mul(u64::from(self.per_extra_letter));
            permille += extra.min(u64::from(self.max_length_extra));
        }
        if token.has_digits {
            permille += u64::from(self.digits);
        }
        if heading {
            permille += u64::from(self.heading_word);
        }
        permille += u64::from(match token.pause {
            Pause::None => 0,
            Pause::Clause => self.clause_pause,
            Pause::Sentence => self.sentence_pause,
            Pause::Paragraph if heading => self.heading_pause,
            Pause::Paragraph => self.paragraph_pause,
        });
        let ms = 60 * permille / u64::from(self.clamp_wpm(self.wpm));
        u32::try_from(ms).unwrap_or(u32::MAX)
    }

    /// `ms` stretched for the `step`-th token (0-based) since playback
    /// started or resumed. Step `k < ease_in_words` is multiplied by
    /// `ease_in_start - (ease_in_start - 1000) × k / ease_in_words` permille;
    /// later steps are unchanged.
    pub fn eased_ms(&self, ms: u32, step: u32) -> u32 {
        self.eased_ms_over(ms, step, self.ease_in_words)
    }

    /// Like [`Pacing::eased_ms`] with the ramp spread over `words` tokens
    /// instead of `ease_in_words`.
    pub fn eased_ms_over(&self, ms: u32, step: u32, words: u32) -> u32 {
        if step >= words || self.ease_in_start <= 1000 {
            return ms;
        }
        let span = u64::from(self.ease_in_start - 1000);
        let factor = u64::from(self.ease_in_start) - span * u64::from(step) / u64::from(words);
        u32::try_from(u64::from(ms) * factor / 1000).unwrap_or(u32::MAX)
    }
}
