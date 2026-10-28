//! The bounded line reader and the bearer token the single-instance handshake uses (moved here
//! from the removed automation crate).

use std::io::BufRead;

/// The longest line a launch may send.
const MAX_LINE_BYTES: usize = 1 << 20;
const TOKEN_BYTES: usize = 32;
const TOKEN_HEX_LEN: usize = TOKEN_BYTES * 2;

/// Result of reading one bounded line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LineRead {
    Eof,
    Line,
    TooLong,
}

/// Read one line without ever buffering more than [`MAX_LINE_BYTES`] plus one byte. A line within
/// the limit that is not UTF-8 is an `InvalidData` error.
pub(crate) fn read_bounded_line(reader: &mut impl BufRead, line: &mut String) -> std::io::Result<LineRead> {
    line.clear();
    let mut bytes = Vec::new();
    let n = std::io::Read::take(reader, (MAX_LINE_BYTES + 1) as u64).read_until(b'\n', &mut bytes)?;
    if n == 0 {
        Ok(LineRead::Eof)
    } else if n > MAX_LINE_BYTES {
        line.push_str(&String::from_utf8_lossy(&bytes));
        Ok(LineRead::TooLong)
    } else {
        *line = String::from_utf8(bytes).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(LineRead::Line)
    }
}

/// A 256-bit token from the operating system's random source, as lowercase hex.
pub(crate) fn generate_token() -> Result<String, String> {
    let mut bytes = [0u8; TOKEN_BYTES];
    getrandom::fill(&mut bytes).map_err(|e| format!("cannot generate a token: {e}"))?;
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut token = String::with_capacity(TOKEN_HEX_LEN);
    for byte in bytes {
        token.push(HEX[(byte >> 4) as usize] as char);
        token.push(HEX[(byte & 0x0f) as usize] as char);
    }
    Ok(token)
}

/// Compare fixed-width tokens without an early exit on a mismatching byte; hex letters match in
/// either case.
pub(crate) fn token_matches(expected: &str, supplied: &str) -> bool {
    if expected.len() != TOKEN_HEX_LEN || supplied.len() != TOKEN_HEX_LEN {
        return false;
    }
    expected.bytes().zip(supplied.bytes()).fold(0u8, |different, (a, b)| different | (a.to_ascii_lowercase() ^ b.to_ascii_lowercase())) == 0
}
