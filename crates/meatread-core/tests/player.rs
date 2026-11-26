use meatread_core::{Block, BlockKind, Document, Pacing, PlayState, Player, Script};

fn player(text: &str) -> Player {
    Player::new(
        Script::new(&Document::from_plain_text(text)),
        Pacing::default(),
    )
}

fn no_ease() -> Pacing {
    Pacing {
        ease_in_words: 0,
        ..Pacing::default()
    }
}

/// Plays to the end with timer-style driving and returns (index, ms shown).
fn timer_schedule(p: &mut Player) -> Vec<(usize, u32)> {
    let mut out = Vec::new();
    p.play();
    while let Some(wait) = p.next_change_in() {
        out.push((p.index(), wait));
        p.advance(wait);
    }
    out
}

/// Plays to the end reporting fixed `dt` ticks and returns, per token, the
/// tick time at which it first appeared.
fn tick_starts(p: &mut Player, dt: u32) -> Vec<(usize, u64)> {
    let mut out = vec![(p.index(), 0)];
    let mut now = 0u64;
    p.play();
    while p.state() == PlayState::Playing {
        p.advance(dt);
        now += u64::from(dt);
        if p.state() == PlayState::Playing && out.last().unwrap().0 != p.index() {
            out.push((p.index(), now));
        }
    }
    out
}

const PASSAGE: &str = "Speed reading is a skill. It takes practice, patience, and focus.\n\n\
                       Dr. Smith measured 3.14 seconds per sentence \u{2014} roughly.";

#[test]
fn empty_document_never_panics() {
    let mut p = player("");
    assert_eq!(p.state(), PlayState::Finished);
    p.play();
    p.toggle();
    p.step(3);
    p.step(-3);
    p.seek(10);
    p.seek_fraction(0.5);
    p.next_sentence();
    p.prev_sentence();
    p.next_paragraph();
    p.prev_paragraph();
    p.restart();
    p.set_wpm(9999);
    assert!(!p.advance(1000));
    assert_eq!(p.next_change_in(), None);
    assert_eq!(p.state(), PlayState::Finished);
    let f = p.frame();
    assert_eq!((f.before, f.pivot, f.after), ("", "", ""));
    assert_eq!((f.index, f.total, f.remaining_ms), (0, 0, 0));
    assert_eq!(f.progress, 1.0);
    assert!(p.context().is_none());
    assert_eq!(p.total_ms(), 0);
    assert_eq!(p.current_ms(), 0);
}

#[test]
fn single_word_document() {
    let mut p = player("Hello");
    assert_eq!(p.state(), PlayState::Paused);
    let f = p.frame();
    assert_eq!((f.before, f.pivot, f.after), ("H", "e", "llo"));
    p.play();
    let wait = p.next_change_in().unwrap();
    assert!(!p.advance(wait - 1));
    assert!(p.advance(1));
    assert_eq!(p.state(), PlayState::Finished);
    assert_eq!(p.frame().pivot, "e");
    assert_eq!(p.remaining_ms(), 0);
    p.play();
    assert_eq!(p.state(), PlayState::Playing);
    assert_eq!(p.index(), 0);
}

#[test]
fn advance_and_timer_driving_agree() {
    let make = || {
        Player::new(
            Script::new(&Document::from_plain_text(PASSAGE)),
            Pacing::default(),
        )
    };
    let timer = timer_schedule(&mut make());
    let mut starts = Vec::new();
    let mut now = 0u64;
    for &(index, ms) in &timer {
        starts.push((index, now));
        now += u64::from(ms);
    }
    assert_eq!(tick_starts(&mut make(), 1), starts);
    let coarse = tick_starts(&mut make(), 97);
    for (index, at) in coarse.iter().skip(1) {
        let exact = starts.iter().find(|s| s.0 == *index).unwrap().1;
        assert!(
            *at >= exact && *at < exact + 97,
            "token {index} drifted: {at} vs {exact}"
        );
    }
}

#[test]
fn late_advance_crosses_several_tokens_and_carries_remainder() {
    let mut p = Player::new(
        Script::new(&Document::from_plain_text("one two three four")),
        no_ease(),
    );
    p.play();
    let base = p.current_ms();
    assert_eq!(base, 200);
    assert!(p.advance(base * 2 + 50));
    assert_eq!(p.index(), 2);
    assert_eq!(p.next_change_in(), Some(base - 50));
    let total = p.total_ms();
    assert!(p.advance(u32::MAX));
    assert_eq!(p.state(), PlayState::Finished);
    assert_eq!(p.index(), 3);
    assert_eq!(total, 3 * 200 + 600);
}

