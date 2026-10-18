//! Modbus RTU: request frames with CRC16, reply framing and checks, register decoding, and a
//! simulated slave (tests, manual examples, demo mode).
//!
//! Frame: `address, function, data…, crc_lo, crc_hi` (CRC16/Modbus: poly 0xA001 reflected, init
//! 0xFFFF, low byte first). Supported functions: 01/02 read coils/discrete inputs, 03/04 read
//! holding/input registers, 05 write single coil, 06 write single register, 16 write registers.

use std::collections::BTreeMap;

use bm_devicedef::{ModbusFn, ModbusOp, RegisterField, WordType};
use serde_json::{json, Value};

use crate::Error;

pub fn crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &b in data {
        crc ^= b as u16;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xA001 } else { crc >> 1 };
        }
    }
    crc
}

fn with_crc(mut frame: Vec<u8>) -> Vec<u8> {
    let c = crc16(&frame);
    frame.extend_from_slice(&c.to_le_bytes());
    frame
}

pub fn code(f: ModbusFn) -> u8 {
    match f {
        ModbusFn::ReadCoils => 0x01,
        ModbusFn::ReadDiscrete => 0x02,
        ModbusFn::ReadHolding => 0x03,
        ModbusFn::ReadInput => 0x04,
        ModbusFn::WriteCoil => 0x05,
        ModbusFn::WriteSingle => 0x06,
        ModbusFn::WriteMultiple => 0x10,
    }
}

pub fn is_read(f: ModbusFn) -> bool {
    matches!(f, ModbusFn::ReadCoils | ModbusFn::ReadDiscrete | ModbusFn::ReadHolding | ModbusFn::ReadInput)
}

/// Registers / coils a read asks for.
pub fn count(op: &ModbusOp) -> u16 {
    op.count.unwrap_or(op.encoding.words())
}

/// Register words of a value (big-endian words; `swap`: low word first).
pub fn encode(value: f64, t: WordType, swap: bool) -> Result<Vec<u16>, Error> {
    let range = |lo: f64, hi: f64| {
        if value.is_finite() && value >= lo && value <= hi {
            Ok(value.round())
        } else {
            Err(Error::BadArgument(format!("{value} does not fit a {t:?} register value")))
        }
    };
    let pair = |v: u32| if swap { vec![v as u16, (v >> 16) as u16] } else { vec![(v >> 16) as u16, v as u16] };
    Ok(match t {
        WordType::U16 => vec![range(0.0, u16::MAX as f64)? as u16],
        WordType::I16 => vec![range(i16::MIN as f64, i16::MAX as f64)? as i16 as u16],
        WordType::U32 => pair(range(0.0, u32::MAX as f64)? as u32),
        WordType::I32 => pair(range(i32::MIN as f64, i32::MAX as f64)? as i32 as u32),
        WordType::F32 => pair((value as f32).to_bits()),
    })
}

/// The request frame. `value`: the write's value (coils: 0 = off, anything else = on).
pub fn request(address: u8, op: &ModbusOp, value: Option<f64>) -> Result<Vec<u8>, Error> {
    let [rh, rl] = op.register.to_be_bytes();
    let need = || value.ok_or_else(|| Error::BadArgument("missing value".into()));
    let mut f = vec![address, code(op.function), rh, rl];
    match op.function {
        _ if is_read(op.function) => f.extend_from_slice(&count(op).to_be_bytes()),
        ModbusFn::WriteCoil => f.extend_from_slice(if need()? != 0.0 { &[0xFF, 0x00] } else { &[0x00, 0x00] }),
        ModbusFn::WriteSingle => {
            if op.encoding.words() != 1 {
                return Err(Error::Definition("write_single takes a 16-bit encoding (use write_multiple)".into()));
            }
            f.extend_from_slice(&encode(need()?, op.encoding, op.swap)?[0].to_be_bytes());
        }
        _ => {
            let words = encode(need()?, op.encoding, op.swap)?;
            f.extend_from_slice(&(words.len() as u16).to_be_bytes());
            f.push((words.len() * 2) as u8);
            for w in words {
                f.extend_from_slice(&w.to_be_bytes());
            }
        }
    }
    Ok(with_crc(f))
}

