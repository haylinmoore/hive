use serde::Deserialize;
use std::collections::BTreeMap;
use std::io;
use std::process::Command;

/// Unit types worth watching. Devices and scopes churn for reasons that have
/// nothing to do with a deploy, so they are left out.
const PATTERNS: [&str; 4] = ["*.service", "*.socket", "*.timer", "*.path"];

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UnitState {
    pub active: String,
    pub sub: String,
    pub nrestarts: u32,
}

impl UnitState {
    pub fn failed(&self) -> bool {
        self.active == "failed" || self.sub == "failed"
    }
}

pub type Snapshot = BTreeMap<String, UnitState>;

#[derive(Deserialize)]
struct ListedUnit {
    unit: String,
    active: String,
    sub: String,
}

/// `systemctl list-units -o json`. Note it is `-o json`, not `--json`, which
/// list-units does not accept.
pub fn snapshot() -> io::Result<Snapshot> {
    let mut cmd = Command::new("systemctl");
    cmd.arg("list-units")
        .args(PATTERNS)
        .args(["--all", "-o", "json"]);
    let out = cmd.output()?;
    if !out.status.success() {
        return Err(io::Error::other(format!(
            "systemctl list-units failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    let mut snap = parse_list_units(&String::from_utf8_lossy(&out.stdout))?;

    // Restart counts live on the service interface only.
    let out = Command::new("systemctl")
        .args(["show", "*.service", "-p", "Id", "-p", "NRestarts"])
        .output()?;
    if out.status.success() {
        for (unit, n) in parse_show_restarts(&String::from_utf8_lossy(&out.stdout)) {
            if let Some(entry) = snap.get_mut(&unit) {
                entry.nrestarts = n;
            }
        }
    }

    Ok(snap)
}

fn parse_list_units(stdout: &str) -> io::Result<Snapshot> {
    let listed: Vec<ListedUnit> =
        serde_json::from_str(stdout.trim()).map_err(|e| io::Error::other(e.to_string()))?;
    Ok(listed
        .into_iter()
        .map(|u| {
            (
                u.unit,
                UnitState {
                    active: u.active,
                    sub: u.sub,
                    nrestarts: 0,
                },
            )
        })
        .collect())
}

/// `systemctl show` emits blank-line separated stanzas of Key=Value.
fn parse_show_restarts(stdout: &str) -> Vec<(String, u32)> {
    let mut out = Vec::new();
    let mut id: Option<String> = None;
    let mut restarts: Option<u32> = None;

    for line in stdout.lines() {
        // Stanzas are separated by a blank line. Resetting there stops an
        // entry without NRestarts from pairing with the next one's count.
        if line.is_empty() {
            id = None;
            restarts = None;
            continue;
        }
        if let Some(v) = line.strip_prefix("Id=") {
            id = Some(v.to_string());
        } else if let Some(v) = line.strip_prefix("NRestarts=") {
            restarts = v.parse().ok();
        }
        if let (Some(i), Some(r)) = (&id, restarts) {
            out.push((i.clone(), r));
            id = None;
            restarts = None;
        }
    }
    out
}

pub fn failed_units(snap: &Snapshot) -> Vec<String> {
    snap.iter()
        .filter(|(_, s)| s.failed())
        .map(|(n, _)| n.clone())
        .collect()
}

/// Units that this deploy broke.
///
/// A plain "is anything failed" check is useless here: one long-broken unit
/// would fail every deploy forever. What matters is the difference. Units
/// already failing beforehand are reported separately, and a unit that no
/// longer exists in the new generation is simply gone.
///
/// The restart count matters as much as the state. A `Restart=always` unit in
/// backoff sits in `activating (auto-restart)` and may never be caught in
/// `failed` at all, so a crash loop is only visible as a climbing NRestarts.
pub fn newly_failed(before: &Snapshot, after: &Snapshot, ignored: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for (unit, now) in after {
        if ignored.iter().any(|i| i == unit) {
            continue;
        }
        let broke = match before.get(unit) {
            Some(was) => (now.failed() && !was.failed()) || now.nrestarts > was.nrestarts,
            // A unit the new generation introduced.
            None => now.failed(),
        };
        if broke {
            out.push(unit.clone());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(active: &str, sub: &str, nrestarts: u32) -> UnitState {
        UnitState {
            active: active.into(),
            sub: sub.into(),
            nrestarts,
        }
    }

    fn snap(units: &[(&str, UnitState)]) -> Snapshot {
        units
            .iter()
            .map(|(n, s)| (n.to_string(), s.clone()))
            .collect()
    }

    #[test]
    fn parses_list_units_json() {
        let json = r#"[{"unit":"nginx.service","load":"loaded","active":"active","sub":"running","description":"nginx"}]"#;
        let s = parse_list_units(json).unwrap();
        assert_eq!(s["nginx.service"], unit("active", "running", 0));
    }

    #[test]
    fn parses_show_stanzas() {
        let out = "Id=a.service\nActiveState=active\nNRestarts=0\n\nId=b.service\nNRestarts=3\n";
        let parsed = parse_show_restarts(out);
        assert_eq!(
            parsed,
            vec![("a.service".to_string(), 0), ("b.service".to_string(), 3)]
        );
    }

    #[test]
    fn a_stanza_without_restarts_does_not_steal_the_next_count() {
        let out = "Id=a.mount\n\nId=b.service\nNRestarts=3\n";
        assert_eq!(parse_show_restarts(out), vec![("b.service".to_string(), 3)]);
    }

    #[test]
    fn a_unit_that_broke_is_reported() {
        let before = snap(&[("nginx.service", unit("active", "running", 0))]);
        let after = snap(&[("nginx.service", unit("failed", "failed", 0))]);
        assert_eq!(newly_failed(&before, &after, &[]), vec!["nginx.service"]);
    }

    #[test]
    fn a_unit_already_broken_is_not_our_fault() {
        let before = snap(&[("backup.timer", unit("failed", "failed", 0))]);
        let after = snap(&[("backup.timer", unit("failed", "failed", 0))]);
        assert!(newly_failed(&before, &after, &[]).is_empty());
        assert_eq!(failed_units(&after), vec!["backup.timer"]);
    }

    #[test]
    fn a_crash_loop_is_caught_by_restart_count() {
        // never observed in `failed`, only ever activating (auto-restart)
        let before = snap(&[("app.service", unit("active", "running", 2))]);
        let after = snap(&[("app.service", unit("activating", "auto-restart", 7))]);
        assert_eq!(newly_failed(&before, &after, &[]), vec!["app.service"]);
    }

    #[test]
    fn a_new_unit_that_fails_counts() {
        let before = snap(&[]);
        let after = snap(&[("brand-new.service", unit("failed", "failed", 0))]);
        assert_eq!(
            newly_failed(&before, &after, &[]),
            vec!["brand-new.service"]
        );
    }

    #[test]
    fn a_removed_unit_is_not_a_failure() {
        let before = snap(&[("gone.service", unit("active", "running", 0))]);
        let after = snap(&[]);
        assert!(newly_failed(&before, &after, &[]).is_empty());
    }

    #[test]
    fn a_unit_that_recovered_is_not_a_failure() {
        let before = snap(&[("flaky.service", unit("failed", "failed", 0))]);
        let after = snap(&[("flaky.service", unit("active", "running", 0))]);
        assert!(newly_failed(&before, &after, &[]).is_empty());
    }

    /// Smoke test against the systemd actually running here.
    /// `cargo test -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn real_snapshot_is_readable() {
        let snap = snapshot().expect("snapshot");
        assert!(!snap.is_empty(), "expected some units");
        assert!(
            snap.keys().any(|k| k.ends_with(".service")),
            "expected services"
        );
        println!("units: {}", snap.len());
        println!("failed: {:?}", failed_units(&snap));
        let restarts: usize = snap.values().filter(|u| u.nrestarts > 0).count();
        println!("with restarts: {restarts}");
    }

    #[test]
    fn ignored_units_are_skipped() {
        let before = snap(&[("flaky.service", unit("active", "running", 0))]);
        let after = snap(&[("flaky.service", unit("failed", "failed", 1))]);
        let ignored = vec!["flaky.service".to_string()];
        assert!(newly_failed(&before, &after, &ignored).is_empty());
    }
}
