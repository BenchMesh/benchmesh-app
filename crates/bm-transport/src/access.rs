//! What to do when the operating system refuses an instrument's port, instead of a bare OS error:
//! - Linux: device nodes of USB instruments (`/dev/usbtmcN`, `/dev/bus/usb/…`) belong to root
//!   and serial ports to the `dialout` group until the user sets that up once ("Permission
//!   denied").
//! - macOS 15 and later: an app needs the Local Network permission to reach instruments on the
//!   LAN; without it a connection fails with "No route to host".
//!
//! The message ends with " - see <help page section>" (the UI turns it into a link and shows
//! the fix).

use std::io;

/// The website section with the udev rule for USB instruments.
pub const USB_HELP: &str = "https://benchmesh.online/docs/linux#usb-permissions";
/// The website section about the serial-port group.
pub const SERIAL_HELP: &str = "https://benchmesh.online/docs/linux#serial-permissions";
/// The website section about macOS's Local Network permission.
pub const LOCAL_NETWORK_HELP: &str = "https://benchmesh.online/docs/macos#local-network";

/// The kind of port that was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Port {
    Usb,
    Serial,
    /// A TCP connection to a LAN instrument (raw SCPI socket, VXI-11).
    Network,
}

/// The platforms with something to explain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Linux,
    Macos,
    Other,
}

impl Platform {
    pub const fn current() -> Platform {
        if cfg!(target_os = "linux") {
            Platform::Linux
        } else if cfg!(target_os = "macos") {
            Platform::Macos
        } else {
            Platform::Other
        }
    }
}

/// macOS `EHOSTUNREACH`: what a connection the Local Network permission blocks fails with.
const MACOS_EHOSTUNREACH: i32 = 65;

/// `e` with what to do about it when it is a refusal this platform explains; any other error as it is.
pub fn explain(e: io::Error, port: Port, path: &str) -> io::Error {
    explained(&e, port, path, Platform::current()).unwrap_or(e)
}

/// The explained error, or None when `e` on `platform` is not such a refusal.
pub fn explained(e: &io::Error, port: Port, path: &str, platform: Platform) -> Option<io::Error> {
    let msg = match (platform, port) {
        (Platform::Linux, Port::Usb) if e.kind() == io::ErrorKind::PermissionDenied => format!(
            "no permission to use {path}: Linux gives USB instruments to root only until a udev rule gives them \
             to the logged-in user (one command, once) - see {USB_HELP}"
        ),
        (Platform::Linux, Port::Serial) if e.kind() == io::ErrorKind::PermissionDenied => format!(
            "no permission to use {path}: Linux gives serial ports to the dialout group (uucp on Arch) - add your \
             user to it and log in again - see {SERIAL_HELP}"
        ),
        (Platform::Macos, Port::Network) if e.raw_os_error() == Some(MACOS_EHOSTUNREACH) => format!(
            "{path}: no route to host. If the instrument is on and on this network, macOS is blocking \
             BenchMesh's local network access: System Settings → Privacy & Security → Local Network, switch \
             BenchMesh on - see {LOCAL_NETWORK_HELP}"
        ),
        _ => return None,
    };
    Some(io::Error::new(e.kind(), msg))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn denied() -> io::Error {
        io::Error::new(io::ErrorKind::PermissionDenied, "Permission denied (os error 13)")
    }

    #[test]
    fn a_refused_usb_instrument_on_linux_names_the_node_and_the_udev_help() {
        let e = explained(&denied(), Port::Usb, "/dev/usbtmc0", Platform::Linux).unwrap();
        assert_eq!(e.kind(), io::ErrorKind::PermissionDenied);
        let m = e.to_string();
        assert!(m.starts_with("no permission to use /dev/usbtmc0: "), "{m}");
        assert!(m.ends_with(USB_HELP), "{m}");
    }

    #[test]
    fn a_refused_serial_port_on_linux_names_the_group_and_its_help() {
        let m = explained(&denied(), Port::Serial, "/dev/ttyUSB0", Platform::Linux).unwrap().to_string();
        assert!(m.contains("/dev/ttyUSB0") && m.contains("dialout"), "{m}");
        assert!(m.ends_with(SERIAL_HELP), "{m}");
    }

    #[test]
    fn an_unreachable_lan_instrument_on_macos_points_at_the_local_network_permission() {
        let e = io::Error::from_raw_os_error(MACOS_EHOSTUNREACH);
        let m = explained(&e, Port::Network, "192.168.1.50:5025", Platform::Macos).unwrap().to_string();
        assert!(m.starts_with("192.168.1.50:5025: no route to host. "), "{m}");
        assert!(m.contains("Local Network") && m.ends_with(LOCAL_NETWORK_HELP), "{m}");
    }

    #[test]
    fn only_the_platform_s_own_refusals_are_explained() {
        assert!(explained(&denied(), Port::Usb, "/dev/usbtmc0", Platform::Macos).is_none());
        assert!(explained(&denied(), Port::Usb, "COM3", Platform::Other).is_none());
        assert!(explained(&io::Error::from_raw_os_error(MACOS_EHOSTUNREACH), Port::Network, "h:5025", Platform::Linux).is_none());
        let refused = io::Error::new(io::ErrorKind::ConnectionRefused, "Connection refused");
        assert!(explained(&refused, Port::Network, "h:5025", Platform::Macos).is_none());
        let gone = io::Error::new(io::ErrorKind::NotFound, "gone");
        assert_eq!(explain(gone, Port::Usb, "/dev/usbtmc0").to_string(), "gone");
    }

    #[test]
    fn help_links_point_at_the_website_sections() {
        // the sections the website keeps (BenchMeshOnline: docs/linux and docs/macos tests)
        assert_eq!(USB_HELP.rsplit_once('#').unwrap().1, "usb-permissions");
        assert_eq!(SERIAL_HELP.rsplit_once('#').unwrap().1, "serial-permissions");
        assert_eq!(LOCAL_NETWORK_HELP, "https://benchmesh.online/docs/macos#local-network");
    }
}