/// Feed the bytes received so far: `Ok(None)` while incomplete, `Ok(Some(data))` with a read's
/// data bytes (empty for a write), or the reason the reply is wrong.
pub fn check_reply(address: u8, op: &ModbusOp, buf: &[u8]) -> Result<Option<Vec<u8>>, Error> {
    if buf.len() < 2 {
        return Ok(None);
    }
    if buf[0] != address {
        return Err(Error::Framing(format!("modbus reply from slave {} (expected {address})", buf[0])));
    }
    let fc = code(op.function);
    let exception = buf[1] == fc | 0x80;
    if !exception && buf[1] != fc {
        return Err(Error::Framing(format!("modbus reply to function {:#04x} (expected {fc:#04x})", buf[1])));
    }
    let len = if exception {
        5
    } else if is_read(op.function) {
        if buf.len() < 3 {
            return Ok(None);
        }
        5 + buf[2] as usize
    } else {
        8
    };
    if buf.len() < len {
        return Ok(None);
    }
    let frame = &buf[..len];
    let crc = u16::from_le_bytes([frame[len - 2], frame[len - 1]]);
    if crc != crc16(&frame[..len - 2]) {
        return Err(Error::Framing("modbus reply CRC mismatch".into()));
    }
    if exception {
        return Err(Error::Refused(format!("modbus exception {}: {}", frame[2], exception_text(frame[2]))));
    }
    Ok(Some(if is_read(op.function) { frame[3..len - 2].to_vec() } else { Vec::new() }))
}

fn exception_text(code: u8) -> &'static str {
    match code {
        1 => "illegal function",
        2 => "illegal data address",
        3 => "illegal data value",
        4 => "slave device failure",
        5 => "acknowledge (busy)",
        6 => "slave device busy",
        _ => "unknown exception",
    }
}

/// A value at `field.at` of register data (big-endian words).
pub fn decode(data: &[u8], field: &RegisterField) -> Result<Value, Error> {
    let word = |i: usize| -> Result<u16, Error> {
        let at = 2 * i;
        data.get(at..at + 2)
            .map(|b| u16::from_be_bytes([b[0], b[1]]))
            .ok_or_else(|| Error::Parse(format!("register {i} is outside the {} registers read", data.len() / 2)))
    };
    let pair = || -> Result<u32, Error> {
        let (a, b) = (word(field.at)? as u32, word(field.at + 1)? as u32);
        Ok(if field.swap { (b << 16) | a } else { (a << 16) | b })
    };
    let raw: f64 = match field.kind {
        WordType::U16 => word(field.at)? as f64,
        WordType::I16 => word(field.at)? as i16 as f64,
        WordType::U32 => pair()? as f64,
        WordType::I32 => pair()? as i32 as f64,
        WordType::F32 => f32::from_bits(pair()?) as f64,
    };
    Ok(match field.scale {
        // keep 12 significant digits: 1234 × 0.01 must read 12.34
        Some(k) => json!(format!("{:.11e}", raw * k).parse::<f64>().unwrap_or(raw * k)),
        None if field.kind != WordType::F32 => json!(raw as i64),
        None => json!(raw),
    })
}

/// A simulated slave over holding/input registers and coils: answers reads, applies writes,
/// replies with exception 2 for unknown addresses. For `SimTransport`.
#[derive(Debug, Clone, Default)]
pub struct Slave {
    pub address: u8,
    pub holding: BTreeMap<u16, u16>,
    pub input: BTreeMap<u16, u16>,
    pub coils: BTreeMap<u16, bool>,
}

