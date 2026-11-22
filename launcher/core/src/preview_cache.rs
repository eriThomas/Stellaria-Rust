use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::taskbar_model::WindowIdentity;

pub const MAX_PREVIEW_ENTRIES: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewEntry {
    pub identity: WindowIdentity,
    pub width: u32,
    pub height: u32,
    pub png_base64: Option<String>,
    pub captured_at_ms: u64,
    pub fallback_reason: Option<String>,
}

/// Bounded preview cache keyed by `(hwnd, pid, session_generation)` so reused HWNDs never collide.
#[derive(Default)]
pub struct PreviewCache {
    entries: VecDeque<PreviewEntry>,
}

impl PreviewCache {
    pub fn get(&self, identity: WindowIdentity, now_ms: u64, ttl_ms: u64) -> Option<PreviewEntry> {
        self.entries
            .iter()
            .find(|e| {
                e.identity == identity && now_ms.saturating_sub(e.captured_at_ms) <= ttl_ms
            })
            .cloned()
    }

    /// Returns the most recent captured bitmap for `identity` regardless of TTL — used for minimized windows.
    pub fn get_last_bitmap(&self, identity: WindowIdentity) -> Option<PreviewEntry> {
        self.entries
            .iter()
            .find(|e| e.identity == identity && e.png_base64.is_some())
            .cloned()
    }

    pub fn insert(&mut self, entry: PreviewEntry) {
        if let Some(pos) = self.entries.iter().position(|e| e.identity == entry.identity) {
            self.entries.remove(pos);
        }
        self.entries.push_back(entry);
        while self.entries.len() > MAX_PREVIEW_ENTRIES {
            self.entries.pop_front();
        }
    }

    pub fn invalidate_generation(&mut self, current_generation: u64) {
        self.entries
            .retain(|e| e.identity.session_generation == current_generation);
    }

    pub fn remove_hwnd(&mut self, hwnd: u64) {
        self.entries.retain(|e| e.identity.hwnd != hwnd);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prevents_reused_hwnd_collision_across_pid_and_session_generation() {
        let mut cache = PreviewCache::default();
        let old_id = WindowIdentity {
            hwnd: 1234,
            pid: 500,
            session_generation: 1,
        };
        let reused_pid = WindowIdentity {
            hwnd: 1234,
            pid: 999,
            session_generation: 1,
        };
        let reused_gen = WindowIdentity {
            hwnd: 1234,
            pid: 500,
            session_generation: 2,
        };

        cache.insert(PreviewEntry {
            identity: old_id,
            width: 320,
            height: 180,
            png_base64: Some("abc".into()),
            captured_at_ms: 1_000,
            fallback_reason: None,
        });

        assert!(cache.get(old_id, 1_500, 2_000).is_some());
        assert!(cache.get(reused_pid, 1_500, 2_000).is_none());
        assert!(cache.get(reused_gen, 1_500, 2_000).is_none());

        cache.invalidate_generation(2);
        assert!(cache.is_empty());
    }
}
