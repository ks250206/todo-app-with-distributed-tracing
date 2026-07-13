use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

const SOURCE_LIMIT: u32 = 20;
const SOURCE_WINDOW: Duration = Duration::from_secs(60);
const LOGIN_ACCOUNT_LIMIT: u32 = 10;
const LOGIN_ACCOUNT_WINDOW: Duration = Duration::from_secs(5 * 60);
const REGISTER_SOURCE_LIMIT: u32 = 5;
const REGISTER_SOURCE_WINDOW: Duration = Duration::from_secs(60);

#[derive(Clone, Default)]
pub struct AuthRateLimiter {
    buckets: Arc<Mutex<HashMap<String, Bucket>>>,
}

struct Bucket {
    attempts: u32,
    expires_at: Instant,
}

impl AuthRateLimiter {
    pub fn check_login(&self, source: &str, account: &str) -> bool {
        self.check([
            (format!("auth:source:{source}"), SOURCE_LIMIT, SOURCE_WINDOW),
            (
                format!("login:account:{account}"),
                LOGIN_ACCOUNT_LIMIT,
                LOGIN_ACCOUNT_WINDOW,
            ),
        ])
    }

    pub fn check_register(&self, source: &str, account: &str) -> bool {
        self.check([
            (format!("auth:source:{source}"), SOURCE_LIMIT, SOURCE_WINDOW),
            (
                format!("register:source:{source}"),
                REGISTER_SOURCE_LIMIT,
                REGISTER_SOURCE_WINDOW,
            ),
            (
                format!("register:account:{account}"),
                REGISTER_SOURCE_LIMIT,
                REGISTER_SOURCE_WINDOW,
            ),
        ])
    }

    fn check<const N: usize>(&self, rules: [(String, u32, Duration); N]) -> bool {
        let now = Instant::now();
        let mut buckets = self.buckets.lock().expect("rate limiter lock poisoned");
        buckets.retain(|_, bucket| bucket.expires_at > now);

        let allowed = rules.iter().all(|(key, limit, _)| {
            buckets
                .get(key)
                .is_none_or(|bucket| bucket.attempts < *limit)
        });
        if !allowed {
            return false;
        }

        for (key, _, window) in rules {
            let bucket = buckets.entry(key).or_insert(Bucket {
                attempts: 0,
                expires_at: now + window,
            });
            bucket.attempts += 1;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_login_by_account_across_sources() {
        let limiter = AuthRateLimiter::default();
        for index in 0..LOGIN_ACCOUNT_LIMIT {
            assert!(limiter.check_login(&format!("192.0.2.{index}"), "account"));
        }
        assert!(!limiter.check_login("198.51.100.1", "account"));
    }

    #[test]
    fn limits_registration_by_source() {
        let limiter = AuthRateLimiter::default();
        for index in 0..REGISTER_SOURCE_LIMIT {
            assert!(limiter.check_register("192.0.2.1", &format!("account-{index}")));
        }
        assert!(!limiter.check_register("192.0.2.1", "another-account"));
    }
}
