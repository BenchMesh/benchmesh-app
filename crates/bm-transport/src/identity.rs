//! Instrument identity: find a configured instrument on whatever port it is plugged into
//! (contract: `ai_context/migration/IDENTITY.md`).

use serde::{Deserialize, Serialize};

use crate::discover::{Port, PortKind, UsbInfo};

/// USB fingerprint. `vid`, `pid`, `serial`, `revision` must match when present; `location`
/// only orders candidates.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UsbMatch {
    pub vid: String,
    pub pid: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub serial: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
}

/// The config entry's `match`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    pub usb: UsbMatch,
    /// Serial number in the instrument's identify reply.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idn_serial: Option<String>,
}

fn same(a: &Option<String>, b: &Option<String>) -> bool {
    a.as_deref().map(str::to_ascii_lowercase) == b.as_deref().map(str::to_ascii_lowercase)
}

impl UsbMatch {
    pub fn matches(&self, u: &UsbInfo) -> bool {
        let opt = |want: &Option<String>, have: &Option<String>| want.is_none() || same(want, have);
        same(&Some(self.vid.clone()), &u.vendor_id)
            && same(&Some(self.pid.clone()), &u.product_id)
            && opt(&self.serial, &u.serial)
            && opt(&self.revision, &u.revision)
    }

    /// The part that must match (everything but the location).
    fn key(&self) -> (String, String, Option<String>, Option<String>) {
        let l = |s: &Option<String>| s.as_deref().map(str::to_ascii_lowercase);
        (self.vid.to_ascii_lowercase(), self.pid.to_ascii_lowercase(), l(&self.serial), l(&self.revision))
    }
}

/// How certainly an identity picks out one instrument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Certainty {
    /// The identify reply carries the instrument's serial number.
    Instrument,
    /// The USB device has a serial number.
    Adapter,
    /// No serial number, but no other attached device has the same vid/pid/revision.
    Chip,
    /// Only the USB port position separates it from an identical device.
    Position,
}

impl Certainty {
    pub fn as_str(self) -> &'static str {
        match self {
            Certainty::Instrument => "instrument",
            Certainty::Adapter => "adapter",
            Certainty::Chip => "chip",
            Certainty::Position => "position",
        }
    }
}

impl Identity {
    /// The identity of the instrument found on `port` (identify reply `idn`).
    pub fn of(port: &Port, idn: Option<&str>) -> Option<Identity> {
        let u = &port.usb;
        let serial = u.serial.clone();
        Some(Identity {
            usb: UsbMatch {
                vid: u.vendor_id.clone()?,
                pid: u.product_id.clone()?,
                revision: if serial.is_none() { u.revision.clone() } else { None },
                serial,
                location: u.location.clone(),
            },
            idn_serial: idn.and_then(idn_serial),
        })
    }

    /// Ports of `kind` this identity may be on, best first: same location, then the last
    /// known path (`hint`), then the rest. Ports held by an open transport are left out.
    pub fn candidates<'a>(&self, kind: PortKind, ports: &'a [Port], hint: Option<&str>) -> Vec<&'a Port> {
        let mut v: Vec<&Port> =
            ports.iter().filter(|p| p.kind == kind && self.usb.matches(&p.usb) && !crate::lock::is_held(&p.path)).collect();
        let rank = |p: &Port| {
            let at_location = self.usb.location.is_some() && same(&self.usb.location, &p.usb.location);
            let at_hint = hint.is_some_and(|h| h == p.path);
            (!at_location, !at_hint)
        };
        v.sort_by_key(|p| rank(p));
        v
    }

    /// Accept an identify reply: it must contain `idn_serial` as a whole token.
    pub fn check_idn(&self, idn: &str) -> Result<(), String> {
        match &self.idn_serial {
            Some(want) if !idn.split(|c: char| c == ',' || c.is_whitespace()).any(|t| t.trim() == want) => {
                Err(format!("identify reply {idn:?} is not serial number {want}"))
            }
            _ => Ok(()),
        }
    }

    pub fn certainty(&self, others: &[Identity]) -> Certainty {
        if self.idn_serial.is_some() {
            Certainty::Instrument
        } else if self.usb.serial.is_some() {
            Certainty::Adapter
        } else if others.iter().filter(|o| o.usb.key() == self.usb.key()).count() <= 1 {
            Certainty::Chip
        } else {
            Certainty::Position
        }
    }

    /// Short text for errors and logs, e.g. `usb 1a86:7523 rev 0264, serial number 24512010`.
    pub fn describe(&self) -> String {
        let u = &self.usb;
        let mut s = format!("usb {}:{}", u.vid, u.pid);
        if let Some(x) = &u.serial {
            s += &format!(" serial {x}");
        }
        if let Some(x) = &u.revision {
            s += &format!(" rev {x}");
        }
        if let Some(x) = &self.idn_serial {
            s += &format!(", serial number {x}");
        }
        s
    }
}

