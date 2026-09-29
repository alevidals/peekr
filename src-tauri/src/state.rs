use std::sync::{
    atomic::AtomicU64,
    Arc,
};
use tokio::sync::{watch, Mutex};

use crate::bitbucket::PullRequest;

/// Shared between the blur handler (lib.rs) and the tray click handler (tray.rs).
///
/// `close_gen` is an ever-increasing generation counter. Every time a close
/// sequence is started (blur or tray click) it is incremented and the new value
/// is captured. The async hide task only actually calls hide() when the counter
/// still matches — any intervening focus-gain or later close attempt bumps the
/// counter and silently cancels outstanding tasks. This avoids all boolean-flag
/// stuck-state races.
///
/// `focused_at` is the timestamp (ms since UNIX_EPOCH) of the last Focused(true)
/// event, used to ignore the spurious Focused(false) that fires right after a
/// programmatic window.hide().
pub struct PopoverHideState {
    pub close_gen: Arc<AtomicU64>,
    pub focused_at: Arc<AtomicU64>,
}

pub struct PrState {
    pub prs: Mutex<Vec<PullRequest>>,
}

impl PrState {
    pub fn new() -> Self {
        Self {
            prs: Mutex::new(Vec::new()),
        }
    }
}

pub struct PollTrigger {
    pub tx: watch::Sender<()>,
}
