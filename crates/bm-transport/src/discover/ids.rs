//! Pure parsing of the OS USB identifiers used by the Windows and macOS backends (compiled and
//! tested on every OS).

use super::hex4;

/// Windows hardware id `USB\VID_1A86&PID_7523&REV_0264` → (`1a86`, `7523`, `0264`).
/// Also accepts `+`-separated ids (`FTDIBUS\VID_0403+PID_6001+...`); missing parts are `None`.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn win_hardware_id(hwid: &str) -> (Option<String>, Option<String>, Option<String>) {
    let upper = hwid.to_ascii_uppercase();
    let field = |tag: &str| upper.find(tag).and_then(|i| upper.get(i + tag.len()..i + tag.len() + 4)).and_then(hex4);
    (field("VID_"), field("PID_"), field("REV_"))
}

/// A Windows instance id of a USB *device* node: `USB\VID_..&PID_..\...` but not an interface
/// of a composite device (`USB\VID_..&PID_..&MI_00\...`).
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn win_is_usb_device(instance_id: &str) -> bool {
    let upper = instance_id.to_ascii_uppercase();
    upper.starts_with("USB\\VID_") && !upper.contains("&MI_")
}

/// The USB serial number from a USB device instance id (`USB\VID_0416&PID_5011\A02014090305` →
/// `A02014090305`). Ids whose last segment contains `&` (`5&2A3B4C5D&0&4`) are synthesized by
/// Windows for devices without a serial number → `None`.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn win_instance_serial(instance_id: &str) -> Option<String> {
    let last = instance_id.rsplit('\\').next()?.trim();
    (instance_id.contains('\\') && !last.is_empty() && !last.contains('&')).then(|| last.to_string())
}

/// macOS `idVendor` / `idProduct` / `bcdDevice` (CFNumber) → 4 lowercase hex digits.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) fn mac_hex4(n: i64) -> Option<String> {
    u16::try_from(n).ok().and_then(|v| hex4(&format!("{v:x}")))
}

/// macOS `locationID` (CFNumber) → `0x14320000`.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) fn mac_location(n: i64) -> Option<String> {
    (0..=u32::MAX as i64).contains(&n).then(|| format!("0x{n:08x}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &str) -> Option<String> {
        Some(v.to_string())
    }

    #[test]
    fn parses_windows_hardware_ids() {
        assert_eq!(win_hardware_id(r"USB\VID_1A86&PID_7523&REV_0264"), (s("1a86"), s("7523"), s("0264")));
        assert_eq!(win_hardware_id(r"USB\VID_0416&PID_5011"), (s("0416"), s("5011"), None));
        assert_eq!(win_hardware_id(r"FTDIBUS\VID_0403+PID_6001+AL036DJTA\0000"), (s("0403"), s("6001"), None));
        assert_eq!(win_hardware_id(r"ACPI\PNP0501"), (None, None, None));
        assert_eq!(win_hardware_id(r"USB\VID_12"), (None, None, None));
    }

    #[test]
    fn detects_usb_device_nodes() {
        assert!(win_is_usb_device(r"USB\VID_0416&PID_5011\A02014090305"));
        assert!(win_is_usb_device(r"USB\VID_1A86&PID_7523\5&2A3B4C5D&0&4"));
        assert!(!win_is_usb_device(r"USB\VID_0416&PID_5011&MI_00\6&1234ABCD&0&0000"));
        assert!(!win_is_usb_device(r"FTDIBUS\VID_0403+PID_6001+AL036DJTA\0000"));
        assert!(!win_is_usb_device(r"USB\ROOT_HUB30\4&1A2B3C4D&0&0"));
        assert!(!win_is_usb_device(r"ACPI\PNP0501\0"));
    }

    #[test]
    fn extracts_serial_from_instance_ids() {
        assert_eq!(win_instance_serial(r"USB\VID_0416&PID_5011\A02014090305"), s("A02014090305"));
        assert_eq!(win_instance_serial(r"USB\VID_0403&PID_6001\AL036DJT"), s("AL036DJT"));
        assert_eq!(win_instance_serial(r"USB\VID_1A86&PID_7523\5&2A3B4C5D&0&4"), None);
        assert_eq!(win_instance_serial(r"USB\VID_1A86&PID_7523\"), None);
        assert_eq!(win_instance_serial("garbage"), None);
    }

    #[test]
    fn formats_macos_numbers() {
        assert_eq!(mac_hex4(0x1a86).as_deref(), Some("1a86"));
        assert_eq!(mac_hex4(0x264).as_deref(), Some("0264"));
        assert_eq!(mac_hex4(0x10000), None);
        assert_eq!(mac_hex4(-1), None);
        assert_eq!(mac_location(0x14320000).as_deref(), Some("0x14320000"));
        assert_eq!(mac_location(0x00100000).as_deref(), Some("0x00100000"));
        assert_eq!(mac_location(-1), None);
    }
}