/// Serial number in a SCPI `*IDN?` reply (`maker,model,serial,firmware`), if it has one.
pub fn idn_serial(idn: &str) -> Option<String> {
    let f: Vec<&str> = idn.split(',').map(str::trim).collect();
    let s = *f.get(2)?;
    (f.len() >= 4 && s.len() >= 3 && s.chars().any(|c| c.is_ascii_alphanumeric() && c != '0')).then(|| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn port(kind: PortKind, path: &str, vid: &str, pid: &str, serial: Option<&str>, rev: &str, loc: &str) -> Port {
        Port {
            kind,
            path: path.into(),
            usb: UsbInfo {
                vendor_id: Some(vid.into()),
                product_id: Some(pid.into()),
                serial: serial.map(Into::into),
                revision: Some(rev.into()),
                location: Some(loc.into()),
                ..UsbInfo::default()
            },
        }
    }

    /// The bench of 2026-10-17 (values read from sysfs).
    fn bench() -> Vec<Port> {
        use PortKind::*;
        vec![
            port(Serial, "/dev/ttyACM0", "0416", "5011", Some("A02014090305"), "0300", "1-1.3.2.2.2"),
            port(Serial, "/dev/ttyUSB0", "1a86", "7523", None, "8134", "1-1.3.2.4"),
            port(Serial, "/dev/ttyUSB1", "0403", "6001", Some("AL036DJT"), "0600", "1-1.3.2.2.1"),
            port(Serial, "/dev/ttyUSB2", "1a86", "7523", None, "0264", "1-1.3.2.2.4"),
            port(Usbtmc, "/dev/usbtmc2", "5345", "1235", Some("25130086"), "0100", "1-1.3.2.2.3"),
            port(Usbtmc, "/dev/usbtmc3", "1ab1", "044d", Some("DHO8A272907001"), "0404", "1-2"),
        ]
    }

    #[test]
    fn idn_serial_numbers() {
        assert_eq!(idn_serial("OWON,XDM1241,24512010,V4.3.0,3").as_deref(), Some("24512010"));
        assert_eq!(idn_serial("RIGOL TECHNOLOGIES,DHO804,DHO8A272907001,00.01.03").as_deref(), Some("DHO8A272907001"));
        assert_eq!(idn_serial("TENMA 72-2540 V2.1"), None);
        assert_eq!(idn_serial("ACME,PSU1,0,1.0"), None);
        assert_eq!(idn_serial("ACME,PSU1,,1.0"), None);
    }

    #[test]
    fn serial_less_ch340s_are_told_apart_by_revision() {
        let ports = bench();
        let xdm = Identity::of(&ports[3], Some("OWON,XDM1241,24512010,V4.3.0,3")).unwrap();
        assert_eq!(xdm.usb.revision.as_deref(), Some("0264"));
        assert_eq!(xdm.usb.serial, None);
        let found: Vec<&str> = xdm.candidates(PortKind::Serial, &ports, None).iter().map(|p| p.path.as_str()).collect();
        assert_eq!(found, ["/dev/ttyUSB2"]);
    }

    #[test]
    fn devices_with_a_serial_number_are_matched_by_it_not_by_revision() {
        let ports = bench();
        let tenma = Identity::of(&ports[0], Some("TENMA 72-2540 V2.1")).unwrap();
        assert_eq!(tenma.usb.serial.as_deref(), Some("A02014090305"));
        assert_eq!(tenma.usb.revision, None, "a firmware update may change the revision of a built-in chip");
        assert_eq!(tenma.idn_serial, None);
        let mut moved = ports.clone();
        moved[0].path = "COM7".into();
        moved[0].usb.revision = Some("0301".into());
        moved[0].usb.location = Some("elsewhere".into());
        assert_eq!(tenma.candidates(PortKind::Serial, &moved, None)[0].path, "COM7");
    }

    #[test]
    fn kind_must_match() {
        let ports = bench();
        let scope = Identity::of(&ports[5], None).unwrap();
        assert!(scope.candidates(PortKind::Serial, &ports, None).is_empty());
        assert_eq!(scope.candidates(PortKind::Usbtmc, &ports, None)[0].path, "/dev/usbtmc3");
    }

    #[test]
    fn identical_adapters_are_ordered_by_location_then_hint() {
        use PortKind::Serial;
        let ports = vec![
            port(Serial, "/dev/ttyUSB5", "1a86", "7523", None, "0264", "1-4"),
            port(Serial, "/dev/ttyUSB6", "1a86", "7523", None, "0264", "1-5"),
            port(Serial, "/dev/ttyUSB7", "1a86", "7523", None, "0264", "1-6"),
        ];
        let mut id = Identity::of(&ports[1], None).unwrap();
        let order = |id: &Identity, hint| id.candidates(Serial, &ports, hint).iter().map(|p| p.path.clone()).collect::<Vec<_>>();
        assert_eq!(order(&id, Some("/dev/ttyUSB7")), ["/dev/ttyUSB6", "/dev/ttyUSB7", "/dev/ttyUSB5"]);
        id.usb.location = Some("moved".into());
        assert_eq!(order(&id, Some("/dev/ttyUSB7"))[0], "/dev/ttyUSB7");
        let all: Vec<Identity> = ports.iter().map(|p| Identity::of(p, None).unwrap()).collect();
        assert_eq!(all[0].certainty(&all), Certainty::Position);
    }

    #[test]
    fn held_ports_are_not_candidates() {
        // a path of its own: the lock is process-wide and other tests run in parallel
        let mut ports = bench();
        ports[3].path = "/dev/ttyUSB-held-test".into();
        let xdm = Identity::of(&ports[3], None).unwrap();
        let _g = crate::lock::acquire("/dev/ttyUSB-held-test").unwrap();
        assert!(xdm.candidates(PortKind::Serial, &ports, None).is_empty());
    }

    #[test]
    fn identify_reply_must_carry_the_serial_number() {
        let id = Identity { idn_serial: Some("24512010".into()), ..Identity::default() };
        assert!(id.check_idn("OWON,XDM1241,24512010,V4.3.0,3").is_ok());
        assert!(id.check_idn("OWON,XDM1241,24512011,V4.3.0,3").unwrap_err().contains("not serial number 24512010"));
        assert!(id.check_idn("OWON,XDM1241,124512010,V4.3.0,3").is_err(), "whole token only");
        assert!(Identity::default().check_idn("TENMA 72-2540 V2.1").is_ok());
    }

    #[test]
    fn certainty_on_the_bench() {
        let ports = bench();
        let idns = [
            "TENMA 72-2540 V2.1",
            "OWON,SPM3103,24460763,FV:V2.1.0",
            "OWON,OEL1515,25130087,FV:V1.1.0",
            "OWON,XDM1241,24512010,V4.3.0,3",
        ];
        let all: Vec<Identity> = ports.iter().take(4).zip(idns).map(|(p, i)| Identity::of(p, Some(i)).unwrap()).collect();
        assert_eq!(all[0].certainty(&all), Certainty::Adapter);
        assert_eq!(all[1].certainty(&all), Certainty::Instrument);
        let mut no_idn = all[3].clone();
        no_idn.idn_serial = None;
        assert_eq!(no_idn.certainty(&all), Certainty::Chip);
    }

    #[test]
    fn config_round_trip() {
        let id: Identity = serde_json_like(r#"{"usb":{"vid":"1a86","pid":"7523","revision":"0264"},"idn_serial":"24512010"}"#);
        assert_eq!(id.usb.revision.as_deref(), Some("0264"));
        assert_eq!(id.describe(), "usb 1a86:7523 rev 0264, serial number 24512010");
    }

    fn serde_json_like(s: &str) -> Identity {
        serde_json::from_str(s).unwrap()
    }
}
