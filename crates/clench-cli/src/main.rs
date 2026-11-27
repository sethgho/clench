//! `clench`: fetch, extract and inspect documents from the command line.

mod fetch;
mod render;

use std::io::Read;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use clench_core::{Document, Pacing, Player, Script};

#[derive(Parser)]
#[command(
    name = "clench",
    version,
    about = "Extract and inspect clench documents"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Extract a document and print it.
    Extract {
        #[command(flatten)]
        input: Input,
        /// Print the document as JSON.
        #[arg(long, conflicts_with = "text")]
        json: bool,
        /// Print the readable text only, one paragraph per line.
        #[arg(long)]
        text: bool,
    },
    /// Extract a document and print its token schedule.
    Script {
        #[command(flatten)]
        input: Input,
        /// Words per minute.
        #[arg(long, default_value_t = 300)]
        wpm: u32,
    },
}

#[derive(Args)]
struct Input {
    /// An http(s) URL, a file path, or `-` for standard input.
    source: String,
    /// The document's URL when reading a file or standard input.
    #[arg(long)]
    url: Option<String>,
    /// The Content-Type to assume when reading a file or standard input.
    #[arg(long)]
    content_type: Option<String>,
}

fn load(input: &Input) -> Result<Document, String> {
    let (bytes, content_type, url) =
        if input.source.starts_with("http://") || input.source.starts_with("https://") {
            let fetched = fetch::get(&input.source)?;
            (fetched.bytes, fetched.content_type, fetched.final_url)
        } else {
            let mut bytes = Vec::new();
            if input.source == "-" {
                std::io::stdin()
                    .read_to_end(&mut bytes)
                    .map_err(|e| format!("reading standard input: {e}"))?;
            } else {
                bytes = std::fs::read(&input.source)
                    .map_err(|e| format!("reading {}: {e}", input.source))?;
            }
            let content_type = input
                .content_type
                .clone()
                .or_else(|| guess_content_type(&input.source));
            (bytes, content_type, input.url.clone().unwrap_or_default())
        };
    clench_extract::extract(&bytes, content_type.as_deref(), &url).map_err(|e| e.to_string())
}

/// A Content-Type for a local file, from its extension.
fn guess_content_type(path: &str) -> Option<String> {
    let ext = std::path::Path::new(path)
        .extension()?
        .to_str()?
        .to_ascii_lowercase();
    let media = match ext.as_str() {
        "html" | "htm" | "xhtml" => "text/html",
        "md" | "markdown" => "text/markdown",
        "pdf" => "application/pdf",
        "txt" => "text/plain",
        _ => return None,
    };
    Some(media.to_string())
}

fn run(cli: Cli) -> Result<(), String> {
    match cli.command {
        Command::Extract { input, json, text } => {
            let doc = load(&input)?;
            if json {
                let out = serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?;
                println!("{out}");
            } else if text {
                print!("{}", render::reading_text(&doc));
            } else {
                print!("{}", render::listing(&doc));
            }
        }
        Command::Script { input, wpm } => {
            let doc = load(&input)?;
            let script = Script::new(&doc);
            let pacing = Pacing {
                wpm,
                ..Pacing::default()
            };
            print!("{}", render::schedule(&script, &pacing));
            let player = Player::new(script, pacing);
            println!("{}", render::summary(&player));
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("clench: {message}");
            ExitCode::FAILURE
        }
    }
}
