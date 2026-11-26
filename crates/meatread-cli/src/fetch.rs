//! HTTP fetching. The extraction library does no I/O; this is the host side.

use std::io::Read;
use std::time::Duration;

use ureq::ResponseExt;

/// Sent with every request. Generic on purpose: it names no person, machine
/// or program version.
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 \
(KHTML, like Gecko) Chrome/124.0 Safari/537.36";

const MAX_BODY_BYTES: u64 = 32 * 1024 * 1024;

pub struct Fetched {
    pub bytes: Vec<u8>,
    pub content_type: Option<String>,
    pub final_url: String,
}

pub fn get(url: &str) -> Result<Fetched, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .user_agent(USER_AGENT)
        .max_redirects(10)
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(30)))
        .build()
        .into();
    let response = agent
        .get(url)
        .header(
            "Accept",
            "text/html,application/xhtml+xml,text/markdown,application/pdf;q=0.9,*/*;q=0.8",
        )
        .header("Accept-Language", "en-US,en;q=0.9")
        .call()
        .map_err(|e| format!("request to {url} failed: {e}"))?;
    let status = response.status().as_u16();
    if !(200..300).contains(&status) {
        return Err(describe_status(status, url));
    }
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let final_url = response.get_uri().to_string();
    let mut bytes = Vec::new();
    response
        .into_body()
        .into_reader()
        .take(MAX_BODY_BYTES)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("reading response body from {url}: {e}"))?;
    Ok(Fetched {
        bytes,
        content_type,
        final_url,
    })
}

fn describe_status(status: u16, url: &str) -> String {
    match status {
        401 | 403 => format!(
            "{url} answered HTTP {status}: the site refused this request. This is \
usually a bot block or a login wall; meatread does not try to get around it"
        ),
        404 | 410 => format!("{url} answered HTTP {status}: no such page"),
        429 => format!("{url} answered HTTP 429: too many requests, try again later"),
        500..=599 => format!("{url} answered HTTP {status}: the server had an error"),
        _ => format!("{url} answered HTTP {status}"),
    }
}
