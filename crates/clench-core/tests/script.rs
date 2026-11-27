use unicode_segmentation::UnicodeSegmentation;

use clench_core::{
    Block, BlockAction, BlockKind, Document, Pause, ReadPolicy, Script, ScriptOptions, TokenKind,
};

fn texts(script: &Script) -> Vec<&str> {
    (0..script.len()).map(|i| script.text(i)).collect()
}

fn is_sep(c: char) -> bool {
    c.is_whitespace() && !matches!(c, '\u{00A0}' | '\u{2007}' | '\u{202F}')
}

/// Checks that the tokens of every read block cover its text exactly, with
/// only separator whitespace between them, and that pivots are in range.
fn assert_reassembles(doc: &Document, script: &Script) {
    for sb in script.blocks() {
        let block = &doc.blocks[sb.index];
        assert_eq!(sb.text, block.text);
        let tokens = &script.tokens()[sb.token_start..sb.token_end];
        assert!(!tokens.is_empty());
        if tokens[0].kind == TokenKind::Marker {
            assert_eq!(tokens.len(), 1);
            assert_eq!((tokens[0].start, tokens[0].end), (0, block.text.len()));
            continue;
        }
        let text = &block.text;
        let mut cursor = 0;
        let mut rebuilt = String::new();
        for (k, t) in tokens.iter().enumerate() {
            let i = sb.token_start + k;
            assert_eq!(t.block, sb.index);
            assert!(t.start < t.end && t.end <= text.len());
            assert!(text.is_char_boundary(t.start) && text.is_char_boundary(t.end));
            assert!(t.start >= cursor);
            let gap = &text[cursor..t.start];
            assert!(gap.chars().all(is_sep), "non-whitespace gap {gap:?}");
            if k > 0 {
                assert_eq!(t.joined, gap.is_empty(), "joined flag wrong at token {i}");
                if !t.joined {
                    rebuilt.push(' ');
                }
            } else {
                assert!(!t.joined);
            }
            let word = script.text(i);
            assert_eq!(word, &text[t.start..t.end]);
            rebuilt.push_str(word);
            assert!(t.pivot_start < t.pivot_end && t.pivot_end <= word.len());
            let parts = script.parts(i);
            assert_eq!(
                format!("{}{}{}", parts.before, parts.pivot, parts.after),
                word
            );
            cursor = t.end;
        }
        assert!(text[cursor..].chars().all(is_sep));
        let normalized: Vec<&str> = text.split(is_sep).filter(|w| !w.is_empty()).collect();
        assert_eq!(rebuilt, normalized.join(" "));
        assert_eq!(tokens.last().unwrap().pause, Pause::Paragraph);
    }
}

/// A small xorshift generator so the generated cases are reproducible.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

const PIECES: &[&str] = &[
    "a",
    "b",
    "word",
    "Word",
    "The",
    "e.g.",
    "Dr.",
    "U.S.",
    "3.14",
    ".",
    ",",
    ";",
    ":",
    "!",
    "?",
    "...",
    "\u{2026}",
    "\"",
    "\u{201C}",
    "\u{201D}",
    "(",
    ")",
    "[",
    "]",
    "-",
    "\u{2014}",
    "/",
    "_",
    "=",
    "&",
    "#",
    "https://",
    "x.com",
    " ",
    " ",
    " ",
    "  ",
    "\n",
    "\t",
    "\u{00A0}",
    "\u{2003}",
    "e\u{301}",
    "\u{1F44D}\u{1F3FD}",
    "\u{1F468}\u{200D}\u{1F469}",
    "\u{65E5}\u{672C}\u{8A9E}",
    "\u{0645}\u{0631}\u{062D}\u{0628}\u{0627}",
    "supercalifragilistic",
    "0",
    "42",
];

fn random_text(rng: &mut Rng) -> String {
    let n = rng.below(40);
    (0..n).map(|_| PIECES[rng.below(PIECES.len())]).collect()
}

#[test]
fn reassembly_invariant_on_generated_documents() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    for case in 0..3000 {
        let blocks = (0..rng.below(6))
            .map(|_| {
                let kind = BlockKind::ALL[rng.below(BlockKind::ALL.len())];
                Block {
                    skip: rng.below(8) == 0,
                    ..Block::new(kind, random_text(&mut rng))
                }
            })
            .collect();
        let doc = Document::from_blocks(blocks);
        let options = ScriptOptions {
            max_width: rng.below(20),
            ..ScriptOptions::default()
        };
        let script = Script::with_options(&doc, &options);
        assert_reassembles(&doc, &script);
        let mut sentence = 0;
        for (i, t) in script.tokens().iter().enumerate() {
            assert!(
                t.sentence == sentence || t.sentence == sentence + 1,
                "case {case}"
            );
            if t.sentence != sentence {
                assert_eq!(script.sentence_start(t.sentence), Some(i));
            }
            sentence = t.sentence;
            if options.max_width > 0 && !t.is_marker() {
                assert!(
                    script.text(i).graphemes(true).count() <= options.max_width,
                    "case {case}"
                );
            }
        }
    }
}

