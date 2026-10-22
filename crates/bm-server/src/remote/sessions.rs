//! Signed-in remote browsers. The cookie carries a random 256-bit token; only its SHA-256 is
//! kept, in memory (a restart signs everyone out). Ending a session also tells its open
//! WebSockets to close (`revoked`). A session has a [`Role`]: what its password allows.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;

use ring::digest;
use serde::Serialize;
use serde_json::{json, Value};
use tokio::sync::watch;

use super::password::{hex, random_bytes};

pub const IDLE_MS: i64 = 12 * 3600 * 1000;
pub const MAX_AGE_MS: i64 = 7 * 24 * 3600 * 1000;
/// More sign-ins than this end the oldest session.
const MAX_SESSIONS: usize = 64;

/// Which password a session signed in with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// The remote access password: use the bench as from the app (except this computer's settings).
    Control,
    /// The view-only password: see the bench, change nothing.
    View,
}

struct Session {
    id: String,
    role: Role,
    address: IpAddr,
    agent: String,
    since: i64,
    last_seen: i64,
    revoke: watch::Sender<bool>,
}

/// What a request learns about its session.
#[derive(Debug, Clone)]
pub struct SessionView {
    pub id: String,
    pub role: Role,
    pub address: IpAddr,
    pub since: i64,
    pub revoked: watch::Receiver<bool>,
    key: String,
}

#[derive(Default)]
pub struct Sessions {
    by_key: Mutex<HashMap<String, Session>>,
}

fn key_of(token: &str) -> String {
    hex(digest::digest(&digest::SHA256, token.as_bytes()).as_ref())
}

fn end(s: Session) {
    let _ = s.revoke.send(true);
}

impl Sessions {
    /// A new session; returns the token for the cookie.
    pub fn create(&self, address: IpAddr, agent: &str, role: Role, now: i64) -> String {
        let token = hex(&random_bytes::<32>());
        let (revoke, _) = watch::channel(false);
        let agent: String = agent.chars().take(200).collect();
        let s = Session { id: hex(&random_bytes::<8>()), role, address, agent, since: now, last_seen: now, revoke };
        let mut m = self.by_key.lock().unwrap();
        m.retain(|_, s| alive(s, now));
        while m.len() >= MAX_SESSIONS {
            let oldest = m.iter().min_by_key(|(_, s)| s.since).map(|(k, _)| k.clone()).unwrap();
            if let Some(s) = m.remove(&oldest) {
                end(s);
            }
        }
        m.insert(key_of(&token), s);
        token
    }

    /// The session of a cookie token, if it is still valid (and marks it as seen).
    pub fn touch(&self, token: &str, now: i64) -> Option<SessionView> {
        let key = key_of(token);
        let mut m = self.by_key.lock().unwrap();
        let s = m.get_mut(&key)?;
        if !alive(s, now) {
            if let Some(s) = m.remove(&key) {
                end(s);
            }
            return None;
        }
        s.last_seen = now;
        Some(SessionView { id: s.id.clone(), role: s.role, address: s.address, since: s.since, revoked: s.revoke.subscribe(), key })
    }

    pub fn end_view(&self, v: &SessionView) {
        if let Some(s) = self.by_key.lock().unwrap().remove(&v.key) {
            end(s);
        }
    }

    /// End the session with this id (from the list); false when there is none.
    pub fn end_id(&self, id: &str) -> bool {
        let mut m = self.by_key.lock().unwrap();
        let key = m.iter().find(|(_, s)| s.id == id).map(|(k, _)| k.clone());
        key.and_then(|k| m.remove(&k)).map(end).is_some()
    }

    pub fn end_all(&self) {
        for (_, s) in self.by_key.lock().unwrap().drain() {
            end(s);
        }
    }

    /// End every session of one role (its password changed or was removed).
    pub fn end_role(&self, role: Role) {
        let mut m = self.by_key.lock().unwrap();
        let keys: Vec<String> = m.iter().filter(|(_, s)| s.role == role).map(|(k, _)| k.clone()).collect();
        for k in keys {
            if let Some(s) = m.remove(&k) {
                end(s);
            }
        }
    }

