//! A deterministic playback state machine driven by the host's clock.

use alloc::vec::Vec;

use crate::document::BlockKind;
use crate::pacing::Pacing;
use crate::script::{Context, Script};

/// Playback state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum PlayState {
    /// Stopped on the current token. A new player starts here.
    Paused,
    /// Advancing as the host reports elapsed time.
    Playing,
    /// The last token's time ran out, or the script is empty.
    Finished,
}

/// Everything a host needs to draw one frame.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Frame<'a> {
    /// Text before the pivot.
    pub before: &'a str,
    /// The pivot grapheme cluster, drawn at the fixation point.
    pub pivot: &'a str,
    /// Text after the pivot.
    pub after: &'a str,
    /// Index of the current token.
    pub index: usize,
    /// Number of tokens in the script.
    pub total: usize,
    /// Index of the current token's block in [`Document::blocks`](crate::Document::blocks).
    pub block: usize,
    /// Kind of the current token's block.
    pub block_kind: BlockKind,
    /// The current token is an engine marker, not author text.
    pub is_marker: bool,
    /// `index / total`, or `1.0` when finished or empty.
    pub progress: f32,
    /// Estimated time left at the current WPM, including the rest of the
    /// current token, ms. Excludes ease-in.
    pub remaining_ms: u64,
    /// Playback state.
    pub state: PlayState,
    /// Current words per minute.
    pub wpm: u32,
}

/// Plays a [`Script`]. The player owns the script and never reads a clock:
/// the host either reports elapsed time with [`Player::advance`] or asks
/// [`Player::next_change_in`] when to wake up next. Both produce the same
/// schedule.
///
/// Any jump (step, seek, sentence or paragraph navigation) starts the target
/// token fresh; while playing it also restarts ease-in. Jumping from
/// [`PlayState::Finished`] leaves the player paused at the target.
#[derive(Debug, Clone)]
pub struct Player {
    script: Script,
    pacing: Pacing,
    prefix_ms: Vec<u64>,
    index: usize,
    state: PlayState,
    elapsed_ms: u32,
    ease_step: u32,
    rewind_on_resume: bool,
}

impl Player {
    /// A paused player at the first token (finished if `script` is empty).
    /// `pacing.wpm` is clamped to its range.
    pub fn new(script: Script, pacing: Pacing) -> Player {
        let state = if script.is_empty() {
            PlayState::Finished
        } else {
            PlayState::Paused
        };
        let mut player = Player {
            script,
            pacing,
            prefix_ms: Vec::new(),
            index: 0,
            state,
            elapsed_ms: 0,
            ease_step: 0,
            rewind_on_resume: false,
        };
        player.retime();
        player
    }

    fn retime(&mut self) {
        self.pacing.wpm = self.pacing.clamp_wpm(self.pacing.wpm);
        self.prefix_ms.clear();
        self.prefix_ms.reserve(self.script.len() + 1);
        let mut total = 0u64;
        self.prefix_ms.push(0);
        for token in self.script.tokens() {
            total += u64::from(self.pacing.token_ms(token));
            self.prefix_ms.push(total);
        }
    }

    /// The script being played.
    pub fn script(&self) -> &Script {
        &self.script
    }

    /// The pacing in effect.
    pub fn pacing(&self) -> &Pacing {
        &self.pacing
    }

    /// Replaces the pacing, keeping the position. `wpm` is clamped.
    pub fn set_pacing(&mut self, pacing: Pacing) {
        self.pacing = pacing;
        self.retime();
    }

    /// Current playback state.
    pub fn state(&self) -> PlayState {
        self.state
    }

    /// Index of the current token (`0` for an empty script).
    pub fn index(&self) -> usize {
        self.index
    }

    /// Current words per minute.
    pub fn wpm(&self) -> u32 {
        self.pacing.wpm
    }

    /// Whether [`Player::play`] from paused first rewinds to the start of the
    /// current sentence.
    pub fn rewind_on_resume(&self) -> bool {
        self.rewind_on_resume
    }

