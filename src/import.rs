use std::io::Read;
use std::net::TcpListener;
use std::time::Duration;

use crate::deck::Card;

const MAX_IMPORT_BYTES: usize = 64 * 1024;
const MAX_CARDS: usize = 500;
const LISTEN_PORT: u16 = 7878;

pub struct ImportResult {
    pub name: Option<String>,
    pub cards: Vec<Card>,
}

/// Parse TSV data with optional #name: header.
/// Format:
///   #name:My Deck Name
///   front text\tback text
///   front2\tback2
pub fn parse_tsv(data: &str) -> Option<ImportResult> {
    let mut name = None;
    let mut cards = Vec::new();

    for line in data.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(n) = line.strip_prefix("#name:") {
            name = Some(n.trim().to_string());
            continue;
        }
        // Skip comment lines
        if line.starts_with('#') {
            continue;
        }
        if let Some((front, back)) = line.split_once('\t') {
            let front = front.trim();
            let back = back.trim();
            if !front.is_empty() && !back.is_empty() {
                if cards.len() >= MAX_CARDS {
                    break;
                }
                cards.push(Card {
                    front: front.to_string(),
                    back: back.to_string(),
                });
            }
        }
    }

    if cards.is_empty() {
        None
    } else {
        Some(ImportResult { name, cards })
    }
}

/// Listen for a single TCP connection on port 7878, read TSV data.
/// Returns None if cancelled or on error.
pub fn listen_for_import() -> Option<ImportResult> {
    let listener = match TcpListener::bind(format!("0.0.0.0:{}", LISTEN_PORT)) {
        Ok(l) => l,
        Err(e) => {
            log::error!("Failed to bind port {}: {:?}", LISTEN_PORT, e);
            return None;
        }
    };

    // Set a timeout so we can check for cancellation
    listener.set_nonblocking(false).ok();

    // Accept one connection
    match listener.accept() {
        Ok((mut stream, addr)) => {
            log::info!("Import connection from {:?}", addr);
            stream.set_read_timeout(Some(Duration::from_secs(10))).ok();

            let mut buf = vec![0u8; MAX_IMPORT_BYTES];
            let mut total = 0;
            loop {
                match stream.read(&mut buf[total..]) {
                    Ok(0) => break,
                    Ok(n) => {
                        total += n;
                        if total >= MAX_IMPORT_BYTES {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }

            if total == 0 {
                return None;
            }

            let text = String::from_utf8_lossy(&buf[..total]);
            parse_tsv(&text)
        }
        Err(e) => {
            log::error!("Accept failed: {:?}", e);
            None
        }
    }
}

pub fn listen_port() -> u16 {
    LISTEN_PORT
}
