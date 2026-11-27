#![cfg(feature = "serde")]

use clench_core::{Block, BlockKind, Document, Pacing, Player, Script, ScriptOptions, Source};
use serde_json::json;

#[test]
fn document_deserializes_with_optional_fields_absent() {
    let value = json!({
        "url": "https://example.com/post",
        "title": "A post",
        "source": "json_ld",
        "word_count": 3,
        "blocks": [
            { "kind": "heading", "text": "Hi", "level": 2 },
            { "kind": "list_item", "text": "one", "depth": 1, "ordered": true },
            { "kind": "blockquote", "text": "quoted", "quote": 2 },
            { "kind": "code", "text": "x", "lang": "rust", "skip": true },
            { "kind": "paragraph", "text": "plain" }
        ]
    });
    let doc: Document = serde_json::from_value(value).unwrap();
    assert_eq!(doc.source, Source::JsonLd);
    assert_eq!(doc.byline, None);
    assert!(!doc.partial && !doc.paywall_hint);
    assert_eq!(doc.blocks[0].level, Some(2));
    assert_eq!(doc.blocks[1].kind, BlockKind::ListItem);
    assert_eq!(doc.blocks[1].ordered, Some(true));
    assert_eq!(doc.blocks[2].quote, 2);
    assert_eq!(doc.blocks[3].lang.as_deref(), Some("rust"));
    assert!(doc.blocks[3].skip);
    assert_eq!(doc.blocks[4], Block::paragraph("plain"));
}

#[test]
fn document_serializes_snake_case_and_round_trips() {
    let doc = Document {
        url: "u".into(),
        title: "t".into(),
        byline: Some("b".into()),
        site_name: Some("s".into()),
        lang: Some("en".into()),
        excerpt: None,
        source: Source::MetaDescription,
        partial: true,
        paywall_hint: true,
        word_count: 1,
        blocks: vec![Block::new(BlockKind::ListItem, "x")],
    };
    let value = serde_json::to_value(&doc).unwrap();
    assert_eq!(value["source"], "meta_description");
    assert_eq!(value["paywall_hint"], true);
    assert_eq!(value["site_name"], "s");
    assert_eq!(value["blocks"][0]["kind"], "list_item");
    assert_eq!(value["blocks"][0]["quote"], 0);
    let back: Document = serde_json::from_value(value).unwrap();
    assert_eq!(back, doc);
    let kinds: Vec<String> = BlockKind::ALL
        .iter()
        .map(|k| {
            serde_json::to_value(k)
                .unwrap()
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect();
    assert_eq!(
        kinds,
        [
            "heading",
            "paragraph",
            "list_item",
            "blockquote",
            "code",
            "table",
            "figure",
            "caption",
            "footnote",
            "rule"
        ]
    );
    let sources: Vec<String> = [
        Source::Readability,
        Source::JsonLd,
        Source::MetaDescription,
        Source::CitationAbstract,
        Source::PlainText,
        Source::Markdown,
        Source::Pdf,
        Source::None,
    ]
    .iter()
    .map(|s| {
        serde_json::to_value(s)
            .unwrap()
            .as_str()
            .unwrap()
            .to_string()
    })
    .collect();
    assert_eq!(
        sources,
        [
            "readability",
            "json_ld",
            "meta_description",
            "citation_abstract",
            "plain_text",
            "markdown",
            "pdf",
            "none"
        ]
    );
}

#[test]
fn options_and_pacing_accept_partial_json() {
    let pacing: Pacing = serde_json::from_value(json!({ "wpm": 450 })).unwrap();
    assert_eq!(
        pacing,
        Pacing {
            wpm: 450,
            ..Pacing::default()
        }
    );
    let options: ScriptOptions =
        serde_json::from_value(json!({ "max_width": 10, "policy": { "code": "skip" } })).unwrap();
    assert_eq!(options.max_width, 10);
    assert_eq!(options.policy.code, clench_core::BlockAction::Skip);
    assert_eq!(options.policy.table, clench_core::BlockAction::Mark);
}

#[test]
fn frames_serialize() {
    let p = Player::new(
        Script::new(&Document::from_plain_text("Hello there")),
        Pacing::default(),
    );
    let value = serde_json::to_value(p.frame()).unwrap();
    assert_eq!(value["pivot"], "e");
    assert_eq!(value["state"], "paused");
    assert_eq!(value["block_kind"], "paragraph");
    assert_eq!(value["block_start"], true);
    assert!(serde_json::to_value(p.context().unwrap()).is_ok());
    assert!(serde_json::to_value(p.script()).is_ok());
}
