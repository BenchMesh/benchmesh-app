//! Password hashes: PBKDF2-HMAC-SHA256 with a random salt (`ring`); verification is
//! constant-time. Only the hash is stored (`remote.json`).

use std::num::NonZeroU32;

use ring::pbkdf2;
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};

pub const MIN_LEN: usize = 10;
pub const MAX_LEN: usize = 1024;
const ROUNDS: u32 = 600_000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PasswordHash {
    pub salt: String,
    pub iterations: u32,
    pub hash: String,
}

/// Why a new password is not accepted, if it is not.
pub fn refusal(password: &str) -> Option<String> {
    let n = password.chars().count();
    if n < MIN_LEN {
        Some(format!("the password needs at least {MIN_LEN} characters"))
    } else if password.len() > MAX_LEN {
        Some(format!("the password may have at most {MAX_LEN} bytes"))
    } else {
        None
    }
}

pub fn hash(password: &str) -> PasswordHash {
    hash_with(password, ROUNDS)
}

/// `hash` with a chosen number of rounds (tests: fewer, so they stay fast).
pub fn hash_with(password: &str, iterations: u32) -> PasswordHash {
    let salt = random_bytes::<16>();
    let mut out = [0u8; 32];
    pbkdf2::derive(pbkdf2::PBKDF2_HMAC_SHA256, NonZeroU32::new(iterations.max(1)).unwrap(), &salt, password.as_bytes(), &mut out);
    PasswordHash { salt: hex(&salt), iterations: iterations.max(1), hash: hex(&out) }
}

pub fn verify(h: &PasswordHash, password: &str) -> bool {
    let (Some(salt), Some(expected), Some(n)) = (unhex(&h.salt), unhex(&h.hash), NonZeroU32::new(h.iterations)) else {
        return false;
    };
    password.len() <= MAX_LEN && pbkdf2::verify(pbkdf2::PBKDF2_HMAC_SHA256, n, &salt, password.as_bytes(), &expected).is_ok()
}

pub fn random_bytes<const N: usize>() -> [u8; N] {
    let mut b = [0u8; N];
    SystemRandom::new().fill(&mut b).expect("the system random number generator failed");
    b
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len()).step_by(2).map(|i| s.get(i..i + 2).and_then(|p| u8::from_str_radix(p, 16).ok())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_right_password_verifies_and_others_do_not() {
        let h = hash_with("correct horse battery", 1000);
        assert!(verify(&h, "correct horse battery"));
        assert!(!verify(&h, "correct horse batterY"));
        assert!(!verify(&h, ""));
        assert_ne!(h.salt, hash_with("correct horse battery", 1000).salt, "a new salt every time");
    }

    #[test]
    fn a_damaged_hash_never_verifies() {
        let mut h = hash_with("correct horse battery", 1000);
        h.hash = "zz".into();
        assert!(!verify(&h, "correct horse battery"));
        h.iterations = 0;
        assert!(!verify(&h, "correct horse battery"));
    }

    #[test]
    fn short_and_huge_passwords_are_refused() {
        assert!(refusal("123456789").is_some());
        assert!(refusal("1234567890").is_none());
        assert!(refusal(&"x".repeat(MAX_LEN + 1)).is_some());
        assert_eq!(hash("x").iterations, ROUNDS);
    }
}