    /// Sets whether resuming rewinds to the start of the current sentence.
    /// Off by default.
    pub fn set_rewind_on_resume(&mut self, rewind: bool) {
        self.rewind_on_resume = rewind;
    }

    /// Starts or resumes playback with ease-in. From finished, restarts at
    /// the first token. Does nothing for an empty script or while playing.
    pub fn play(&mut self) {
        match self.state {
            PlayState::Playing => return,
            _ if self.script.is_empty() => return,
            PlayState::Finished => self.index = 0,
            PlayState::Paused => {
                if self.rewind_on_resume {
                    self.index = self.sentence_start_of(self.index);
                }
            }
        }
        self.state = PlayState::Playing;
        self.elapsed_ms = 0;
        self.ease_step = 0;
    }

    /// Pauses on the current token. When resumed, that token is shown for its
    /// full duration again.
    pub fn pause(&mut self) {
        if self.state == PlayState::Playing {
            self.state = PlayState::Paused;
            self.elapsed_ms = 0;
        }
    }

    /// Pauses if playing, otherwise plays.
    pub fn toggle(&mut self) {
        if self.state == PlayState::Playing {
            self.pause();
        } else {
            self.play();
        }
    }

    /// Returns to the first token. Playing stays playing (with ease-in);
    /// finished becomes paused.
    pub fn restart(&mut self) {
        self.jump(0);
    }

    /// Moves `delta` tokens forward (negative: back), clamped to the script.
    pub fn step(&mut self, delta: isize) {
        let target = if delta < 0 {
            self.index.saturating_sub(delta.unsigned_abs())
        } else {
            self.index.saturating_add(delta.unsigned_abs())
        };
        self.jump(target);
    }

    /// Jumps to the start of the next sentence; does nothing in the last one.
    pub fn next_sentence(&mut self) {
        let Some(token) = self.script.tokens().get(self.index) else {
            return;
        };
        if let Some(start) = self.script.sentence_start(token.sentence + 1) {
            self.jump(start);
        }
    }

    /// Jumps to the start of the current sentence, or to the start of the
    /// previous one if already at the start.
    pub fn prev_sentence(&mut self) {
        let Some(token) = self.script.tokens().get(self.index) else {
            return;
        };
        let start = self.script.sentence_start(token.sentence).unwrap_or(0);
        if self.index > start || token.sentence == 0 {
            self.jump(start);
        } else {
            self.jump(self.script.sentence_start(token.sentence - 1).unwrap_or(0));
        }
    }

    /// Jumps to the first token of the next block; does nothing in the last.
    pub fn next_paragraph(&mut self) {
        let Some(token) = self.script.tokens().get(self.index) else {
            return;
        };
        if let Some(block) = self.script.blocks().get(token.paragraph + 1) {
            self.jump(block.token_start);
        }
    }

    /// Jumps to the first token of the current block, or of the previous
    /// block if already there.
    pub fn prev_paragraph(&mut self) {
        let Some(token) = self.script.tokens().get(self.index) else {
            return;
        };
        let blocks = self.script.blocks();
        let start = blocks[token.paragraph].token_start;
        if self.index > start || token.paragraph == 0 {
            self.jump(start);
        } else {
            self.jump(blocks[token.paragraph - 1].token_start);
        }
    }

    /// Jumps to token `index`, clamped to the last token.
    pub fn seek(&mut self, index: usize) {
        self.jump(index);
    }

    /// Jumps to the token at `fraction` (`0.0..=1.0`, clamped; NaN is `0.0`)
    /// of the way through the script.
    pub fn seek_fraction(&mut self, fraction: f32) {
        let f = if fraction > 0.0 {
            fraction.min(1.0)
        } else {
            0.0
        };
        let target = (f * self.script.len() as f32) as usize;
        self.jump(target);
    }

    fn jump(&mut self, target: usize) {
        if self.script.is_empty() {
            return;
        }
        self.index = target.min(self.script.len() - 1);
        self.elapsed_ms = 0;
        self.ease_step = 0;
        if self.state == PlayState::Finished {
            self.state = PlayState::Paused;
        }
    }

