//! Linux kernel USBTMC (`/dev/usbtmcN`).
//!
//! Each `read()` on the device node issues one USBTMC `REQUEST_DEV_DEP_MSG_IN`; the
//! buffer must be large enough for the whole message (the OWON DGE ignores the requested
//! transfer size and streams the full message). The per-call timeout is set with
//! `USBTMC_IOCTL_SET_TIMEOUT`.

use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::time::Duration;

use bm_driver::engine::Transport;

const USBTMC_IOC_NR: u64 = 91;
// _IOW(USBTMC_IOC_NR, 10, __u32)
const USBTMC_IOCTL_SET_TIMEOUT: u64 = (1 << 30) | (4 << 16) | (USBTMC_IOC_NR << 8) | 10;
// _IO(USBTMC_IOC_NR, 2)
const USBTMC_IOCTL_CLEAR: u64 = (USBTMC_IOC_NR << 8) | 2;
/// Kernel minimum (USBTMC_MIN_TIMEOUT).
const MIN_TIMEOUT_MS: u32 = 100;
/// Large enough for any message we expect (a DGE screenshot is ~392 KB).
const MAX_MESSAGE: usize = 1 << 20;

pub struct UsbTmcTransport {
    file: File,
    buf: Vec<u8>,
    timeout_ms: u32,
}

impl UsbTmcTransport {
    /// Open the device. No automatic device clear: on the DHO804 a clear sometimes recovers a
    /// device left one reply behind by an interrupted session, and sometimes leaves the next
    /// exchange unanswered (verified 2026-10-17). Interrupted sessions are avoided instead
    /// (graceful shutdown); proper abort/resync needs the userspace USBTMC transport.
    pub fn open(path: &str) -> io::Result<Self> {
        let file = OpenOptions::new().read(true).write(true).open(path)?;
        Ok(UsbTmcTransport { file, buf: vec![0; MAX_MESSAGE], timeout_ms: 0 })
    }

    fn set_timeout(&mut self, t: Duration) -> io::Result<()> {
        let ms = (t.as_millis() as u32).max(MIN_TIMEOUT_MS);
        if ms != self.timeout_ms {
            let v: u32 = ms;
            // SAFETY: valid fd; the ioctl reads one u32 through the pointer.
            let r = unsafe { libc::ioctl(self.file.as_raw_fd(), USBTMC_IOCTL_SET_TIMEOUT as _, &v as *const u32) };
            if r < 0 {
                return Err(io::Error::last_os_error());
            }
            self.timeout_ms = ms;
        }
        Ok(())
    }

    /// USBTMC device clear (resets the device's input/output buffers).
    pub fn clear(&mut self) -> io::Result<()> {
        // SAFETY: valid fd, argument-less ioctl.
        let r = unsafe { libc::ioctl(self.file.as_raw_fd(), USBTMC_IOCTL_CLEAR as _) };
        if r < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

impl Transport for UsbTmcTransport {
    fn write(&mut self, data: &[u8]) -> io::Result<()> {
        self.file.write_all(data)
    }

    fn read(&mut self, timeout: Duration) -> io::Result<Vec<u8>> {
        self.set_timeout(timeout)?;
        match self.file.read(&mut self.buf) {
            Ok(n) => Ok(self.buf[..n].to_vec()),
            Err(e) if e.raw_os_error() == Some(libc::ETIMEDOUT) => Ok(Vec::new()),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ioctl_numbers_match_linux_headers() {
        // linux/usb/tmc.h: USBTMC_IOCTL_SET_TIMEOUT = 0x40045b0a, USBTMC_IOCTL_CLEAR = 0x5b02
        assert_eq!(USBTMC_IOCTL_SET_TIMEOUT, 0x4004_5b0a);
        assert_eq!(USBTMC_IOCTL_CLEAR, 0x5b02);
    }
}
