use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Fetch,
    Verify,
    Build,
    Activate,
    Check,
}

impl Phase {
    pub fn label(&self) -> &'static str {
        match self {
            Phase::Fetch => "fetch",
            Phase::Verify => "verify",
            Phase::Build => "build",
            Phase::Activate => "activate",
            Phase::Check => "check",
        }
    }

    pub fn all() -> [Phase; 5] {
        [
            Phase::Fetch,
            Phase::Verify,
            Phase::Build,
            Phase::Activate,
            Phase::Check,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Queued,
    Running,
    Succeeded,
    Degraded,
    Failed,
    Rejected,
    Superseded,
    Cancelled,
    Interrupted,
}

impl State {
    pub fn is_terminal(&self) -> bool {
        !matches!(self, State::Queued | State::Running)
    }

    /// Whether the node ended up on the newly built generation.
    pub fn switched(&self) -> bool {
        matches!(self, State::Succeeded | State::Degraded)
    }

    pub fn label(&self) -> &'static str {
        match self {
            State::Queued => "queued",
            State::Running => "running",
            State::Succeeded => "succeeded",
            State::Degraded => "degraded",
            State::Failed => "failed",
            State::Rejected => "rejected",
            State::Superseded => "superseded",
            State::Cancelled => "cancelled",
            State::Interrupted => "interrupted",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployError {
    pub phase: Phase,
    pub exit_code: Option<i32>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deployment {
    pub id: u64,
    pub rev: String,
    #[serde(default)]
    pub subject: Option<String>,
    pub state: State,
    #[serde(default)]
    pub phase: Option<Phase>,
    pub queued_at: i64,
    #[serde(default)]
    pub started_at: Option<i64>,
    #[serde(default)]
    pub finished_at: Option<i64>,
    #[serde(default)]
    pub durations: BTreeMap<Phase, f64>,
    #[serde(default)]
    pub toplevel: Option<String>,
    #[serde(default)]
    pub generation: Option<u64>,
    #[serde(default)]
    pub previous_generation: Option<u64>,
    #[serde(default)]
    pub error: Option<DeployError>,
    #[serde(default)]
    pub new_failed_units: Vec<String>,
    #[serde(default)]
    pub preexisting_failed_units: Vec<String>,
    #[serde(default)]
    pub log_bytes: u64,
    #[serde(default)]
    pub log_reaped: bool,
}

impl Deployment {
    fn new(id: u64, rev: &str, now: i64) -> Self {
        Deployment {
            id,
            rev: rev.to_string(),
            subject: None,
            state: State::Queued,
            phase: None,
            queued_at: now,
            started_at: None,
            finished_at: None,
            durations: BTreeMap::new(),
            toplevel: None,
            generation: None,
            previous_generation: None,
            error: None,
            new_failed_units: Vec::new(),
            preexisting_failed_units: Vec::new(),
            log_bytes: 0,
            log_reaped: false,
        }
    }

    pub fn total_duration(&self) -> Option<f64> {
        match (self.started_at, self.finished_at) {
            (Some(a), Some(b)) => Some((b - a) as f64),
            _ => None,
        }
    }

    fn finish(&mut self, state: State, now: i64) {
        self.state = state;
        self.phase = None;
        self.finished_at = Some(now);
    }
}

/// Result of admitting a deploy request.
#[derive(Debug, PartialEq, Eq)]
pub enum Admission {
    /// Nothing was active, this one runs immediately.
    Started(u64),
    /// Something is running, this one waits.
    Queued(u64),
    /// An identical rev was already active, no new record.
    Existing(u64),
}

impl Admission {
    pub fn id(&self) -> u64 {
        match self {
            Admission::Started(id) | Admission::Queued(id) | Admission::Existing(id) => *id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Store {
    /// Random per-store token. Ids restart from 1 if the state file is ever
    /// lost, so callers pair the two to know they are polling what they think.
    pub epoch: String,
    pub next_id: u64,
    /// Newest first.
    pub deployments: Vec<Deployment>,
    /// The newest commit a hub has told us about.
    #[serde(default)]
    pub target: Option<String>,
    /// A commit waiting for someone to approve it, on nodes that do not
    /// deploy on their own.
    #[serde(default)]
    pub available: Option<String>,
}

/// What became of a hub telling us about a commit.
#[derive(Debug, PartialEq, Eq)]
pub enum Offer {
    /// Already deployed or already tried, nothing to do.
    Known,
    Admitted(Admission),
    /// Recorded as available, waiting for an approval.
    Waiting,
}

pub fn is_hex_sha(rev: &str) -> bool {
    rev.len() == 40 && rev.chars().all(|c| c.is_ascii_hexdigit())
}

impl Store {
    pub fn new(epoch: String) -> Self {
        Store {
            epoch,
            next_id: 1,
            deployments: Vec::new(),
            target: None,
            available: None,
        }
    }

    /// Whether `rev` has had its chance here. A failure counts: both hubs
    /// repeat the target every few seconds, and retrying a broken commit on
    /// each of those would loop forever. A retry is an explicit approval.
    fn attempted(&self, rev: &str) -> bool {
        self.deployments.iter().any(|d| {
            d.rev == rev
                && !matches!(
                    d.state,
                    State::Superseded | State::Cancelled | State::Interrupted
                )
        })
    }

    /// A hub says `rev` is the newest commit. Both hubs say so independently,
    /// so this has to be idempotent.
    pub fn offer(&mut self, rev: &str, auto: bool, now: i64) -> Offer {
        self.target = Some(rev.to_string());
        if self.attempted(rev) {
            return Offer::Known;
        }
        if auto {
            Offer::Admitted(self.admit(rev, now))
        } else {
            self.available = Some(rev.to_string());
            Offer::Waiting
        }
    }

    /// Someone asked for `rev` by hand, either an update waiting on a node
    /// that does not deploy itself or a retry of one that failed.
    pub fn approve(&mut self, rev: &str, now: i64) -> Admission {
        if self.available.as_deref() == Some(rev) {
            self.available = None;
        }
        self.admit(rev, now)
    }

    pub fn latest(&self) -> Option<&Deployment> {
        self.deployments.first()
    }

    pub fn get(&self, id: u64) -> Option<&Deployment> {
        self.deployments.iter().find(|d| d.id == id)
    }

    pub fn get_mut(&mut self, id: u64) -> Option<&mut Deployment> {
        self.deployments.iter_mut().find(|d| d.id == id)
    }

    pub fn running(&self) -> Option<&Deployment> {
        self.deployments.iter().find(|d| d.state == State::Running)
    }

    pub fn queued(&self) -> Option<&Deployment> {
        self.deployments.iter().find(|d| d.state == State::Queued)
    }

    /// Most recent deployment that actually switched the system.
    pub fn last_switched(&self) -> Option<&Deployment> {
        self.deployments.iter().find(|d| d.state.switched())
    }

    fn active_with_rev(&self, rev: &str) -> Option<u64> {
        self.deployments
            .iter()
            .find(|d| !d.state.is_terminal() && d.rev == rev)
            .map(|d| d.id)
    }

    /// Admit a request for `rev`.
    ///
    /// At most one deployment runs and at most one waits. A second waiting
    /// request replaces the first, because only the newest commit is worth
    /// deploying and the one it displaced never needed to run.
    pub fn admit(&mut self, rev: &str, now: i64) -> Admission {
        if let Some(id) = self.active_with_rev(rev) {
            return Admission::Existing(id);
        }

        let id = self.next_id;
        self.next_id += 1;
        let mut deployment = Deployment::new(id, rev, now);

        if self.running().is_none() {
            deployment.state = State::Running;
            deployment.started_at = Some(now);
            deployment.phase = Some(Phase::Fetch);
            self.deployments.insert(0, deployment);
            Admission::Started(id)
        } else {
            if let Some(waiting) = self
                .deployments
                .iter_mut()
                .find(|d| d.state == State::Queued)
            {
                waiting.finish(State::Superseded, now);
            }
            self.deployments.insert(0, deployment);
            Admission::Queued(id)
        }
    }

    /// Promote the waiting deployment once the running one is done.
    pub fn take_queued(&mut self, now: i64) -> Option<u64> {
        if self.running().is_some() {
            return None;
        }
        let waiting = self
            .deployments
            .iter_mut()
            .find(|d| d.state == State::Queued)?;
        waiting.state = State::Running;
        waiting.started_at = Some(now);
        waiting.phase = Some(Phase::Fetch);
        Some(waiting.id)
    }

    /// Retire a request that waited so long it is no longer worth deploying.
    pub fn expire_queued(&mut self, max_age: i64, now: i64) -> Option<u64> {
        let waiting = self
            .deployments
            .iter_mut()
            .find(|d| d.state == State::Queued && now - d.queued_at >= max_age)?;
        waiting.finish(State::Superseded, now);
        Some(waiting.id)
    }

    pub fn finish(&mut self, id: u64, state: State, now: i64) {
        if let Some(d) = self.get_mut(id) {
            d.finish(state, now);
        }
    }

    /// After a restart, a record still marked running whose runner is gone was
    /// killed by a reboot or an OOM. Without this the phantom blocks admission
    /// forever.
    /// `grace` protects a just-started deployment: the runner is started with
    /// --no-block, so for a moment the unit does not exist yet and looks dead.
    pub fn reconcile(&mut self, now: i64, grace: i64, is_active: impl Fn(u64) -> bool) -> Vec<u64> {
        let mut interrupted = Vec::new();
        for d in self.deployments.iter_mut() {
            let settling = d.started_at.map(|t| now - t < grace).unwrap_or(false);
            if d.state == State::Running && !settling && !is_active(d.id) {
                d.finish(State::Interrupted, now);
                interrupted.push(d.id);
            }
        }
        interrupted
    }

    /// Records are small, so we keep far more of them than logs.
    pub fn prune(&mut self, max_records: usize) {
        if self.deployments.len() > max_records {
            self.deployments.truncate(max_records);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> Store {
        Store::new("test".to_string())
    }

    #[test]
    fn first_request_runs_immediately() {
        let mut s = store();
        assert_eq!(s.admit("aaa", 10), Admission::Started(1));
        assert_eq!(s.running().unwrap().rev, "aaa");
        assert_eq!(s.running().unwrap().phase, Some(Phase::Fetch));
    }

    #[test]
    fn second_request_waits() {
        let mut s = store();
        s.admit("aaa", 10);
        assert_eq!(s.admit("bbb", 11), Admission::Queued(2));
        assert_eq!(s.running().unwrap().rev, "aaa");
        assert_eq!(s.queued().unwrap().rev, "bbb");
    }

    #[test]
    fn identical_rev_is_idempotent() {
        let mut s = store();
        s.admit("aaa", 10);
        // same rev while running
        assert_eq!(s.admit("aaa", 11), Admission::Existing(1));
        s.admit("bbb", 12);
        // same rev while queued
        assert_eq!(s.admit("bbb", 13), Admission::Existing(2));
        assert_eq!(s.deployments.len(), 2);
    }

    #[test]
    fn third_request_supersedes_the_waiting_one() {
        let mut s = store();
        s.admit("aaa", 10);
        s.admit("bbb", 11);
        assert_eq!(s.admit("ccc", 12), Admission::Queued(3));

        assert_eq!(s.get(2).unwrap().state, State::Superseded);
        assert_eq!(s.get(2).unwrap().finished_at, Some(12));
        assert_eq!(s.queued().unwrap().rev, "ccc");
        // the running one is never displaced
        assert_eq!(s.running().unwrap().rev, "aaa");
    }

    #[test]
    fn queued_is_promoted_when_running_finishes() {
        let mut s = store();
        s.admit("aaa", 10);
        s.admit("bbb", 11);

        assert_eq!(s.take_queued(12), None, "cannot promote while one runs");
        s.finish(1, State::Succeeded, 12);
        assert_eq!(s.take_queued(13), Some(2));
        assert_eq!(s.running().unwrap().rev, "bbb");
        assert_eq!(s.running().unwrap().started_at, Some(13));
    }

    #[test]
    fn stale_queued_request_is_retired() {
        let mut s = store();
        s.admit("aaa", 0);
        s.admit("bbb", 10);

        assert_eq!(s.expire_queued(1800, 1000), None);
        assert_eq!(s.expire_queued(1800, 1810), Some(2));
        assert_eq!(s.get(2).unwrap().state, State::Superseded);
    }

    #[test]
    fn reboot_mid_deploy_is_interrupted() {
        let mut s = store();
        s.admit("aaa", 10);
        s.admit("bbb", 11);

        // the runner unit for 1 is gone, nothing wrote a terminal record
        assert_eq!(s.reconcile(60, 20, |_| false), vec![1]);
        assert_eq!(s.get(1).unwrap().state, State::Interrupted);
        // the waiting one is untouched and can now run
        assert_eq!(s.get(2).unwrap().state, State::Queued);
        assert_eq!(s.take_queued(21), Some(2));
    }

    #[test]
    fn a_just_started_runner_is_left_alone() {
        let mut s = store();
        s.admit("aaa", 10);
        // started at 10, checked at 15, grace 20: still settling even though
        // the unit is not visible yet
        assert!(s.reconcile(15, 20, |_| false).is_empty());
        assert_eq!(s.get(1).unwrap().state, State::Running);
        // past the grace window it is fair game
        assert_eq!(s.reconcile(40, 20, |_| false), vec![1]);
    }

    #[test]
    fn a_live_runner_is_adopted_not_interrupted() {
        let mut s = store();
        s.admit("aaa", 10);
        assert!(s.reconcile(60, 20, |_| true).is_empty());
        assert_eq!(s.get(1).unwrap().state, State::Running);
    }

    #[test]
    fn ids_keep_climbing_across_supersedes() {
        let mut s = store();
        s.admit("aaa", 0);
        for (i, rev) in ["b", "c", "d"].iter().enumerate() {
            s.admit(rev, i as i64 + 1);
        }
        assert_eq!(s.next_id, 5);
        assert_eq!(s.deployments.len(), 4);
    }

    #[test]
    fn prune_keeps_the_newest() {
        let mut s = store();
        s.admit("aaa", 0);
        s.finish(1, State::Succeeded, 1);
        for i in 0..10 {
            let rev = format!("rev{i}");
            let id = s.admit(&rev, i + 2).id();
            s.finish(id, State::Succeeded, i + 3);
        }
        s.prune(3);
        assert_eq!(s.deployments.len(), 3);
        assert_eq!(s.deployments[0].rev, "rev9");
    }

    #[test]
    fn the_same_offer_from_both_hubs_deploys_once() {
        let mut s = store();
        assert_eq!(
            s.offer("aaa", true, 1),
            Offer::Admitted(Admission::Started(1))
        );
        assert_eq!(s.offer("aaa", true, 2), Offer::Known);
        s.finish(1, State::Succeeded, 3);
        assert_eq!(s.offer("aaa", true, 4), Offer::Known);
        assert_eq!(s.deployments.len(), 1);
    }

    #[test]
    fn a_failed_commit_is_not_retried_on_its_own() {
        let mut s = store();
        s.offer("aaa", true, 1);
        s.finish(1, State::Failed, 2);
        assert_eq!(s.offer("aaa", true, 3), Offer::Known);
        assert_eq!(s.approve("aaa", 4), Admission::Started(2));
    }

    #[test]
    fn an_interrupted_commit_gets_another_go() {
        let mut s = store();
        s.offer("aaa", true, 1);
        s.finish(1, State::Interrupted, 2);
        assert_eq!(
            s.offer("aaa", true, 3),
            Offer::Admitted(Admission::Started(2))
        );
    }

    #[test]
    fn a_manual_node_waits_for_approval() {
        let mut s = store();
        assert_eq!(s.offer("aaa", false, 1), Offer::Waiting);
        assert!(s.deployments.is_empty());
        assert_eq!(s.available.as_deref(), Some("aaa"));

        assert_eq!(s.offer("bbb", false, 2), Offer::Waiting);
        assert_eq!(s.available.as_deref(), Some("bbb"));

        assert_eq!(s.approve("bbb", 3), Admission::Started(1));
        assert_eq!(s.available, None);
        assert_eq!(s.target.as_deref(), Some("bbb"));
    }

    #[test]
    fn revs_must_look_like_revs() {
        assert!(is_hex_sha("d6459d9903c9c12e92cdb542018f82d743867963"));
        assert!(!is_hex_sha("d6459d9"));
        assert!(!is_hex_sha("main"));
    }

    #[test]
    fn an_old_state_file_still_loads() {
        let raw = r#"{"epoch":"x","next_id":3,"deployments":[]}"#;
        let s: Store = serde_json::from_str(raw).unwrap();
        assert_eq!(s.target, None);
        assert_eq!(s.available, None);
    }

    #[test]
    fn switched_states_are_the_ones_on_the_new_generation() {
        assert!(State::Succeeded.switched());
        assert!(State::Degraded.switched());
        assert!(!State::Failed.switched());
        assert!(!State::Rejected.switched());
        assert!(State::Superseded.is_terminal());
        assert!(!State::Running.is_terminal());
    }
}
