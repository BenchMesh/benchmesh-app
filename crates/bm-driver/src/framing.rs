//! Incremental reply framing: bytes in, one complete reply out. Shared by the live engine
//! (bytes arrive in chunks) and capture replay (all bytes of an exchange at once).

use bm_devicedef::{Reply, RxFraming, RxMode};

use crate::Error;

/// Frames one reply. Feed bytes with [`Framer::push`]; when the transport reports silence
/// or a timeout, call [`Framer::finish`].
#[derive(Debug)]
pub struct Framer<'a> {
    rx: &'a RxFraming,
    reply: &'a Reply,
    buf: Vec<u8>,
    started: bool,
}

/// A complete reply plus any bytes that arrived after it (belong to nobody; the engine
/// drains them).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Framed {
    pub reply: Vec<u8>,
    pub leftover: Vec<u8>,
}

impl<'a> Framer<'a> {
    pub fn new(rx: &'a RxFraming, reply: &'a Reply) -> Self {
        Framer { rx, reply, buf: Vec::new(), started: false }
    }

    /// Bytes received so far (after leading-noise skipping).
    pub fn buffered(&self) -> &[u8] {
        &self.buf
    }

    /// Feed bytes; returns the reply once complete.
    pub fn push(&mut self, bytes: &[u8]) -> Result<Option<Framed>, Error> {
        for &b in bytes {
            if !self.started && self.rx.skip_leading.as_bytes().contains(&b) {
                continue;
            }
            self.started = true;
            self.buf.push(b);
        }
        if self.buf.is_empty() {
            return Ok(None);
        }
        match self.reply {
            Reply::None => Err(Error::Framing("received data for a command with `reply: none`".into())),
            Reply::Bytes(n) => {
                let n = *n as usize;
                Ok((self.buf.len() >= n).then(|| self.split(n, 0)))
            }
            Reply::Block => self.block(),
            Reply::Line => match self.rx.mode {
                RxMode::Terminator => {
                    let term = self.rx.terminator.as_deref().unwrap_or("").as_bytes();
                    Ok(find(&self.buf, term).map(|i| self.split(i, term.len())))
                }
                // One transport read is one message.
                RxMode::Message => Ok(Some(self.split(self.buf.len(), 0))),
                // Complete only on silence (finish).
                RxMode::Idle => Ok(None),
                RxMode::Fixed => Err(Error::Framing("line reply in fixed rx mode".into())),
            },
        }
    }

    /// The line went idle or the reply timed out.
    pub fn finish(self) -> Result<Framed, Error> {
        let idle_ok = matches!(self.reply, Reply::Line) && matches!(self.rx.mode, RxMode::Idle | RxMode::Message);
        if idle_ok {
            let reply = strip(&self.buf, &self.rx.strip_trailing);
            return Ok(Framed { reply, leftover: Vec::new() });
        }
        if self.buf.is_empty() {
            return Err(Error::Timeout { partial: Vec::new() });
        }
        Err(Error::Timeout { partial: self.buf })
    }

    fn split(&mut self, len: usize, skip: usize) -> Framed {
        let reply = match self.reply {
            Reply::Block => self.buf[..len].to_vec(),
            _ => strip(&self.buf[..len], &self.rx.strip_trailing),
        };
        let leftover = self.buf[(len + skip).min(self.buf.len())..].to_vec();
        Framed { reply, leftover }
    }

    /// IEEE 488.2 definite-length block `#<n><len><payload>`; returns the payload. A short text
    /// prefix before the `#` (Siglent `C1:WF DAT2,`) is skipped.
    fn block(&mut self) -> Result<Option<Framed>, Error> {
        match self.buf.iter().take(BLOCK_PREFIX_MAX + 1).position(|&b| b == b'#') {
            Some(0) => {}
            Some(at) => {
                self.buf.drain(..at);
            }
            None if self.buf.len() > BLOCK_PREFIX_MAX => {
                return Err(Error::Framing(format!("block reply has no '#' header: {:?}", preview(&self.buf))));
            }
            None => return Ok(None),
        }
        if self.buf.len() < 2 {
            return Ok(None);
        }
        let n = (self.buf[1] as char)
            .to_digit(10)
            .filter(|d| *d > 0)
            .ok_or_else(|| Error::Framing("indefinite or invalid IEEE block header".into()))? as usize;
        if self.buf.len() < 2 + n {
            return Ok(None);
        }
        let len: usize = std::str::from_utf8(&self.buf[2..2 + n])
            .ok()
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| Error::Framing("invalid IEEE block length".into()))?;
        let total = 2 + n + len;
        if self.buf.len() < total {
            return Ok(None);
        }
        let payload = self.buf[2 + n..total].to_vec();
        let leftover = self.buf[total..].to_vec();
        Ok(Some(Framed { reply: payload, leftover }))
    }
}

