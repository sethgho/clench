use std::path::PathBuf;
use std::process::{Command, Output};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_meatread"))
        .args(args)
        .output()
        .expect("run meatread")
}

fn stdout(o: &Output) -> String {
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8(o.stdout.clone()).unwrap()
}

#[test]
fn extract_listing_shows_metadata_and_blocks() {
    let path = fixture("01-iankduncan.html");
    let out = stdout(&run(&["extract", path.to_str().unwrap()]));
    assert!(out.contains("title:    Smart People Who Give a Shit Use Tools"));
    assert!(out.contains("partial: false"));
    assert!(out.contains("0  paragraph"));
    assert!(out.contains("LLMs mostly amplify whoever is sitting"));
}

#[test]
fn extract_text_prints_one_block_per_line() {
    let path = fixture("01-iankduncan.html");
    let out = stdout(&run(&["extract", path.to_str().unwrap(), "--text"]));
    assert!(out.starts_with("LLMs mostly amplify whoever is sitting at the keyboard."));
    assert!(!out.contains("Cover:"));
}

#[test]
fn extract_json_round_trips_through_core() {
    let path = fixture("01-iankduncan.html");
    let out = stdout(&run(&["extract", path.to_str().unwrap(), "--json"]));
    let doc: meatread_core::Document = serde_json::from_str(&out).unwrap();
    assert_eq!(doc.title, "Smart People Who Give a Shit Use Tools");
    assert!((1800..=1830).contains(&doc.word_count));
}

#[test]
fn script_prints_schedule_and_summary() {
    let path = fixture("01-iankduncan.html");
    let slow = stdout(&run(&["script", path.to_str().unwrap(), "--wpm", "200"]));
    let fast = stdout(&run(&["script", path.to_str().unwrap(), "--wpm", "600"]));
    let last = |s: &str| s.lines().last().unwrap().to_string();
    assert!(
        last(&slow).contains("at 200 wpm: estimated reading time"),
        "{}",
        last(&slow)
    );
    assert!(last(&fast).contains("at 600 wpm"));
    assert!(slow.lines().nth(1).unwrap().contains("ms"));
}

#[test]
fn pdf_and_stdin_inputs_work() {
    let pdf = fixture("attention.pdf");
    let out = stdout(&run(&["extract", pdf.to_str().unwrap()]));
    assert!(out.contains("source:   Pdf"));

    let mut child = Command::new(env!("CARGO_BIN_EXE_meatread"))
        .args(["extract", "-", "--text", "--content-type", "text/markdown"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::Write;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"# Hi\n\nSome *words* here.\n")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert_eq!(stdout(&out), "Hi\nSome words here.\n");
}

#[test]
fn errors_are_reported_on_stderr_with_failure_status() {
    let out = run(&["extract", "/nonexistent/file.html"]);
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("meatread: reading /nonexistent/file.html")
    );
}