#[test]
fn reassembly_on_fixed_passages() {
    let doc = Document::from_plain_text(
        "  The quick  brown\tfox — jumped (over) the lazy dog.  \n\n\
         Visit https://example.com/a/very/long/path?with=query&and=more#frag today.\n\n\
         \u{201C}Hello,\u{201D} she said\u{2026} and left.",
    );
    let script = Script::new(&doc);
    assert_reassembles(&doc, &script);
}

#[test]
fn default_policy() {
    let p = ReadPolicy::default();
    for kind in [
        BlockKind::Heading,
        BlockKind::Paragraph,
        BlockKind::ListItem,
        BlockKind::Blockquote,
    ] {
        assert_eq!(p.action(kind), BlockAction::Read);
    }
    for kind in [BlockKind::Code, BlockKind::Table] {
        assert_eq!(p.action(kind), BlockAction::Mark);
    }
    for kind in [
        BlockKind::Figure,
        BlockKind::Caption,
        BlockKind::Footnote,
        BlockKind::Rule,
    ] {
        assert_eq!(p.action(kind), BlockAction::Skip);
    }
}

fn mixed_doc() -> Document {
    Document::from_blocks(vec![
        Block::heading(1, "Title"),
        Block::paragraph("Intro text."),
        Block::new(BlockKind::Code, "fn main() {}"),
        Block::new(BlockKind::Figure, "A cat"),
        Block::new(BlockKind::Table, "a | b"),
        Block::new(BlockKind::Rule, ""),
        Block::new(BlockKind::ListItem, "Item one"),
    ])
}

#[test]
fn code_and_tables_leave_markers_others_are_silent() {
    let doc = mixed_doc();
    let script = Script::new(&doc);
    assert_eq!(
        texts(&script),
        [
            "Title", "Intro", "text.", "[code]", "[table]", "Item", "one"
        ]
    );
    let code = &script.tokens()[3];
    assert_eq!(code.kind, TokenKind::Marker);
    assert!(code.is_marker());
    assert_eq!(code.block, 2);
    assert_eq!(code.block_kind, BlockKind::Code);
    let ctx = script.context(3).unwrap();
    assert_eq!(&ctx.text[ctx.start..ctx.end], "fn main() {}");
    assert!(
        script
            .tokens()
            .iter()
            .filter(|t| !t.is_marker())
            .all(|t| t.kind == TokenKind::Word)
    );
}

#[test]
fn policy_is_configurable() {
    let doc = mixed_doc();
    let mut options = ScriptOptions::default();
    options.policy.set(BlockKind::Code, BlockAction::Read);
    options.policy.set(BlockKind::Table, BlockAction::Skip);
    options.policy.set(BlockKind::Figure, BlockAction::Mark);
    options.policy.set(BlockKind::Heading, BlockAction::Skip);
    let script = Script::with_options(&doc, &options);
    assert_eq!(
        texts(&script),
        [
            "Intro", "text.", "fn", "main()", "{}", "[figure]", "Item", "one"
        ]
    );
}

#[test]
fn skip_hint_suppresses_read_blocks_only() {
    let doc = Document::from_blocks(vec![
        Block {
            skip: true,
            ..Block::paragraph("Subscribe now")
        },
        Block {
            skip: true,
            ..Block::new(BlockKind::Code, "x = 1")
        },
        Block::paragraph("Body."),
    ]);
    assert_eq!(texts(&Script::new(&doc)), ["[code]", "Body."]);
    let mut options = ScriptOptions::default();
    options.policy.honor_skip_hint = false;
    assert_eq!(
        texts(&Script::with_options(&doc, &options)),
        ["Subscribe", "now", "[code]", "Body."]
    );
}

#[test]
fn empty_and_whitespace_blocks_produce_nothing() {
    let doc = Document::from_blocks(vec![Block::paragraph(""), Block::paragraph(" \n\t ")]);
    let script = Script::new(&doc);
    assert!(script.is_empty());
    assert!(script.blocks().is_empty());
    assert_eq!(script.sentence_count(), 0);
    assert_eq!(script.text(0), "");
    assert_eq!(script.parts(5).pivot, "");
    assert!(script.context(0).is_none());
}

