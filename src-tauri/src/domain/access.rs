use serde::Serialize;
use std::time::{Duration, Instant};

pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5 * 60);

#[derive(Clone, Copy, Default, PartialEq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AccessStatus {
    #[default]
    Pending,
    Checking,
    Ready,
    Denied,
    Expired,
    Unavailable,
}

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrimaryAccess {
    pub status: AccessStatus,
    pub message: String,
    #[serde(skip)]
    pub key: Option<String>,
    #[serde(skip)]
    pub owner: Option<String>,
    #[serde(skip)]
    pub checked_at: Option<Instant>,
    #[serde(skip)]
    pub heartbeat_at: Option<Instant>,
}

impl PrimaryAccess {
    pub fn ready(&self) -> bool {
        self.status == AccessStatus::Ready
    }

    pub fn needs_check(&self, key: &str, force: bool) -> bool {
        if self.key.as_deref() != Some(key) {
            return true;
        }
        // A rejected token must never be accepted again from stale page storage.
        if self.status == AccessStatus::Expired {
            return false;
        }
        let interval = if self.status == AccessStatus::Unavailable {
            Duration::from_secs(30)
        } else {
            HEARTBEAT_INTERVAL
        };
        force
            || self
                .checked_at
                .is_none_or(|time| time.elapsed() >= interval)
    }

    pub fn heartbeat_due(&self) -> bool {
        self.heartbeat_at
            .is_none_or(|time| time.elapsed() >= HEARTBEAT_INTERVAL)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expiry_requires_new_credentials_and_never_reuses_old_grant() {
        let mut gate = PrimaryAccess {
            status: AccessStatus::Ready,
            key: Some("old".into()),
            checked_at: Some(Instant::now()),
            ..Default::default()
        };
        assert!(!gate.needs_check("old", false));
        assert!(gate.needs_check("old", true));
        assert!(gate.needs_check("new", false));
        gate.status = AccessStatus::Expired;
        assert!(!gate.ready());
        assert!(!gate.needs_check("old", true));
        assert!(gate.needs_check("new", false));
    }

    #[test]
    fn heartbeat_and_permission_checks_have_independent_clocks() {
        let mut gate = PrimaryAccess {
            checked_at: Some(Instant::now()),
            heartbeat_at: Some(Instant::now() - HEARTBEAT_INTERVAL),
            ..Default::default()
        };
        assert!(gate.heartbeat_due());
        gate.heartbeat_at = Some(Instant::now());
        assert!(!gate.heartbeat_due());
        let json = serde_json::to_value(gate).unwrap();
        assert!(json.get("key").is_none());
        assert!(json.get("owner").is_none());
    }
}
