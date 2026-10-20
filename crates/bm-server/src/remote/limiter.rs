//! Slows down password guessing: per address, after 5 failed sign-ins each further try waits
//! 1 s, doubling up to 15 min; at most 30 failures per minute from all addresses together.

use std::collections::{HashMap, VecDeque};
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const FREE_TRIES: u32 = 5;
const MAX_WAIT: Duration = Duration::from_secs(15 * 60);
const GLOBAL_PER_MINUTE: usize = 30;
const MINUTE: Duration = Duration::from_secs(60);
const MAX_ADDRESSES: usize = 4096;

#[derive(Default)]
pub struct Limiter {
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    per_address: HashMap<IpAddr, (u32, Option<Instant>)>,
    recent: VecDeque<Instant>,
}

impl Limiter {
    /// `Err(wait)` when this address may not try now.
    pub fn check(&self, ip: IpAddr, now: Instant) -> Result<(), Duration> {
        let mut g = self.inner.lock().unwrap();
        while g.recent.front().is_some_and(|t| now.duration_since(*t) >= MINUTE) {
            g.recent.pop_front();
        }
        if g.recent.len() >= GLOBAL_PER_MINUTE {
            return Err(MINUTE - now.duration_since(g.recent[0]));
        }
        match g.per_address.get(&ip) {
            Some((_, Some(until))) if *until > now => Err(*until - now),
            _ => Ok(()),
        }
    }

    pub fn failed(&self, ip: IpAddr, now: Instant) {
        let mut g = self.inner.lock().unwrap();
        g.recent.push_back(now);
        if g.per_address.len() >= MAX_ADDRESSES && !g.per_address.contains_key(&ip) {
            g.per_address.retain(|_, (_, until)| until.is_some_and(|u| u > now));
        }
        let e = g.per_address.entry(ip).or_default();
        e.0 += 1;
        if e.0 >= FREE_TRIES {
            let wait = Duration::from_secs(1u64 << (e.0 - FREE_TRIES).min(20)).min(MAX_WAIT);
            e.1 = Some(now + wait);
        }
    }

    pub fn succeeded(&self, ip: IpAddr) {
        self.inner.lock().unwrap().per_address.remove(&ip);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(n: u8) -> IpAddr {
        IpAddr::V4(std::net::Ipv4Addr::new(192, 168, 1, n))
    }

    #[test]
    fn an_address_waits_longer_after_each_failure() {
        let l = Limiter::default();
        let t0 = Instant::now();
        for _ in 0..4 {
            assert!(l.check(ip(1), t0).is_ok());
            l.failed(ip(1), t0);
        }
        assert!(l.check(ip(1), t0).is_ok(), "4 failures are free");
        l.failed(ip(1), t0);
        assert_eq!(l.check(ip(1), t0), Err(Duration::from_secs(1)));
        assert!(l.check(ip(1), t0 + Duration::from_secs(1)).is_ok());
        l.failed(ip(1), t0);
        assert_eq!(l.check(ip(1), t0), Err(Duration::from_secs(2)));
        assert!(l.check(ip(2), t0).is_ok(), "other addresses are not affected");
        for _ in 0..30 {
            l.failed(ip(3), t0 + Duration::from_secs(70));
        }
        assert_eq!(l.check(ip(3), t0 + Duration::from_secs(70) + MINUTE), Err(MAX_WAIT - MINUTE), "the wait stops growing at 15 min");
        l.succeeded(ip(1));
        assert!(l.check(ip(1), t0 + Duration::from_secs(200)).is_ok(), "a sign-in clears the address");
    }

    #[test]
    fn many_failures_from_everywhere_stop_all_tries_for_a_minute() {
        let l = Limiter::default();
        let t0 = Instant::now();
        for n in 0..GLOBAL_PER_MINUTE as u8 {
            l.failed(ip(n), t0);
        }
        assert!(l.check(ip(200), t0 + Duration::from_secs(10)).is_err());
        assert!(l.check(ip(200), t0 + MINUTE).is_ok());
    }
}
