use meatread_core::{Block, BlockKind, Document, Pacing, PlayState, Player, Script};

/// Un-eased duration of each token of `doc` at `pacing`.
fn durations(doc: &Document, pacing: &Pacing) -> Vec<(String, u32)> {
    let script = Script::new(doc);
    (0..script.len())
        .map(|i| {
            (
                script.text(i).to_string(),
                pacing.token_ms(&script.tokens()[i]),
            )
        })
        .collect()
}

fn ms_of(text: &str) -> Vec<u32> {
    durations(&Document::from_plain_text(text), &Pacing::default())
        .into_iter()
        .map(|(_, ms)| ms)
        .collect()
}

#[test]
fn base_duration_is_sixty_thousand_over_wpm() {
    for wpm in [100, 250, 300, 600, 1500] {
        let p = Pacing {
            wpm,
            ..Pacing::default()
        };
        assert_eq!(p.base_ms(), 60_000 / wpm);
    }
}

#[test]
fn each_rule_adds_its_configured_share() {
    // 300 wpm: base 200 ms; one permille is 0.2 ms.
    assert_eq!(ms_of("go on"), [200, 600]);
    assert_eq!(ms_of("go, on"), [300, 600]);
    assert_eq!(ms_of("Go. On"), [440, 600]);
    assert_eq!(ms_of("abcdefghij on"), [232, 600]);
    assert_eq!(ms_of("abcdefghijklmnop on"), [280, 600]);
    assert_eq!(ms_of("in 2024 we"), [200, 260, 600]);
}

#[test]
fn headings_get_extra_time_and_a_pause_after() {
    let doc = Document::from_blocks(vec![
        Block::heading(1, "Big news"),
        Block::paragraph("ok then"),
    ]);
    let ms: Vec<u32> = durations(&doc, &Pacing::default())
        .into_iter()
        .map(|(_, ms)| ms)
        .collect();
    assert_eq!(ms, [280, 780, 200, 600]);
}

#[test]
fn markers_have_a_fixed_duration() {
    let doc = Document::from_blocks(vec![Block::new(BlockKind::Code, "x")]);
    for wpm in [100, 300, 1500] {
        let p = Pacing {
            wpm,
            ..Pacing::default()
        };
        assert_eq!(durations(&doc, &p), [("[code]".to_string(), 1500)]);
    }
}

#[test]
fn every_parameter_is_tunable() {
    let p = Pacing {
        wpm: 600,
        long_word_letters: 3,
        per_extra_letter: 100,
        max_length_extra: 150,
        digits: 0,
        clause_pause: 1000,
        sentence_pause: 0,
        paragraph_pause: 0,
        ..Pacing::default()
    };
    let ms: Vec<u32> = durations(&Document::from_plain_text("word, 12345. done"), &p)
        .into_iter()
        .map(|(_, ms)| ms)
        .collect();
    assert_eq!(ms, [210, 115, 110]);
}

#[test]
fn ease_in_factor() {
    let p = Pacing::default();
    let eased: Vec<u32> = (0..7).map(|step| p.eased_ms(1000, step)).collect();
    assert_eq!(eased, [2000, 1800, 1600, 1400, 1200, 1000, 1000]);
    let off = Pacing {
        ease_in_words: 0,
        ..Pacing::default()
    };
    assert_eq!(off.eased_ms(1000, 0), 1000);
}

#[test]
fn wpm_clamp() {
    let p = Pacing::default();
    assert_eq!(
        (p.clamp_wpm(0), p.clamp_wpm(300), p.clamp_wpm(9000)),
        (100, 300, 1500)
    );
    let odd = Pacing {
        min_wpm: 0,
        max_wpm: 0,
        ..Pacing::default()
    };
    assert_eq!(odd.clamp_wpm(0), 1);
}

/// A fixed passage with its full eased schedule at the default pacing. Any
/// change to tokenization or pacing defaults shows up here.
#[test]
fn golden_schedule() {
    let doc = Document::from_blocks(vec![
        Block::heading(1, "Why read?"),
        Block::paragraph(
            "Don\u{2019}t let a machine summarize it. Dr. Lee read 3.5 posts a day\u{2014}slowly, carefully.",
        ),
        Block::new(BlockKind::Code, "print(1)"),
        Block::paragraph("See https://example.com/articles/read-it-yourself for more."),
    ]);
    let mut player = Player::new(Script::new(&doc), Pacing::default());
    player.play();
    let mut schedule = Vec::new();
    while let Some(ms) = player.next_change_in() {
        schedule.push((player.script().text(player.index()).to_string(), ms));
        player.advance(ms);
    }
    assert_eq!(player.state(), PlayState::Finished);
    let expected: &[(&str, u32)] = &[
        ("Why", 560),
        ("read?", 1404),
        ("Don\u{2019}t", 320),
        ("let", 280),
        ("a", 240),
        ("machine", 208),
        ("summarize", 224),
        ("it.", 440),
        ("Dr.", 200),
        ("Lee", 200),
        ("read", 200),
        ("3.5", 260),
        ("posts", 200),
        ("a", 200),
        ("day\u{2014}slowly,", 332),
        ("carefully.", 624),
        ("[code]", 1500),
        ("See", 200),
        ("https://example.", 272),
        ("com/articles/", 248),
        ("read-it-yourself", 280),
        ("for", 200),
        ("more.", 600),
    ];
    let got: Vec<(&str, u32)> = schedule.iter().map(|(t, ms)| (t.as_str(), *ms)).collect();
    assert_eq!(got, expected);
}
