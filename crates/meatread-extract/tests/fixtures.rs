//! Regression tests over saved third-party pages. Expectations come from
//! `fixtures/truth.json`; the table below records where the current output
//! deliberately differs.

mod common;

use std::collections::HashSet;

use common::*;
use dom_smoothie::dom_query::Document as Dom;
use meatread_core::{BlockKind, Source};
use serde_json::Value;

/// What a fixture is expected to produce today.
struct Expect {
    name: &'static str,
    source: Source,
    partial: bool,
    paywall_hint: bool,
    /// Readable word count the pipeline currently yields, as a regression pin.
    words: usize,
    /// Why a gap exists, when it does. Empty when output matches the truth file.
    gap: &'static str,
}

const EXPECT: [Expect; 16] = [
    Expect {
        name: "01-iankduncan",
        source: Source::Readability,
        partial: false,
        paywall_hint: false,
        words: 1815,
        gap: "",
    },
    Expect {
        name: "02-news-arstechnica",
        source: Source::Readability,
        partial: false,
        paywall_hint: false,
        words: 3780,
        gap: "",
    },
    Expect {
        name: "03-substack-acx",
        source: Source::Readability,
        partial: false,
        paywall_hint: false,
        words: 2176,
        gap: "",
    },
    Expect {
        name: "04-ghost-404media",
        source: Source::Readability,
        partial: true,
        paywall_hint: true,
        words: 71,
        gap: "paywalled: the HTML holds only the teaser, so the body is flagged partial instead of fixed; a leading tag label (\"AI\") is kept",
    },
    Expect {
        name: "05-static-matklad",
        source: Source::Readability,
        partial: false,
        paywall_hint: false,
        words: 3820,
        gap: "",
    },
    Expect {
        name: "06-wikipedia",
        source: Source::Readability,
        partial: false,
        paywall_hint: false,
        words: 3026,
        gap: "",
    },
    Expect {
        name: "07-rustbook",
        source: Source::Readability,
        partial: false,
        paywall_hint: false,
        words: 3610,
        gap: "",
    },
    Expect {
        name: "08-mdn",
        source: Source::Readability,
        partial: false,
        paywall_hint: false,
        words: 813,
        gap: "",
    },
    Expect {
        name: "09-github-readme",
        source: Source::Readability,
        partial: false,
        paywall_hint: false,
        words: 831,
        gap: "",
    },
    Expect {
        name: "10-arxiv-abs",
        source: Source::MetaDescription,
        partial: true,
        paywall_hint: false,
        words: 166,
        gap: "an abstract page: the paper itself is not in the HTML, so only the citation abstract is returned, flagged partial",
    },
    Expect {
        name: "11-paulgraham",
        source: Source::Readability,
        partial: false,
        paywall_hint: false,
        words: 11849,
        gap: "",
    },
    Expect {
        name: "12-recipe-allrecipes",
        source: Source::Readability,
        partial: false,
        paywall_hint: false,
        words: 924,
        gap: "",
    },
    Expect {
        name: "13-hn-thread",
        source: Source::Readability,
        partial: false,
        paywall_hint: false,
        words: 4369,
        gap: "",
    },
    Expect {
        name: "14-nonenglish-spiegel",
        source: Source::Readability,
        partial: false,
        paywall_hint: true,
        words: 498,
        gap: "structured data says isAccessibleForFree:false yet the full text is served, so paywall_hint is set without partial; an in-text \"Exklusiv im Abo\" label is kept",
    },
    Expect {
        name: "15-spa-bsky",
        source: Source::MetaDescription,
        partial: true,
        paywall_hint: false,
        words: 46,
        gap: "client-rendered page: the post text is not in the HTML, only the meta description is, so the result is flagged partial",
    },
    Expect {
        name: "16-news-bbc",
        source: Source::Readability,
        partial: false,
        paywall_hint: false,
        words: 366,
        gap: "",
    },
];

/// How many characters after the truth file's `end` string may remain.
const END_TAIL_CHARS: usize = 400;
/// How far into the text the truth file's `start` string must appear.
const START_WINDOW_CHARS: usize = 600;