#[test]
fn long_tokens_split_and_keep_every_character() {
    let doc = Document::from_plain_text("See https://example.com/some/long/path now.");
    let options = ScriptOptions {
        max_width: 12,
        ..ScriptOptions::default()
    };
    let script = Script::with_options(&doc, &options);
    assert_eq!(
        texts(&script),
        [
            "See",
            "https://",
            "example.com/",
            "some/long/",
            "path",
            "now."
        ]
    );
    let joined: Vec<bool> = script.tokens().iter().map(|t| t.joined).collect();
    assert_eq!(joined, [false, false, true, true, true, false]);
    assert!(script.tokens()[1..5].iter().all(|t| t.sentence == 0));
}

#[test]
fn punctuation_stays_attached() {
    let doc = Document::from_plain_text("\u{201C}Wait\u{2014}what?!\u{201D} (really), yes.");
    assert_eq!(
        texts(&Script::new(&doc)),
        ["\u{201C}Wait\u{2014}what?!\u{201D}", "(really),", "yes."]
    );
}

#[test]
fn no_break_spaces_keep_words_together() {
    let doc = Document::from_plain_text("It is 10\u{00A0}km away.");
    assert_eq!(
        texts(&Script::new(&doc)),
        ["It", "is", "10\u{00A0}km", "away."]
    );
}

fn sentence_starts(text: &str) -> Vec<String> {
    let script = Script::new(&Document::from_plain_text(text));
    (0..script.sentence_count())
        .map(|s| script.text(script.sentence_start(s).unwrap()).to_string())
        .collect()
}

#[test]
fn sentence_boundaries_handle_common_traps() {
    assert_eq!(
        sentence_starts("One two. Three four! Five? Six."),
        ["One", "Three", "Five?", "Six."]
    );
    assert_eq!(sentence_starts("Ask Dr. Smith. He knows."), ["Ask", "He"]);
    assert_eq!(
        sentence_starts("Use tools, e.g. Rust, i.e. fast. Next."),
        ["Use", "Next."]
    );
    assert_eq!(
        sentence_starts("The U.S. Army won. Then."),
        ["The", "Then."]
    );
    assert_eq!(sentence_starts("Pi is 3.14 roughly. Ok."), ["Pi", "Ok."]);
    assert_eq!(
        sentence_starts("Well... maybe. Or not\u{2026} Who knows."),
        ["Well...", "Or", "Who"]
    );
    assert_eq!(
        sentence_starts("He said \u{201C}stop.\u{201D} Then left."),
        ["He", "Then"]
    );
    assert_eq!(sentence_starts("(It ended.) Next one."), ["(It", "Next"]);
    assert_eq!(
        sentence_starts("\"Really?\" she asked. Yes."),
        ["\"Really?\"", "Yes."]
    );
    assert_eq!(sentence_starts("See Fig. 3 below. Done."), ["See", "Done."]);
    assert_eq!(sentence_starts("It cost 5. 30 people paid."), ["It", "30"]);
    assert_eq!(
        sentence_starts("First block\n\nSecond block"),
        ["First", "Second"]
    );
}

#[test]
fn tokens_know_their_paragraph_and_sentence() {
    let doc = Document::from_plain_text("A b. C d.\n\nE f.");
    let script = Script::new(&doc);
    let membership: Vec<(usize, usize)> = script
        .tokens()
        .iter()
        .map(|t| (t.paragraph, t.sentence))
        .collect();
    assert_eq!(membership, [(0, 0), (0, 0), (0, 1), (0, 1), (1, 2), (1, 2)]);
    let pauses: Vec<Pause> = script.tokens().iter().map(|t| t.pause).collect();
    use Pause::*;
    assert_eq!(pauses, [None, Sentence, None, Paragraph, None, Paragraph]);
}

#[test]
fn clause_pauses() {
    let doc = Document::from_plain_text("a, b; c: d\u{2014} e (f) g -- h end");
    let script = Script::new(&doc);
    let pauses: Vec<Pause> = script.tokens().iter().map(|t| t.pause).collect();
    use Pause::*;
    assert_eq!(
        pauses,
        [
            Clause, Clause, Clause, Clause, None, Clause, None, Clause, None, Paragraph
        ]
    );
}

#[test]
fn pivots_through_the_script() {
    let doc = Document::from_plain_text("I read \u{201C}everything\u{201D} caf\u{65}\u{301}s.");
    let script = Script::new(&doc);
    let pivots: Vec<&str> = (0..script.len()).map(|i| script.parts(i).pivot).collect();
    assert_eq!(pivots, ["I", "e", "r", "a"]);
    let p = script.parts(2);
    assert_eq!(
        (p.before, p.pivot, p.after),
        ("\u{201C}eve", "r", "ything\u{201D}")
    );
}