    fn sentence_start_of(&self, index: usize) -> usize {
        self.script
            .tokens()
            .get(index)
            .and_then(|t| self.script.sentence_start(t.sentence))
            .unwrap_or(0)
    }

    /// Sets the WPM (clamped to the pacing's range), keeping the position.
    pub fn set_wpm(&mut self, wpm: u32) {
        self.pacing.wpm = wpm;
        self.retime();
    }

    /// Changes the WPM by `delta`, clamped to the pacing's range.
    pub fn nudge_wpm(&mut self, delta: i32) {
        let wpm = i64::from(self.pacing.wpm) + i64::from(delta);
        self.set_wpm(u32::try_from(wpm.max(0)).unwrap_or(u32::MAX));
    }

    /// Total display time of the current token at the current ease-in step,
    /// ms. `0` for an empty script.
    pub fn current_ms(&self) -> u32 {
        let Some(token) = self.script.tokens().get(self.index) else {
            return 0;
        };
        let ms = (self.prefix_ms[self.index + 1] - self.prefix_ms[self.index]) as u32;
        if token.is_marker() {
            ms
        } else {
            self.pacing.eased_ms(ms, self.ease_step)
        }
    }

    /// Reports that `dt_ms` has elapsed. While playing this moves across as
    /// many tokens as the time covers, carrying the leftover into the next
    /// token so no time is lost, and finishes after the last token. Returns
    /// whether the current token or state changed.
    pub fn advance(&mut self, dt_ms: u32) -> bool {
        if self.state != PlayState::Playing {
            return false;
        }
        let start = self.index;
        let mut budget = u64::from(self.elapsed_ms) + u64::from(dt_ms);
        loop {
            let duration = u64::from(self.current_ms());
            if budget < duration {
                self.elapsed_ms = budget as u32;
                return self.index != start;
            }
            budget -= duration;
            if self.index + 1 >= self.script.len() {
                self.state = PlayState::Finished;
                self.elapsed_ms = 0;
                return true;
            }
            self.index += 1;
            self.ease_step = self.ease_step.saturating_add(1);
        }
    }

    /// Milliseconds until the current token changes if nothing else
    /// happens, or `None` unless playing. Calling
    /// `advance(next_change_in())` moves exactly one step.
    pub fn next_change_in(&self) -> Option<u32> {
        (self.state == PlayState::Playing)
            .then(|| self.current_ms().saturating_sub(self.elapsed_ms))
    }

    /// Estimated reading time of the whole script at the current WPM, ms,
    /// excluding ease-in.
    pub fn total_ms(&self) -> u64 {
        self.prefix_ms.last().copied().unwrap_or(0)
    }

    /// Estimated time left at the current WPM, ms, excluding ease-in. `0`
    /// when finished.
    pub fn remaining_ms(&self) -> u64 {
        if self.state == PlayState::Finished {
            return 0;
        }
        (self.total_ms() - self.prefix_ms[self.index]).saturating_sub(u64::from(self.elapsed_ms))
    }

    /// A snapshot for rendering. Cheap: no allocation or scanning.
    pub fn frame(&self) -> Frame<'_> {
        let parts = self.script.parts(self.index);
        let token = self.script.tokens().get(self.index);
        let total = self.script.len();
        let progress = if self.state == PlayState::Finished || total == 0 {
            1.0
        } else {
            self.index as f32 / total as f32
        };
        Frame {
            before: parts.before,
            pivot: parts.pivot,
            after: parts.after,
            index: self.index,
            total,
            block: token.map_or(0, |t| t.block),
            block_kind: token.map_or(BlockKind::Paragraph, |t| t.block_kind),
            is_marker: token.is_some_and(|t| t.is_marker()),
            progress,
            remaining_ms: self.remaining_ms(),
            state: self.state,
            wpm: self.pacing.wpm,
        }
    }

    /// The current token's block text and byte range, for a pause screen.
    /// `None` for an empty script.
    pub fn context(&self) -> Option<Context<'_>> {
        self.script.context(self.index)
    }
}
