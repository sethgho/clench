use std::fs;
use std::path::PathBuf;

use clench_core::session::{Settings, build_player};
use clench_ffi::{ClenchError, Document, PlayState, Session, extract, version};

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn fixture(name: &str) -> (Vec<u8>, Option<String>) {
    let dir = fixtures();
    let bytes = fs::read(dir.join(name)).unwrap();
    let url_file = dir.join(name).with_extension("url");
    let url = fs::read_to_string(url_file)
        .ok()
        .map(|s| s.trim().to_string());
    (bytes, url)
}

fn content_type(name: &str) -> Option<String> {
    match name.rsplit('.').next() {
        Some("html") => Some("text/html".into()),
        Some("pdf") => Some("application/pdf".into()),
        _ => None,
    }
}

fn extract_fixture(name: &str) -> Document {
    let (bytes, url) = fixture(name);
    extract(bytes, content_type(name), url).unwrap()
}

#[test]
fn version_is_the_crate_version() {
    assert_eq!(version(), env!("CARGO_PKG_VERSION"));
}

#[test]
fn extract_matches_the_native_extractor() {
    for name in [
        "01-iankduncan.html",
        "05-static-matklad.html",
        "attention.pdf",
    ] {
        let (bytes, url) = fixture(name);
        let ct = content_type(name);
        let native =
            clench_extract::extract(&bytes, ct.as_deref(), url.as_deref().unwrap_or("")).unwrap();
        let ffi = extract(bytes, ct, url).unwrap();
        assert_eq!(ffi, Document::from(native.clone()), "{name}");
        assert_eq!(
            clench_core::Document::from(ffi),
            native,
            "{name} round trip"
        );
    }
}

#[test]
fn acceptance_page_reads_as_in_the_other_hosts() {
    let doc = extract_fixture("01-iankduncan.html");
    assert_eq!(doc.title, "Smart People Who Give a Shit Use Tools");
    assert_eq!(doc.word_count, 1815);
    let session = Session::new(doc, None).unwrap();
    assert_eq!(session.word_count(), 1815);
    assert_eq!(session.state(), PlayState::Paused);
    let f = session.frame();
    assert_eq!(format!("{}{}{}", f.before, f.pivot, f.after), "LLMs");
}

#[test]
fn errors_are_typed() {
    assert_eq!(
        extract(b"   \n ".to_vec(), Some("text/plain".into()), None),
        Err(ClenchError::NoContent)
    );
    assert!(matches!(
        extract(vec![0x89, b'P', b'N', b'G'], Some("image/png".into()), None),
        Err(ClenchError::UnsupportedType { .. })
    ));
    let deep = format!("{}text", "<div>".repeat(5000));
    assert!(matches!(
        extract(deep.into_bytes(), Some("text/html".into()), None),
        Err(ClenchError::TooDeeplyNested { .. })
    ));
}

#[test]
fn settings_are_the_shared_json_shape() {
    let doc = extract_fixture("05-static-matklad.html");
    let session = Session::new(doc.clone(), Some(r#"{"wpm": 450}"#.into())).unwrap();
    assert_eq!(session.wpm(), 450);
    assert!(matches!(
        Session::new(doc, Some(r#"{"wmp": 450}"#.into())),
        Err(ClenchError::InvalidSettings { .. })
    ));
}

#[test]
fn session_follows_the_core_players_schedule() {
    let doc = extract_fixture("05-static-matklad.html");
    let core_doc = clench_core::Document::from(doc.clone());
    let mut player = build_player(&core_doc, &Settings::default());
    let session = Session::new(doc, None).unwrap();
    assert_eq!(session.token_count(), player.script().len() as u64);
    assert_eq!(session.total_ms(), player.total_ms());

    session.play();
    player.play();
    for _ in 0..300 {
        let wait = session.next_change_in();
        assert_eq!(wait, player.next_change_in());
        let Some(wait) = wait else { break };
        assert_eq!(session.advance(wait), player.advance(wait));
        let (a, b) = (session.frame(), player.frame());
        assert_eq!(
            (a.before.as_str(), a.pivot.as_str(), a.after.as_str()),
            (b.before, b.pivot, b.after)
        );
        assert_eq!(a.index, b.index as u64);
        assert_eq!(a.remaining_ms, b.remaining_ms);
    }
    assert!(session.index() > 0);
}

#[test]
fn controls_and_context() {
    let doc = extract_fixture("05-static-matklad.html");
    let session = Session::new(doc, None).unwrap();
    assert_eq!(session.next_change_in(), None);
    session.seek(2);
    assert_eq!(session.index(), 2);
    let ctx = session.context().unwrap();
    assert!(!ctx.token.is_empty());
    let frame = session.frame();
    assert_eq!(
        ctx.token,
        format!("{}{}{}", frame.before, frame.pivot, frame.after)
    );

    session.next_paragraph();
    assert!(session.frame().block_start);
    session.seek(u64::MAX);
    assert_eq!(session.index(), session.token_count() - 1);
    session.seek_fraction(0.0);
    assert_eq!(session.index(), 0);
    session.set_wpm(10_000);
    assert!(session.wpm() < 10_000);
    session.toggle();
    assert_eq!(session.state(), PlayState::Playing);
    session.pause();
    assert_eq!(session.state(), PlayState::Paused);
    assert!(session.omitted().iter().any(|o| o.marked > 0));
}

#[test]
fn empty_document_is_finished() {
    let doc = Document::from(clench_core::Document::from_blocks(vec![]));
    let session = Session::new(doc, None).unwrap();
    assert_eq!(session.state(), PlayState::Finished);
    assert_eq!(session.context(), None);
    assert_eq!(session.word_count(), 0);
}