#[test]
fn deterministic_across_runs() {
    let a = timer_schedule(&mut player(PASSAGE));
    let b = timer_schedule(&mut player(PASSAGE));
    assert_eq!(a, b);
    assert_eq!(
        tick_starts(&mut player(PASSAGE), 13),
        tick_starts(&mut player(PASSAGE), 13)
    );
}

#[test]
fn ease_in_ramps_and_restarts_after_pause_and_jumps() {
    let mut p = player("a b c d e f g h i j k l");
    let base = Pacing::default().base_ms();
    p.play();
    let mut shown = Vec::new();
    for _ in 0..7 {
        let ms = p.next_change_in().unwrap();
        shown.push(ms);
        p.advance(ms);
    }
    assert_eq!(
        shown,
        [
            2 * base,
            1800 * base / 1000,
            1600 * base / 1000,
            1400 * base / 1000,
            1200 * base / 1000,
            base,
            base
        ]
    );
    p.pause();
    p.play();
    assert_eq!(p.next_change_in(), Some(2 * base));
    p.advance(2 * base + 1200 * base / 1000);
    p.step(-1);
    assert_eq!(p.next_change_in(), Some(2 * base));
    p.advance(base);
    p.seek(9);
    assert_eq!(p.next_change_in(), Some(2 * base));
}

#[test]
fn wpm_is_clamped_and_changes_keep_position() {
    let mut p = player(PASSAGE);
    p.seek(5);
    p.set_wpm(10);
    assert_eq!(p.wpm(), 100);
    p.set_wpm(100_000);
    assert_eq!(p.wpm(), 1500);
    p.set_wpm(400);
    p.nudge_wpm(25);
    assert_eq!(p.wpm(), 425);
    p.nudge_wpm(-10_000);
    assert_eq!(p.wpm(), 100);
    assert_eq!(p.index(), 5);
    let slow = p.total_ms();
    p.set_wpm(600);
    assert!(p.total_ms() < slow);
    assert_eq!(p.frame().wpm, 600);
    let p2 = Player::new(
        Script::new(&Document::from_plain_text("x")),
        Pacing {
            wpm: 5,
            ..Pacing::default()
        },
    );
    assert_eq!(p2.wpm(), 100);
}

#[test]
fn remaining_time_tracks_position() {
    let mut p = Player::new(Script::new(&Document::from_plain_text(PASSAGE)), no_ease());
    let total = p.total_ms();
    let durations: u64 = p
        .script()
        .tokens()
        .iter()
        .map(|t| u64::from(p.pacing().token_ms(t)))
        .sum();
    assert_eq!(total, durations);
    assert_eq!(p.remaining_ms(), total);
    p.play();
    let first = p.current_ms();
    p.advance(first / 2);
    assert_eq!(p.remaining_ms(), total - u64::from(first / 2));
    p.advance(first - first / 2);
    assert_eq!(p.remaining_ms(), total - u64::from(first));
    assert_eq!(p.frame().remaining_ms, p.remaining_ms());
}

#[test]
fn sentence_navigation() {
    let mut p = player("A b c. D e f. G h.");
    p.next_sentence();
    assert_eq!(p.index(), 3);
    p.next_sentence();
    assert_eq!(p.index(), 6);
    p.next_sentence();
    assert_eq!(p.index(), 6);
    p.step(1);
    p.prev_sentence();
    assert_eq!(p.index(), 6, "mid-sentence goes to its start");
    p.prev_sentence();
    assert_eq!(p.index(), 3, "at a start goes to the previous sentence");
    p.prev_sentence();
    p.prev_sentence();
    assert_eq!(p.index(), 0);
}

#[test]
fn paragraph_navigation() {
    let doc = Document::from_blocks(vec![
        Block::heading(2, "Head"),
        Block::paragraph("one two"),
        Block::new(BlockKind::Code, "let x;"),
        Block::paragraph("three four"),
    ]);
    let mut p = Player::new(Script::new(&doc), Pacing::default());
    p.next_paragraph();
    assert_eq!(p.index(), 1);
    p.next_paragraph();
    assert_eq!(p.index(), 3);
    assert!(p.frame().is_marker);
    assert_eq!(p.frame().block, 2);
    assert_eq!(p.frame().block_kind, BlockKind::Code);
    p.next_paragraph();
    p.next_paragraph();
    assert_eq!(p.index(), 4);
    p.step(1);
    p.prev_paragraph();
    assert_eq!(p.index(), 4);
    p.prev_paragraph();
    assert_eq!(p.index(), 3);
    p.seek(0);
    p.prev_paragraph();
    assert_eq!(p.index(), 0);
}