impl Slave {
    pub fn respond(&mut self, req: &[u8]) -> Option<Vec<u8>> {
        if req.len() < 8 || req[0] != self.address || crc16(&req[..req.len() - 2]).to_le_bytes() != req[req.len() - 2..] {
            return None; // not for us / corrupt: a real slave stays silent
        }
        let (fc, reg) = (req[1], u16::from_be_bytes([req[2], req[3]]));
        let val = u16::from_be_bytes([req[4], req[5]]);
        let fail = |e: u8| Some(with_crc(vec![self.address, fc | 0x80, e]));
        match fc {
            0x01 | 0x02 => {
                let mut bytes = vec![0u8; (val as usize).div_ceil(8)];
                for i in 0..val {
                    match self.coils.get(&(reg + i)) {
                        Some(true) => bytes[(i / 8) as usize] |= 1 << (i % 8),
                        Some(false) => {}
                        None => return fail(2),
                    }
                }
                let mut f = vec![self.address, fc, bytes.len() as u8];
                f.extend(bytes);
                Some(with_crc(f))
            }
            0x03 | 0x04 => {
                let map = if fc == 0x03 { &self.holding } else { &self.input };
                let mut f = vec![self.address, fc, (val * 2) as u8];
                for i in 0..val {
                    let Some(w) = map.get(&(reg + i)) else { return fail(2) };
                    f.extend_from_slice(&w.to_be_bytes());
                }
                Some(with_crc(f))
            }
            0x05 => {
                if !self.coils.contains_key(&reg) {
                    return fail(2);
                }
                self.coils.insert(reg, val == 0xFF00);
                Some(req[..8].to_vec())
            }
            0x06 => {
                if !self.holding.contains_key(&reg) {
                    return fail(2);
                }
                self.holding.insert(reg, val);
                Some(req[..8].to_vec())
            }
            0x10 => {
                for i in 0..val {
                    let at = 7 + 2 * i as usize;
                    let Some(b) = req.get(at..at + 2) else { return fail(3) };
                    if !self.holding.contains_key(&(reg + i)) {
                        return fail(2);
                    }
                    self.holding.insert(reg + i, u16::from_be_bytes([b[0], b[1]]));
                }
                Some(with_crc(req[..6].to_vec()))
            }
            _ => fail(1),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn op(yaml: &str) -> ModbusOp {
        bm_devicedef::from_yaml(yaml).unwrap()
    }

    #[test]
    fn frames_match_the_riden_examples() {
        // PSU.md coverage report: read 8..18 = 01 03 00 08 00 0B <crc>; output on = 01 06 00 12 00 01 <crc>
        let read = request(1, &op("{fn: read_holding, register: 8, count: 11}"), None).unwrap();
        assert_eq!(&read[..6], [0x01, 0x03, 0x00, 0x08, 0x00, 0x0B]);
        assert_eq!(crc16(&read), 0, "a frame with its CRC appended checks to zero");
        let on = request(1, &op("{fn: write_single, register: 18, value: v}"), Some(1.0)).unwrap();
        assert_eq!(&on[..6], [0x01, 0x06, 0x00, 0x12, 0x00, 0x01]);
        // well-known reference: 01 03 00 00 00 01 → CRC 84 0A
        assert_eq!(request(1, &op("{fn: read_holding, register: 0}"), None).unwrap(), [1, 3, 0, 0, 0, 1, 0x84, 0x0A]);
    }

    #[test]
    fn replies_are_framed_checked_and_decoded() {
        let mut s = Slave { address: 1, holding: [(10, 1234), (11, 567)].into(), ..Slave::default() };
        let o = op("{fn: read_holding, register: 10, count: 2}");
        let reply = s.respond(&request(1, &o, None).unwrap()).unwrap();
        assert_eq!(check_reply(1, &o, &reply[..4]).unwrap(), None, "incomplete");
        let data = check_reply(1, &o, &reply).unwrap().unwrap();
        let f = |at, scale| RegisterField { at, kind: WordType::U16, scale: Some(scale), swap: false };
        assert_eq!(decode(&data, &f(0, 0.01)).unwrap(), json!(12.34));
        assert_eq!(decode(&data, &f(1, 0.001)).unwrap(), json!(0.567));
        let mut bad = reply.clone();
        bad[3] ^= 1;
        assert!(check_reply(1, &o, &bad).is_err(), "CRC");
        // unknown register → exception 2
        let e = s.respond(&request(1, &op("{fn: read_holding, register: 99}"), None).unwrap()).unwrap();
        assert!(matches!(check_reply(1, &op("{fn: read_holding, register: 99}"), &e), Err(Error::Refused(m)) if m.contains("illegal data address")));
    }

    #[test]
    fn writes_change_the_slave_and_32_bit_values_round_trip() {
        let mut s = Slave { address: 1, holding: [(8, 0), (20, 0), (21, 0)].into(), ..Slave::default() };
        let w = op("{fn: write_single, register: 8, value: v}");
        let echo = s.respond(&request(1, &w, Some(500.0)).unwrap()).unwrap();
        assert_eq!(check_reply(1, &w, &echo).unwrap(), Some(vec![]));
        assert_eq!(s.holding[&8], 500);
        for (t, v) in [(WordType::U32, 100_000.0), (WordType::I32, -5.0), (WordType::F32, 1.5)] {
            for swap in [false, true] {
                let w = ModbusOp { function: ModbusFn::WriteMultiple, register: 20, count: None, value: Some("v".into()), encoding: t, swap };
                let echo = s.respond(&request(1, &w, Some(v)).unwrap()).unwrap();
                check_reply(1, &w, &echo).unwrap();
                let data: Vec<u8> = [s.holding[&20], s.holding[&21]].iter().flat_map(|w| w.to_be_bytes()).collect();
                let got = decode(&data, &RegisterField { at: 0, kind: t, scale: None, swap }).unwrap();
                assert_eq!(got.as_f64(), Some(v), "{t:?} swap={swap}");
            }
        }
        assert!(encode(70_000.0, WordType::U16, false).is_err());
    }
}