    /// `[{id, role, address, agent, since, last_seen}]`, oldest first.
    pub fn list(&self, now: i64) -> Vec<Value> {
        let mut m = self.by_key.lock().unwrap();
        m.retain(|_, s| alive(s, now));
        let mut v: Vec<&Session> = m.values().collect();
        v.sort_by_key(|s| s.since);
        v.iter().map(|s| json!({"id": s.id, "role": s.role, "address": s.address.to_string(), "agent": s.agent, "since": s.since, "last_seen": s.last_seen})).collect()
    }
}

fn alive(s: &Session, now: i64) -> bool {
    now - s.last_seen < IDLE_MS && now - s.since < MAX_AGE_MS
}

#[cfg(test)]
mod tests {
    use super::*;

    const IP: IpAddr = IpAddr::V4(std::net::Ipv4Addr::new(192, 168, 1, 30));

    #[test]
    fn a_token_finds_its_session_until_it_ends() {
        let s = Sessions::default();
        let t = s.create(IP, "Firefox", Role::Control, 1000);
        let v = s.touch(&t, 2000).unwrap();
        assert_eq!(v.address, IP);
        assert!(s.touch("not-a-token", 2000).is_none());
        let list = s.list(2000);
        assert_eq!(list[0]["last_seen"], 2000);
        assert_eq!(list[0]["agent"], "Firefox");
        assert_eq!(list[0]["role"], "control");
        assert!(list[0].get("key").is_none() && !list[0].to_string().contains(&t), "the token is never listed");
        s.end_view(&v);
        assert!(s.touch(&t, 3000).is_none());
        assert!(*v.revoked.borrow(), "open WebSockets are told");
    }

    #[test]
    fn idle_and_old_sessions_expire() {
        let s = Sessions::default();
        let t = s.create(IP, "", Role::Control, 0);
        assert!(s.touch(&t, IDLE_MS - 1).is_some());
        assert!(s.touch(&t, 2 * IDLE_MS).is_none(), "idle too long");
        let t = s.create(IP, "", Role::Control, 0);
        let mut now = 0;
        while now < MAX_AGE_MS - IDLE_MS {
            now += IDLE_MS / 2;
            assert!(s.touch(&t, now).is_some());
        }
        assert!(s.touch(&t, MAX_AGE_MS).is_none(), "at most a week, even when used");
    }

    #[test]
    fn sessions_end_one_by_id_or_all() {
        let s = Sessions::default();
        let a = s.create(IP, "a", Role::Control, 1);
        let b = s.create(IP, "b", Role::Control, 2);
        let id = s.list(3)[0]["id"].as_str().unwrap().to_string();
        assert!(s.end_id(&id));
        assert!(!s.end_id(&id));
        assert!(s.touch(&a, 4).is_none() && s.touch(&b, 4).is_some());
        let v = s.touch(&b, 5).unwrap();
        s.end_all();
        assert!(s.touch(&b, 6).is_none() && *v.revoked.borrow());
    }

    #[test]
    fn sessions_of_one_role_end_together() {
        let s = Sessions::default();
        let control = s.create(IP, "", Role::Control, 1);
        let view = s.create(IP, "", Role::View, 2);
        assert_eq!(s.touch(&view, 3).unwrap().role, Role::View);
        assert_eq!(s.list(3)[1]["role"], "view");
        s.end_role(Role::View);
        assert!(s.touch(&view, 4).is_none() && s.touch(&control, 4).is_some());
    }

    #[test]
    fn too_many_sign_ins_end_the_oldest() {
        let s = Sessions::default();
        let first = s.create(IP, "", Role::Control, 0);
        for i in 1..=MAX_SESSIONS as i64 {
            s.create(IP, "", Role::Control, i);
        }
        assert!(s.touch(&first, 100).is_none());
        assert_eq!(s.list(100).len(), MAX_SESSIONS);
    }
}