#[test]
fn controls_and_states() {
    let mut p = player("One two three. Four five.");
    assert_eq!(p.state(), PlayState::Paused);
    p.toggle();
    assert_eq!(p.state(), PlayState::Playing);
    p.toggle();
    assert_eq!(p.state(), PlayState::Paused);
    p.seek(1_000);
    assert_eq!(p.index(), 4);
    p.step(-100);
    assert_eq!(p.index(), 0);
    p.step(isize::MAX);
    assert_eq!(p.index(), 4);
    p.seek_fraction(0.5);
    assert_eq!(p.index(), 2);
    p.seek_fraction(f32::NAN);
    assert_eq!(p.index(), 0);
    p.seek_fraction(7.0);
    assert_eq!(p.index(), 4);
    p.seek_fraction(-1.0);
    assert_eq!(p.index(), 0);

    p.play();
    p.advance(u32::MAX);
    assert_eq!(p.state(), PlayState::Finished);
    assert_eq!(p.frame().progress, 1.0);
    p.step(-1);
    assert_eq!(p.state(), PlayState::Paused);
    assert_eq!(p.index(), 3);
    p.play();
    p.restart();
    assert_eq!((p.index(), p.state()), (0, PlayState::Playing));
    p.pause();
    p.restart();
    assert_eq!(p.state(), PlayState::Paused);
    assert!(!p.advance(10_000), "paused players ignore time");
    p.play();
    p.advance(u32::MAX);
    p.toggle();
    assert_eq!((p.index(), p.state()), (0, PlayState::Playing));
}

#[test]
fn rewind_on_resume() {
    let mut p = player("Alpha beta gamma. Delta epsilon zeta.");
    p.set_rewind_on_resume(true);
    assert!(p.rewind_on_resume());
    p.seek(5);
    p.play();
    assert_eq!(p.index(), 3);
    p.pause();
    p.set_rewind_on_resume(false);
    p.seek(5);
    p.play();
    assert_eq!(p.index(), 5);
}

#[test]
fn frame_and_context() {
    let mut p = player("First words here.\n\nSecond paragraph, longer.");
    p.seek(4);
    let f = p.frame();
    assert_eq!(format!("{}{}{}", f.before, f.pivot, f.after), "paragraph,");
    assert_eq!(f.pivot, "r");
    assert_eq!((f.index, f.total, f.block), (4, 6, 1));
    assert_eq!(f.block_kind, BlockKind::Paragraph);
    assert!(!f.is_marker);
    assert_eq!(f.state, PlayState::Paused);
    assert_eq!(f.progress, 4.0 / 6.0);
    let ctx = p.context().unwrap();
    assert_eq!(ctx.text, "Second paragraph, longer.");
    assert_eq!(&ctx.text[ctx.start..ctx.end], "paragraph,");
    assert_eq!(ctx.block, 1);
}

#[test]
fn script_survives_random_control_sequences() {
    let doc = Document::from_blocks(vec![
        Block::heading(1, "Heading here"),
        Block::paragraph(PASSAGE),
        Block::new(BlockKind::Table, "a b"),
        Block::paragraph("Tail."),
    ]);
    let mut p = Player::new(Script::new(&doc), Pacing::default());
    let mut x = 0x2545_F491_4F6C_DD1Du64;
    for _ in 0..20_000 {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        match x % 14 {
            0 => p.play(),
            1 => p.pause(),
            2 => p.toggle(),
            3 => p.step((x >> 8) as isize % 7 - 3),
            4 => p.next_sentence(),
            5 => p.prev_sentence(),
            6 => p.next_paragraph(),
            7 => p.prev_paragraph(),
            8 => p.seek((x >> 8) as usize % 80),
            9 => p.seek_fraction((x >> 40) as f32 / (1u64 << 24) as f32),
            10 => p.nudge_wpm((x >> 8) as i32 % 200),
            11 => p.restart(),
            _ => {
                p.advance((x >> 16) as u32 % 2_000);
            }
        }
        let f = p.frame();
        assert!(f.index < f.total);
        assert!(f.remaining_ms <= p.total_ms());
        assert!((100..=1500).contains(&f.wpm));
    }
}
