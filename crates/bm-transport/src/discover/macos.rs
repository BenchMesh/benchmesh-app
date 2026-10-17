//! macOS: USB serial ports (`/dev/cu.*`) via the IORegistry (no root needed).
//!
//! `IOSerialBSDClient` services → `IOCalloutDevice` path → walk up the IOService plane to the USB
//! device (nearest ancestor with `idVendor` that is not an interface, i.e. no `bInterfaceNumber`)
//! → its ids, strings and `locationID`. Ports without a USB ancestor (Bluetooth, debug-console)
//! are left out.

use core_foundation::base::{CFType, TCFType};
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;
use core_foundation_sys::base::kCFAllocatorDefault;
use io_kit_sys::keys::kIOServicePlane;
use io_kit_sys::types::{io_iterator_t, io_object_t};
use io_kit_sys::{IOIteratorNext, IOObjectRelease, IORegistryEntryCreateCFProperty, IORegistryEntryGetParentEntry, IOServiceGetMatchingServices, IOServiceMatching};

use super::ids::{mac_hex4, mac_location};
use super::{text, Port, PortKind, UsbInfo};

/// `MACH_PORT_NULL` selects the default main port (`kIOMainPortDefault`).
const MAIN_PORT_DEFAULT: u32 = 0;
const KERN_SUCCESS: i32 = 0;
/// IOService levels walked up from the serial client (client → driver → interface → device …).
const MAX_DEPTH: usize = 16;

fn property(entry: io_object_t, key: &str) -> Option<CFType> {
    let key = CFString::new(key);
    // SAFETY: `entry` is a live registry entry; the returned object follows the create rule.
    let r = unsafe { IORegistryEntryCreateCFProperty(entry, key.as_concrete_TypeRef(), kCFAllocatorDefault, 0) };
    // SAFETY: non-null and owned by us (create rule).
    (!r.is_null()).then(|| unsafe { CFType::wrap_under_create_rule(r) })
}

fn number(entry: io_object_t, key: &str) -> Option<i64> {
    property(entry, key)?.downcast::<CFNumber>()?.to_i64()
}

/// The first of `keys` present as a string property.
fn string(entry: io_object_t, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|k| text(property(entry, k)?.downcast::<CFString>().map(|s| s.to_string())))
}

/// The USB device at or above `entry` (a new reference the caller releases), or `None`.
fn usb_device(entry: io_object_t) -> Option<io_object_t> {
    let mut node = entry;
    for _ in 0..MAX_DEPTH {
        let mut parent: io_object_t = 0;
        // SAFETY: `node` is live; `parent` is a valid out pointer.
        let r = unsafe { IORegistryEntryGetParentEntry(node, kIOServicePlane, &mut parent) };
        if node != entry {
            // SAFETY: intermediate parents were obtained by us above and are released once.
            unsafe { IOObjectRelease(node) };
        }
        if r != KERN_SUCCESS {
            return None;
        }
        node = parent;
        if property(node, "idVendor").is_some() && property(node, "bInterfaceNumber").is_none() {
            return Some(node);
        }
    }
    if node != entry {
        // SAFETY: the last parent obtained in the loop is released once.
        unsafe { IOObjectRelease(node) };
    }
    None
}

fn usb_info(device: io_object_t) -> UsbInfo {
    UsbInfo {
        vendor_id: number(device, "idVendor").and_then(mac_hex4),
        product_id: number(device, "idProduct").and_then(mac_hex4),
        manufacturer: string(device, &["USB Vendor Name", "kUSBVendorString"]),
        product: string(device, &["USB Product Name", "kUSBProductString"]),
        serial: string(device, &["USB Serial Number", "kUSBSerialNumberString"]),
        revision: number(device, "bcdDevice").and_then(mac_hex4),
        location: number(device, "locationID").and_then(mac_location),
    }
}

fn port(service: io_object_t) -> Option<Port> {
    let path = string(service, &["IOCalloutDevice"])?;
    let device = usb_device(service)?;
    let usb = usb_info(device);
    // SAFETY: `device` is the reference returned by `usb_device`, released once.
    unsafe { IOObjectRelease(device) };
    Some(Port { kind: PortKind::Serial, path, usb })
}

pub fn ports() -> Vec<Port> {
    // SAFETY: a NUL-terminated class name; the dictionary is consumed by IOServiceGetMatchingServices.
    let matching = unsafe { IOServiceMatching(c"IOSerialBSDClient".as_ptr()) };
    if matching.is_null() {
        return Vec::new();
    }
    let mut iter: io_iterator_t = 0;
    // SAFETY: `matching` is a valid dictionary (one reference consumed); `iter` a valid out pointer.
    if unsafe { IOServiceGetMatchingServices(MAIN_PORT_DEFAULT, matching, &mut iter) } != KERN_SUCCESS {
        return Vec::new();
    }
    let mut out = Vec::new();
    loop {
        // SAFETY: `iter` is a live iterator; each returned object is released below.
        let service = unsafe { IOIteratorNext(iter) };
        if service == 0 {
            break;
        }
        out.extend(port(service));
        // SAFETY: `service` came from IOIteratorNext and is released once.
        unsafe { IOObjectRelease(service) };
    }
    // SAFETY: `iter` came from IOServiceGetMatchingServices and is released once.
    unsafe { IOObjectRelease(iter) };
    out
}