fn truth() -> Value {
    let raw = std::fs::read_to_string(fixtures_dir().join("truth.json")).unwrap();
    serde_json::from_str(&raw).unwrap()
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|s| s.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn truth_file_expectations() {
    let truth = truth();
    let mut failures = Vec::new();
    for e in &EXPECT {
        let doc = load(e.name);
        let t = &truth[e.name];
        let text = reading_text(&doc);
        let lower = text.to_lowercase();
        let everything = all_text(&doc).to_lowercase();

        if doc.source != e.source {
            failures.push(format!(
                "{}: source {:?}, expected {:?}",
                e.name, doc.source, e.source
            ));
        }
        if doc.partial != e.partial {
            failures.push(format!(
                "{}: partial {}, expected {}",
                e.name, doc.partial, e.partial
            ));
        }
        if doc.paywall_hint != e.paywall_hint {
            failures.push(format!("{}: paywall_hint {}", e.name, doc.paywall_hint));
        }
        let (lo, hi) = (e.words * 98 / 100, e.words * 102 / 100 + 1);
        if !(lo..=hi).contains(&doc.word_count) {
            failures.push(format!(
                "{}: {} words, expected about {}",
                e.name, doc.word_count, e.words
            ));
        }

        if let Some(start) = t["start"].as_str().filter(|s| !s.is_empty()) {
            let start = norm(start).to_lowercase();
            match lower.find(&start) {
                Some(at) if at <= START_WINDOW_CHARS => {}
                other => failures.push(format!("{}: start {start:?} at {other:?}", e.name)),
            }
        }
        if let Some(end) = t["end"].as_str().filter(|s| !s.is_empty()) {
            // The end may sit in a block that is skipped by default (a
            // closing license in a code block), so search every block.
            let end = norm(end).to_lowercase();
            match everything.rfind(&end) {
                Some(at) if everything.len() - (at + end.len()) <= END_TAIL_CHARS => {}
                other => failures.push(format!(
                    "{}: end {end:?} at {other:?} of {}",
                    e.name,
                    everything.len()
                )),
            }
        }
        for must in strings(&t["must"]) {
            if !everything.contains(&norm(&must).to_lowercase()) {
                failures.push(format!("{}: missing {must:?}", e.name));
            }
        }
        for junk in strings(&t["junk"]) {
            if text.contains(&norm(&junk)) {
                failures.push(format!("{}: junk present {junk:?}", e.name));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn documented_gaps_are_the_only_ones() {
    for e in &EXPECT {
        let flagged = e.partial || e.paywall_hint;
        assert_eq!(
            !e.gap.is_empty(),
            flagged || e.name == "10-arxiv-abs",
            "{}: gap note and flags disagree",
            e.name
        );
    }
}

#[test]
fn acceptance_smart_people_who_give_a_shit_use_tools() {
    let name = "01-iankduncan";
    assert_eq!(
        fixture_url(name),
        "https://www.iankduncan.com/engineering/2026-11-19-smart-people-who-give-a-shit-use-tools"
    );
    let doc = load(name);
    assert_eq!(doc.title, "Smart People Who Give a Shit Use Tools");
    assert_eq!(doc.source, Source::Readability);
    assert!(!doc.partial, "a complete essay must not be flagged partial");
    assert!(!doc.paywall_hint);
    assert!(
        (1800..=1830).contains(&doc.word_count),
        "word count {}",
        doc.word_count
    );

    let body: Vec<_> = readable(&doc).collect();
    assert_eq!(
        body[0].text.split(". ").next().unwrap(),
        "LLMs mostly amplify whoever is sitting at the keyboard"
    );
    assert!(
        body[0]
            .text
            .starts_with("LLMs mostly amplify whoever is sitting at the keyboard.")
    );
    assert!(
        body.last().unwrap().text.ends_with(
            "instead of pretending that giving a shit requires typing every character by hand."
        ),
        "ends with: {:?}",
        body.last().unwrap().text
    );

    // The cover credit is typed as a caption and flagged as not to be read.
    let credit = doc
        .blocks
        .iter()
        .find(|b| b.text.starts_with("Cover:"))
        .expect("credit kept as a block");
    assert_eq!(credit.kind, BlockKind::Caption);
    assert!(credit.skip);

    let text = reading_text(&doc);
    for junk in [
        "Cover:",
        "Software engineer",
        "Keep reading",
        "Borrowing at 17%",
        "Engineering essays",
        "All essays",
        "min read",
        "Jan Collaert",
    ] {
        assert!(!text.contains(junk), "junk {junk:?} present");
    }

    // What a reader plays contains none of it either.
    let script = meatread_core::Script::new(&doc);
    let played: Vec<&str> = (0..script.len()).map(|i| script.text(i)).collect();
    assert!(!played.contains(&"Collaert"));
    assert!(!played.contains(&"Cover:"));
}

#[test]
fn arstechnica_kicker_and_author_note_are_dropped() {
    let doc = load("02-news-arstechnica");
    let first = &readable(&doc).next().unwrap().text;
    assert!(
        first.starts_with("Frontier AI is already solving"),
        "{first}"
    );
    assert!(!reading_text(&doc).contains("originally appeared on"));
}

#[test]
fn standalone_dateline_is_dropped_but_inline_dates_stay() {
    let matklad = load("05-static-matklad");
    assert!(!reading_text(&matklad).contains("Sep 4, 2021"));
    // A date written by the author as text is content.
    let pg = load("11-paulgraham");
    assert_eq!(readable(&pg).next().unwrap().text, "July 2023");
}

#[test]
fn spiegel_recirculation_module_is_dropped_and_teaser_is_restored() {
    let doc = load("14-nonenglish-spiegel");
    let text = reading_text(&doc);
    assert!(!text.contains("Mehr zum Thema"));
    assert!(!doc.blocks.iter().any(|b| b.text == "5 Min"));
    assert!(
        readable(&doc).next().unwrap().text.starts_with(
            "Die EU wird sich nicht über den nächsten mehrjährigen Finanzrahmen einig."
        )
    );
    assert_eq!(doc.lang.as_deref(), Some("de"));
}

#[test]
fn paywalled_and_client_rendered_pages_are_flagged_not_fixed() {
    let paywalled = load("04-ghost-404media");
    assert!(paywalled.partial && paywalled.paywall_hint);
    assert!(paywalled.word_count < 100);
    assert!(!reading_text(&paywalled).contains("This post is for paid members only"));

    let spa = load("15-spa-bsky");
    assert!(spa.partial);
    assert_eq!(spa.source, Source::MetaDescription);
    assert!(spa.word_count < 80);
}

#[test]
fn arxiv_landing_page_reads_as_its_abstract() {
    let doc = load("10-arxiv-abs");
    assert_eq!(doc.title, "Attention Is All You Need");
    assert!(
        doc.byline
            .as_deref()
            .unwrap()
            .starts_with("Vaswani, Ashish; ")
    );
    assert_eq!(doc.blocks.len(), 1);
    assert!(
        doc.blocks[0]
            .text
            .starts_with("The dominant sequence transduction models")
    );
}

// ---------------------------------------------------------------- structure

fn source_pre_texts(name: &str) -> Vec<String> {
    let html = String::from_utf8(fixture_html_bytes(name)).unwrap();
    let dom = Dom::from(html.as_str());
    dom.select("pre")
        .nodes()
        .iter()
        .map(|n| n.text().trim_end_matches(['\n', '\r']).to_string())
        .collect()
}

#[test]
fn code_blocks_keep_text_byte_for_byte() {
    for (name, expected_blocks) in [("05-static-matklad", 13), ("07-rustbook", 15)] {
        let doc = load(name);
        let extracted: Vec<&str> = of_kind(&doc, BlockKind::Code)
            .iter()
            .map(|b| b.text.as_str())
            .collect();
        let source = source_pre_texts(name);
        assert_eq!(extracted.len(), expected_blocks, "{name}");
        assert_eq!(source.len(), expected_blocks, "{name}: source <pre> count");
        for (got, want) in extracted.iter().zip(&source) {
            assert_eq!(got, want, "{name}: code block changed");
        }
        for b in of_kind(&doc, BlockKind::Code) {
            assert!(b.skip, "code is skipped by default");
        }
    }
}

#[test]
fn code_language_tags() {
    let rust = load("07-rustbook");
    let langs: Vec<_> = of_kind(&rust, BlockKind::Code)
        .iter()
        .map(|b| b.lang.clone())
        .collect();
    assert_eq!(
        langs
            .iter()
            .filter(|l| l.as_deref() == Some("rust"))
            .count(),
        14
    );
    assert!(langs.contains(&Some("console".to_string())));

    let matklad = load("05-static-matklad");
    assert!(
        of_kind(&matklad, BlockKind::Code)
            .iter()
            .all(|b| b.lang.is_none())
    );

    let mdn = load("08-mdn");
    assert!(
        of_kind(&mdn, BlockKind::Code)
            .iter()
            .any(|b| b.lang.as_deref() == Some("js"))
    );
}

#[test]
fn matklad_must_have_commands_are_in_code() {
    let doc = load("05-static-matklad");
    let code: Vec<&str> = of_kind(&doc, BlockKind::Code)
        .iter()
        .map(|b| b.text.as_str())
        .collect();
    assert!(
        code.iter()
            .any(|c| c.contains("RUST_LOG=rust_analyzer=very complex filtering expression"))
    );
    assert!(
        code.iter()
            .any(|c| c.contains("cargo +nightly build -p rust-analyzer"))
    );
}

#[test]
fn wikipedia_lists_quotes_and_footnotes_are_typed() {
    let doc = load("06-wikipedia");
    let footnotes = of_kind(&doc, BlockKind::Footnote);
    assert!(footnotes.len() >= 25, "{} footnotes", footnotes.len());
    assert!(
        doc.blocks
            .iter()
            .all(|b| !b.text.contains('\u{2191}') && !b.text.starts_with('^')),
        "back-link arrows must be dropped"
    );
    assert!(
        of_kind(&doc, BlockKind::Blockquote)
            .iter()
            .all(|b| b.quote == 1)
    );
    assert!(!of_kind(&doc, BlockKind::Blockquote).is_empty());

    let items = of_kind(&doc, BlockKind::ListItem);
    assert!(
        items
            .iter()
            .all(|b| b.depth == Some(1) && b.ordered.is_some())
    );
    assert!(items.iter().any(|b| b.ordered == Some(true)));
    assert!(items.iter().any(|b| b.ordered == Some(false)));
    assert!(
        of_kind(&doc, BlockKind::Heading)
            .iter()
            .all(|b| b.level.is_some_and(|l| (1..=6).contains(&l)))
    );
}

#[test]
fn paulgraham_br_layout_is_split_into_paragraphs() {
    let doc = load("11-paulgraham");
    let paragraphs = of_kind(&doc, BlockKind::Paragraph);
    assert!(paragraphs.len() > 250, "{} paragraphs", paragraphs.len());
    let longest = paragraphs
        .iter()
        .map(|b| b.text.split_whitespace().count())
        .max()
        .unwrap();
    assert!(
        longest < 500,
        "a {longest}-word paragraph suggests unsplit text"
    );
    assert!(
        of_kind(&doc, BlockKind::Table).is_empty(),
        "layout tables are not data"
    );
    assert!(doc.blocks.iter().any(|b| b.text.starts_with("Notes")));
}

#[test]
fn data_table_is_a_table_and_layout_tables_are_not() {
    let mdn = load("08-mdn");
    let tables = of_kind(&mdn, BlockKind::Table);
    assert_eq!(tables.len(), 1);
    assert!(tables[0].skip);
    assert!(
        tables[0].text.contains('\n'),
        "rows are separated by newlines"
    );

    let hn = load("13-hn-thread");
    assert!(of_kind(&hn, BlockKind::Table).is_empty());
    assert!(hn.word_count > 4000);
}

// -------------------------------------------------------------- never alter

fn json_strings(v: &Value, out: &mut String) {
    match v {
        Value::String(s) => {
            out.push(' ');
            out.push_str(s);
        }
        Value::Array(a) => a.iter().for_each(|v| json_strings(v, out)),
        Value::Object(m) => m.values().for_each(|v| json_strings(v, out)),
        _ => {}
    }
}

fn is_sep(c: char) -> bool {
    c.is_whitespace() && !matches!(c, '\u{a0}' | '\u{2007}' | '\u{202f}')
}

/// Elements a browser lays out on their own line, so text on either side is
/// separated even without white space in the markup.
const BREAKING: &[&str] = &[
    "html",
    "head",
    "body",
    "title",
    "p",
    "div",
    "section",
    "article",
    "main",
    "header",
    "footer",
    "aside",
    "nav",
    "ul",
    "ol",
    "li",
    "dl",
    "dt",
    "dd",
    "blockquote",
    "pre",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "table",
    "tr",
    "td",
    "th",
    "thead",
    "tbody",
    "tfoot",
    "caption",
    "figure",
    "figcaption",
    "br",
    "hr",
    "form",
    "fieldset",
    "legend",
    "address",
    "details",
    "summary",
    "center",
    "script",
    "style",
    "noscript",
    "template",
    "button",
    "select",
    "option",
    "textarea",
    "label",
    "menu",
    "hgroup",
];

fn collect_text(node: &dom_smoothie::dom_query::NodeRef, out: &mut String) {
    for child in node.children() {
        if child.is_text() {
            out.push_str(&child.text());
        } else if child.is_element() {
            let breaks = child
                .node_name()
                .is_some_and(|n| BREAKING.contains(&n.to_ascii_lowercase().as_str()));
            if breaks {
                out.push(' ');
            }
            collect_text(&child, out);
            if breaks {
                out.push(' ');
            }
        }
    }
}

/// Every word the source page contains: its text as laid out (scripts
/// included), attribute values that can surface as text, and decoded JSON-LD
/// strings.
fn source_text(html: &str) -> String {
    let dom = Dom::from(html);
    let mut corpus = String::new();
    for root in dom.select("html").nodes() {
        collect_text(root, &mut corpus);
    }
    for node in dom.select("*").nodes() {
        for attr in ["alt", "title", "content"] {
            if let Some(v) = node.attr(attr) {
                corpus.push(' ');
                corpus.push_str(&v);
            }
        }
    }
    for node in dom.select(r#"script[type="application/ld+json"]"#).nodes() {
        if let Ok(v) = serde_json::from_str::<Value>(&node.text()) {
            json_strings(&v, &mut corpus);
        }
    }
    corpus
}

#[test]
fn extraction_never_alters_words() {
    // Every extracted word must be a word of the source. Dropping an element
    // can leave a fragment of a source word (a citation marker removed from
    // `sentence."[1]`, a bracket split off by main-content detection); those
    // are counted separately and must still be a contiguous part of the
    // source text.
    let mut exact = 0usize;
    let mut fragments = Vec::new();
    let mut unexplained = Vec::new();
    for name in FIXTURES {
        let html = String::from_utf8(fixture_html_bytes(name)).unwrap();
        let corpus = source_text(&html);
        let words: HashSet<&str> = corpus.split(is_sep).filter(|w| !w.is_empty()).collect();
        let doc = load(name);
        let metadata = [
            Some(doc.title.as_str()),
            doc.byline.as_deref(),
            doc.site_name.as_deref(),
        ]
        .into_iter()
        .flatten()
        .map(|f| (usize::MAX, f));
        let blocks = doc
            .blocks
            .iter()
            .enumerate()
            .map(|(i, b)| (i, b.text.as_str()));
        for (i, text) in blocks.chain(metadata) {
            for w in text.split(is_sep).filter(|w| !w.is_empty()) {
                // Several authors are joined with "; " in the byline.
                let w = if i == usize::MAX {
                    w.trim_end_matches(';')
                } else {
                    w
                };
                if words.contains(w) {
                    exact += 1;
                } else if corpus.contains(w) {
                    fragments.push(format!("{name} block {i}: {w:?}"));
                } else {
                    unexplained.push(format!("{name} block {i}: {w:?}"));
                }
            }
        }
    }
    eprintln!(
        "never-alter: {exact} exact, {} fragments of source words, {} unexplained",
        fragments.len(),
        unexplained.len()
    );
    // Main-content detection removes a run of `<br>` that separates a
    // paragraph from a following `<font>` element, so the two words meet.
    assert_eq!(
        unexplained,
        ["11-paulgraham block 290: \"it.Thanks\""],
        "the only known exception"
    );
}

#[test]
fn block_text_has_collapsed_whitespace_except_code_and_tables() {
    for name in FIXTURES {
        let doc = load(name);
        for b in &doc.blocks {
            if matches!(b.kind, BlockKind::Code | BlockKind::Table) {
                continue;
            }
            assert!(
                !b.text.contains(['\n', '\t', '\r'])
                    && !b.text.contains("  ")
                    && b.text == b.text.trim_matches(' '),
                "{name}: uncollapsed whitespace in {:?}",
                b.text
            );
        }
    }
}

// ---------------------------------------------------------- end to end

#[test]
fn every_fixture_scripts_and_plays_to_the_end() {
    use meatread_core::{Pacing, PlayState, Player, Script, TokenKind};

    for name in FIXTURES {
        let doc = load(name);
        let script = Script::new(&doc);
        assert!(!script.is_empty(), "{name}");

        // Tokens rebuild each read block's words.
        for sb in script.blocks() {
            let tokens = &script.tokens()[sb.token_start..sb.token_end];
            if tokens[0].kind == TokenKind::Marker {
                continue;
            }
            let mut rebuilt = String::new();
            for (k, t) in tokens.iter().enumerate() {
                if k > 0 && !t.joined {
                    rebuilt.push(' ');
                }
                rebuilt.push_str(script.text(sb.token_start + k));
            }
            let expected: Vec<&str> = doc.blocks[sb.index]
                .text
                .split(is_sep)
                .filter(|w| !w.is_empty())
                .collect();
            assert_eq!(rebuilt, expected.join(" "), "{name} block {}", sb.index);
        }

        let mut player = Player::new(
            script,
            Pacing {
                wpm: 600,
                ..Pacing::default()
            },
        );
        player.play();
        let mut steps = 0usize;
        while player.state() == PlayState::Playing {
            let wait = player
                .next_change_in()
                .expect("playing player has a deadline");
            player.advance(wait);
            steps += 1;
            assert!(
                steps <= player.script().len() + 1,
                "{name}: player did not terminate"
            );
        }
        assert_eq!(player.state(), PlayState::Finished, "{name}");
    }
}
