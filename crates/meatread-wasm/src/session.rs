//! The platform-independent part of the bindings: settings, player
//! construction and the plain-data views handed to JavaScript.

use meatread_core::{
    BlockKind, Context, Document, Pacing, Player, Script, ScriptOptions, TokenKind,
};
use serde::{Deserialize, Serialize};

/// Reading settings. Every field is optional; missing fields keep the
/// engine's defaults, and unknown fields are rejected so typos surface.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    /// Words per minute. Overrides `pacing.wpm` when both are given.
    pub wpm: Option<u32>,
    /// Pacing overrides; missing fields take [`Pacing::default`] values.
    pub pacing: Option<Pacing>,
    /// Script options (read policy, long-word width).
    pub script: Option<ScriptOptions>,
    /// Whether resuming from pause rewinds to the start of the sentence.
    pub rewind_on_resume: bool,
}

/// A player for `doc` configured by `settings`.
pub fn build_player(doc: &Document, settings: &Settings) -> Player {
    let options = settings.script.clone().unwrap_or_default();
    let mut pacing = settings.pacing.clone().unwrap_or_default();
    if let Some(wpm) = settings.wpm {
        pacing.wpm = wpm;
    }
    let mut player = Player::new(Script::with_options(doc, &options), pacing);
    player.set_rewind_on_resume(settings.rewind_on_resume);
    player
}

/// Number of author words in `script`: word tokens that do not continue a
/// split long word. Markers are not counted.
pub fn word_count(script: &Script) -> usize {
    script
        .tokens()
        .iter()
        .filter(|t| t.kind == TokenKind::Word && !t.joined)
        .count()
}

/// The current token's block split into the text before the token, the
/// token itself and the text after it. JavaScript strings index by UTF-16
/// code unit, so text pieces are handed over rather than byte offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ContextView<'a> {
    /// Index of the block in the document's `blocks`.
    pub block: usize,
    /// The block's kind.
    pub kind: BlockKind,
    /// Block text before the current token.
    pub before: &'a str,
    /// The current token's text (for a marker, the whole skipped block).
    pub token: &'a str,
    /// Block text after the current token.
    pub after: &'a str,
}

impl<'a> From<Context<'a>> for ContextView<'a> {
    fn from(ctx: Context<'a>) -> ContextView<'a> {
        ContextView {
            block: ctx.block,
            kind: ctx.kind,
            before: &ctx.text[..ctx.start],
            token: &ctx.text[ctx.start..ctx.end],
            after: &ctx.text[ctx.end..],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use meatread_core::{Block, PlayState};

    fn doc() -> Document {
        Document::from_blocks(vec![
            Block::paragraph("Read the post yourself. It is short."),
            Block::new(BlockKind::Code, "fn main() {}"),
            Block::paragraph("Supercalifragilisticexpialidocious words split."),
        ])
    }

    #[test]
    fn settings_parse_partially_and_reject_unknown_fields() {
        let s: Settings =
            serde_json::from_str(r#"{"wpm": 450, "pacing": {"ease_in_words": 0}}"#).unwrap();
        assert_eq!(s.wpm, Some(450));
        let pacing = s.pacing.clone().unwrap();
        assert_eq!(pacing.ease_in_words, 0);
        assert_eq!(pacing.sentence_pause, Pacing::default().sentence_pause);
        assert!(serde_json::from_str::<Settings>(r#"{"wmp": 450}"#).is_err());
        assert_eq!(
            serde_json::from_str::<Settings>("{}").unwrap(),
            Settings::default()
        );
    }

    #[test]
    fn wpm_overrides_pacing_and_policy_applies() {
        let s: Settings = serde_json::from_str(
            r#"{"wpm": 500, "pacing": {"wpm": 200}, "script": {"policy": {"code": "skip"}}}"#,
        )
        .unwrap();
        let player = build_player(&doc(), &s);
        assert_eq!(player.wpm(), 500);
        assert!(player.script().tokens().iter().all(|t| !t.is_marker()));
        assert_eq!(player.state(), PlayState::Paused);
    }

    #[test]
    fn word_count_skips_markers_and_joined_pieces() {
        let player = build_player(&doc(), &Settings::default());
        assert!(player.script().len() > word_count(player.script()));
        assert_eq!(word_count(player.script()), 7 + 3);
    }

    #[test]
    fn omitted_serializes_as_plain_data() {
        let player = build_player(&doc(), &Settings::default());
        let json = serde_json::to_value(player.script().omitted()).unwrap();
        assert_eq!(
            json,
            serde_json::json!([{
                "kind": "code",
                "marked": 1,
                "marked_words": 3,
                "skipped": 0,
                "skipped_words": 0
            }])
        );
    }

    #[test]
    fn context_view_splits_around_the_token() {
        let mut player = build_player(&doc(), &Settings::default());
        player.seek(2);
        let view = ContextView::from(player.context().unwrap());
        assert_eq!(view.before, "Read the ");
        assert_eq!(view.token, "post");
        assert_eq!(view.after, " yourself. It is short.");
    }
}