/// Longest text a device may put before a block's `#` header.
const BLOCK_PREFIX_MAX: usize = 64;

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return None;
    }
    hay.windows(needle.len()).position(|w| w == needle)
}

fn strip(bytes: &[u8], chars: &str) -> Vec<u8> {
    let set = chars.as_bytes();
    let end = bytes.iter().rposition(|b| !set.contains(b)).map(|i| i + 1).unwrap_or(0);
    bytes[..end].to_vec()
}

pub(crate) fn preview(b: &[u8]) -> String {
    let s: String = b.iter().take(40).map(|&c| c as char).collect();
    s.escape_debug().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rx(mode: RxMode, term: Option<&str>, skip: &str, strip: &str) -> RxFraming {
        RxFraming {
            mode,
            terminator: term.map(Into::into),
            skip_leading: skip.into(),
            strip_trailing: strip.into(),
            timeout_ms: 300,
            idle_ms: Some(15),
            drain_idle_ms: None,
        }
    }

    #[test]
    fn terminator_with_leading_noise_and_leftover() {
        let r = rx(RxMode::Terminator, Some("\n"), "\r\n", "\r");
        let mut f = Framer::new(&r, &Reply::Line);
        assert_eq!(f.push(b"\n\"VO").unwrap(), None);
        let out = f.push(b"LT\"\r\nOW").unwrap().unwrap();
        assert_eq!(out.reply, b"\"VOLT\"");
        assert_eq!(out.leftover, b"OW");
    }

    #[test]
    fn missing_terminator_times_out_with_partial() {
        let r = rx(RxMode::Terminator, Some("\r"), "", "");
        let mut f = Framer::new(&r, &Reply::Line);
        f.push(b"0.000\n").unwrap();
        assert!(matches!(f.finish(), Err(Error::Timeout { partial }) if partial == b"0.000\n"));
    }

    #[test]
    fn fixed_length_completes_early_in_idle_mode() {
        let r = rx(RxMode::Idle, None, "", "");
        let mut f = Framer::new(&r, &Reply::Bytes(6));
        assert_eq!(f.push(b"0.98").unwrap(), None);
        assert_eq!(f.push(b"8 ").unwrap().unwrap().reply, b"0.988 ");
        let mut f = Framer::new(&r, &Reply::Line);
        f.push(b"TENMA 72-2540 V2.1").unwrap();
        assert_eq!(f.finish().unwrap().reply, b"TENMA 72-2540 V2.1");
    }

    #[test]
    fn ieee_block_payload() {
        let r = rx(RxMode::Message, None, "", "\r\n");
        let mut f = Framer::new(&r, &Reply::Block);
        assert_eq!(f.push(b"#15ab").unwrap(), None);
        let out = f.push(b"cde\n").unwrap().unwrap();
        assert_eq!(out.reply, b"abcde");
        assert_eq!(out.leftover, b"\n");
    }

    #[test]
    fn a_text_prefix_before_the_block_header_is_skipped() {
        let r = rx(RxMode::Message, None, "", "\r\n");
        let mut f = Framer::new(&r, &Reply::Block);
        assert_eq!(f.push(b"C1:WF DAT2,").unwrap(), None, "prefix alone: wait for the header");
        let out = f.push(b"#13\x01\x02\x03\n\n").unwrap().unwrap();
        assert_eq!(out.reply, [1, 2, 3]);
        let mut f = Framer::new(&r, &Reply::Block);
        assert!(f.push(&[b'x'; 80]).is_err(), "no header within the prefix limit");
    }

    #[test]
    fn message_mode_line() {
        let r = rx(RxMode::Message, None, "", "\r\n");
        let mut f = Framer::new(&r, &Reply::Line);
        assert_eq!(f.push(b"0\n").unwrap().unwrap().reply, b"0");
    }
}
