//! What to do when Linux refuses to open an instrument's port. Device nodes of USB instruments
//! (`/dev/usbtmcN`, `/dev/bus/usb/…`) belong to root and serial ports to the `dialout` group
//! until the user sets that up once; the bare "Permission denied (os error 13)" says neither.
//! The message ends with the help page (the UI turns it into a link and shows the command).

use std::io;

/// The website section with the udev rule for USB instruments.
pub const USB_HELP: &str = "https://benchmesh.online/docs/linux#usb-permissions";
/// The website section about the serial-port group.
pub const SERIAL_HELP: &str = "https://benchmesh.online/docs/linux#serial-permissions";

/// The kind of port that was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Port {
    Usb,
    Serial,
}

/// `e` with what to do about it when it is a refused permission on Linux; any other error as it is.
pub fn explain(e: io::Error, port: Port, path: &str) -> io::Error {
    if !cfg!(target_os = "linux") || e.kind() != io::ErrorKind::PermissionDenied {
        return e;
    }
    let msg = match port {
        Port::Usb => format!(
            "no permission to use {path}: Linux gives USB instruments to root only until a udev rule gives them \
             to the logged-in user (one command, once) - see {USB_HELP}"
        ),
        Port::Serial => format!(
            "no permission to use {path}: Linux gives serial ports to the dialout group (uucp on Arch) - add your \
             user to it and log in again - see {SERIAL_HELP}"
        ),
    };
    io::Error::new(io::ErrorKind::PermissionDenied, msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "linux")]
    fn denied() -> io::Error {
        io::Error::from_raw_os_error(13)
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn a_refused_usb_instrument_names_the_node_and_the_udev_help() {
        let e = explain(denied(), Port::Usb, "/dev/usbtmc0");
        assert_eq!(e.kind(), io::ErrorKind::PermissionDenied);
        let m = e.to_string();
        assert!(m.starts_with("no permission to use /dev/usbtmc0: "), "{m}");
        assert!(m.ends_with(USB_HELP), "{m}");
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn a_refused_serial_port_names_the_group_and_its_help() {
        let m = explain(denied(), Port::Serial, "/dev/ttyUSB0").to_string();
        assert!(m.contains("/dev/ttyUSB0") && m.contains("dialout"), "{m}");
        assert!(m.ends_with(SERIAL_HELP), "{m}");
    }

    #[test]
    fn other_errors_stay_as_they_are() {
        let e = explain(io::Error::new(io::ErrorKind::NotFound, "gone"), Port::Usb, "/dev/usbtmc0");
        assert_eq!(e.to_string(), "gone");
    }

    #[test]
    fn help_links_point_at_the_linux_page_sections() {
        // the sections the website's /docs/linux page must keep (BenchMeshOnline docs/linux test)
        assert_eq!(USB_HELP.rsplit_once('#').unwrap().1, "usb-permissions");
        assert_eq!(SERIAL_HELP.rsplit_once('#').unwrap().1, "serial-permissions");
    }
}
