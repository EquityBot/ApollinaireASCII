// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: MIT

//! HTTPS image download for the CLI binary. The library and browser never use the network.

use apollinaire_ascii::{MAX_INPUT_BYTES, Result};
use std::time::Duration;

pub const TIMEOUT: Duration = Duration::from_secs(30);

pub fn is_url(input: &str) -> bool {
    let lower = input.to_ascii_lowercase();
    lower.starts_with("https://") || lower.starts_with("http://")
}

/// Download at most 16 MiB of `image/*` content from an `https://` URL within 30 seconds.
pub fn image(url: &str) -> Result<Vec<u8>> {
    get(url, TIMEOUT, false)
}

/// `loopback_http` permits plain HTTP to 127.0.0.1 so tests can use a local server.
fn get(url: &str, timeout: Duration, loopback_http: bool) -> Result<Vec<u8>> {
    let plain = loopback_http && url.starts_with("http://127.0.0.1:");
    if !url.to_ascii_lowercase().starts_with("https://") && !plain {
        return Err(format!("Only https:// URLs are accepted: {url}"));
    }
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .https_only(!plain)
        .timeout_global(Some(timeout))
        .max_redirects(5)
        .user_agent(concat!("apollinaire/", env!("CARGO_PKG_VERSION")))
        .build()
        .into();
    let mut response = agent
        .get(url)
        .call()
        .map_err(|e| format!("Cannot fetch {url}: {e}"))?;
    let header = |name: &str| {
        response
            .headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(str::to_ascii_lowercase)
    };
    let kind = header("content-type").unwrap_or_default();
    if !kind.starts_with("image/") {
        let kind = if kind.is_empty() {
            "no content type"
        } else {
            &kind
        };
        return Err(format!("{url} returned {kind}, not an image"));
    }
    let too_large = || format!("{url} exceeds the 16 MiB input limit");
    if header("content-length")
        .and_then(|v| v.parse::<u64>().ok())
        .is_some_and(|n| n > MAX_INPUT_BYTES as u64)
    {
        return Err(too_large());
    }
    response
        .body_mut()
        .with_config()
        .limit(MAX_INPUT_BYTES as u64)
        .read_to_vec()
        .map_err(|e| match e {
            ureq::Error::BodyExceedsLimit(_) => too_large(),
            other => format!("Cannot fetch {url}: {other}"),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    /// Serve one canned response on a loopback port and return its URL.
    fn serve(head: &'static str, body: Vec<u8>, delay: Duration) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/picture.png", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0u8; 4096];
            let _ = stream.read(&mut request);
            std::thread::sleep(delay);
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.write_all(&body);
        });
        url
    }
    fn png() -> Vec<u8> {
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_luma8(4, 4)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        bytes.into_inner()
    }

    #[test]
    fn only_https_urls_are_accepted_in_production() {
        for url in [
            "http://127.0.0.1:1/a.png",
            "ftp://example.com/a.png",
            "file:///a.png",
        ] {
            assert!(image(url).unwrap_err().contains("Only https://"), "{url}");
        }
    }
    #[test]
    fn image_content_is_downloaded() {
        let body = png();
        let head = "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nConnection: close\r\n\r\n";
        let url = serve(head, body.clone(), Duration::ZERO);
        assert_eq!(get(&url, TIMEOUT, true).unwrap(), body);
    }
    #[test]
    fn non_image_content_is_refused() {
        let head = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n";
        let url = serve(head, b"<html></html>".to_vec(), Duration::ZERO);
        let error = get(&url, TIMEOUT, true).unwrap_err();
        assert!(
            error.contains("returned text/html, not an image"),
            "{error}"
        );
    }
    #[test]
    fn oversized_bodies_are_refused_with_or_without_a_length() {
        let head = "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: 99999999\r\nConnection: close\r\n\r\n";
        let url = serve(head, vec![], Duration::ZERO);
        assert!(get(&url, TIMEOUT, true).unwrap_err().contains("16 MiB"));
        let head = "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nConnection: close\r\n\r\n";
        let url = serve(head, vec![0; MAX_INPUT_BYTES + 1], Duration::ZERO);
        assert!(get(&url, TIMEOUT, true).unwrap_err().contains("16 MiB"));
    }
    #[test]
    fn slow_servers_time_out() {
        let head = "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nConnection: close\r\n\r\n";
        let url = serve(head, png(), Duration::from_secs(5));
        let started = std::time::Instant::now();
        assert!(get(&url, Duration::from_millis(300), true).is_err());
        assert!(started.elapsed() < Duration::from_secs(4));
    }
}
