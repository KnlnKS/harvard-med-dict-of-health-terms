use anyhow::{Context, Result, bail, ensure};
use std::{thread, time::Duration};

pub fn get(url: &str, content_type: &str) -> Result<String> {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(25)))
        .build()
        .new_agent();
    for attempt in 0..3 {
        let response = agent.get(url)
            .header("User-Agent", "HarvardHealthDictionary/1.0 (+https://github.com/KnlnKS/harvard-med-dict-of-health-terms)")
            .header("Cache-Control", "no-cache")
            .call();
        let error = match response {
            Ok(mut response) => {
                ensure!(
                    response.status() == 200,
                    "Unexpected HTTP status from {url}"
                );
                let mime = response
                    .headers()
                    .get("Content-Type")
                    .and_then(|value| value.to_str().ok())
                    .unwrap_or_default();
                ensure!(
                    mime.contains(content_type),
                    "Unexpected content type {mime} from {url}"
                );
                match response.body_mut().read_to_string() {
                    Ok(body) => return Ok(body),
                    Err(error) => error,
                }
            }
            Err(error) => error,
        };
        let retryable = matches!(
            error,
            ureq::Error::StatusCode(408 | 429 | 500 | 502 | 503 | 504)
                | ureq::Error::Io(_)
                | ureq::Error::Timeout(_)
                | ureq::Error::ConnectionFailed
                | ureq::Error::HostNotFound
                | ureq::Error::Protocol(_)
                | ureq::Error::Decompress(..)
        );
        if !retryable || attempt == 2 {
            return Err(error).with_context(|| format!("Cannot fetch {url}"));
        }
        thread::sleep(Duration::from_secs(1 << attempt));
    }
    bail!("Cannot fetch {url}")
}