#[test]
fn non_space_delimited_text_does_not_panic() {
    let doc = Document::from_plain_text(
        "\u{8FD9}\u{662F}\u{4E00}\u{4E2A}\u{6CA1}\u{6709}\u{7A7A}\u{683C}\u{7684}\u{53E5}\u{5B50}\u{FF0C}\u{5B83}\u{5F88}\u{957F}\u{5F88}\u{957F}\u{5F88}\u{957F}\u{3002}",
    );
    let script = Script::new(&doc);
    assert!(!script.is_empty());
    assert_reassembles(&doc, &script);
}

#[test]
fn plain_text_documents() {
    let doc = Document::from_plain_text("\n\n  One\ntwo  \n \n\n\tThree\r\n\r\nFour\n");
    let texts: Vec<&str> = doc.blocks.iter().map(|b| b.text.as_str()).collect();
    assert_eq!(texts, ["One\ntwo", "Three", "Four"]);
    assert_eq!(doc.word_count, 4);
    assert_eq!(doc.source, clench_core::Source::PlainText);
    assert!(Document::from_plain_text("").blocks.is_empty());
}

#[test]
fn policy_word_count_matches_the_script_a_reader_plays() {
    let mut hinted = Block::paragraph("hinted away words");
    hinted.skip = true;
    let doc = Document::from_blocks(vec![
        Block::heading(1, "A title"),
        Block::paragraph("Three plain words."),
        hinted,
        Block::new(BlockKind::Footnote, "a footnote with five words"),
        Block::new(BlockKind::Code, "fn main() {}"),
        Block::new(BlockKind::Caption, "a caption"),
    ]);
    let policy = ReadPolicy::default();
    assert_eq!(policy.resolve(&doc.blocks[2]), BlockAction::Skip);
    assert_eq!(policy.resolve(&doc.blocks[3]), BlockAction::Skip);
    assert_eq!(policy.resolve(&doc.blocks[4]), BlockAction::Mark);
    assert_eq!(doc.word_count, 5);
    let played = Script::new(&doc)
        .tokens()
        .iter()
        .filter(|t| t.kind == TokenKind::Word && !t.joined)
        .count();
    assert_eq!(policy.word_count(&doc.blocks), played);

    let all = ReadPolicy {
        footnote: BlockAction::Read,
        honor_skip_hint: false,
        ..ReadPolicy::default()
    };
    assert_eq!(all.word_count(&doc.blocks), 5 + 3 + 5);
}

#[test]
fn the_pronoun_i_and_everyday_words_can_end_a_sentence() {
    assert_eq!(
        sentence_starts("It was late, so did I. Then we left."),
        ["It", "Then"]
    );
    assert_eq!(
        sentence_starts("The answer was no. We left."),
        ["The", "We"]
    );
    assert_eq!(sentence_starts("Wait a sec. Then we go."), ["Wait", "Then"]);
    assert_eq!(
        sentence_starts("It was Vol. IV of the set. Done."),
        ["It", "Done."]
    );
    assert_eq!(
        sentence_starts("See p. 12 and No. 5 of Sec. 3 here. Done."),
        ["See", "Done."]
    );
    assert_eq!(
        sentence_starts("He is the new rep. Smith. Then more."),
        ["He", "Then"]
    );
}

#[test]
fn omitted_totals_marked_and_skipped_blocks_per_kind() {
    let mut hinted = Block::paragraph("one hinted away paragraph");
    hinted.skip = true;
    let doc = Document::from_blocks(vec![
        Block::paragraph("Read this paragraph."),
        Block::new(BlockKind::Code, "fn main() { run(); }"),
        Block::new(BlockKind::Caption, "Photo: someone"),
        Block::new(BlockKind::Code, "let x = 1;"),
        Block::new(BlockKind::Footnote, "a note with five words"),
        hinted,
        Block::new(BlockKind::Rule, ""),
        Block::new(BlockKind::Footnote, "second note"),
    ]);
    let script = Script::new(&doc);
    let got: Vec<_> = script
        .omitted()
        .iter()
        .map(|o| (o.kind, o.marked, o.marked_words, o.skipped, o.skipped_words))
        .collect();
    assert_eq!(
        got,
        [
            (BlockKind::Code, 2, 9, 0, 0),
            (BlockKind::Caption, 0, 0, 1, 2),
            (BlockKind::Footnote, 0, 0, 2, 7),
            (BlockKind::Paragraph, 0, 0, 1, 4),
            (BlockKind::Rule, 0, 0, 1, 0),
        ]
    );

    let everything = ScriptOptions {
        policy: ReadPolicy {
            code: BlockAction::Read,
            caption: BlockAction::Read,
            footnote: BlockAction::Read,
            rule: BlockAction::Read,
            honor_skip_hint: false,
            ..ReadPolicy::default()
        },
        ..ScriptOptions::default()
    };
    assert!(Script::with_options(&doc, &everything).omitted().is_empty());
    assert!(Script::new(&Document::default()).omitted().is_empty());
}
