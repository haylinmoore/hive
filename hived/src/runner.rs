use crate::config::Config;
use crate::health::{self, Snapshot};
use crate::state::{DeployError, Phase, State};
use crate::store::StateDir;
use std::io;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct Outcome {
    pub state: State,
    pub error: Option<DeployError>,
}

struct Run<'a> {
    cfg: &'a Config,
    dir: &'a StateDir,
    id: u64,
}

impl Run<'_> {
    fn log(&self, line: &str) {
        let _ = self.dir.append_log(self.id, line.as_bytes());
    }

    fn banner(&self, text: &str) {
        self.log(&format!("\n=== {text} ===\n"));
    }

    /// Run a command, streaming combined output into the deploy log and
    /// keeping the tail so a failure can explain itself without the log.
    fn exec(&self, phase: Phase, cmd: &mut Command, timeout: Duration) -> Result<(), DeployError> {
        let started = Instant::now();
        cmd.stdin(Stdio::null());

        let dir = self.dir.clone();
        let id = self.id;
        let sink: Sink = Arc::new(move |chunk: &[u8]| {
            let _ = dir.append_log(id, chunk);
        });

        let output = match run_with_timeout(cmd, timeout, sink) {
            Ok(o) => o,
            Err(e) => {
                let message = format!("{e}");
                self.log(&format!("{message}\n"));
                return Err(DeployError {
                    phase,
                    exit_code: None,
                    message,
                });
            }
        };

        if output.timed_out {
            let message = format!(
                "{} timed out after {}s",
                phase.label(),
                started.elapsed().as_secs()
            );
            return Err(DeployError {
                phase,
                exit_code: None,
                message,
            });
        }
        if !output.success {
            return Err(DeployError {
                phase,
                exit_code: output.code,
                message: tail(&output.combined, self.cfg.error_tail_bytes),
            });
        }
        Ok(())
    }

    fn capture(&self, cmd: &mut Command) -> io::Result<String> {
        let out = cmd.stdin(Stdio::null()).output()?;
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }
}

struct Captured {
    combined: Vec<u8>,
    success: bool,
    code: Option<i32>,
    timed_out: bool,
}

pub type Sink = Arc<dyn Fn(&[u8]) + Send + Sync>;

/// How much output we keep in memory for the error tail. A runaway build can
/// print far more than that, and the log file already has all of it.
const TAIL_BUFFER: usize = 256 * 1024;

/// Wait for a child, killing it if it overruns.
///
/// Output is handed to `sink` as it arrives rather than at the end, so a
/// forty minute build is readable while it runs instead of appearing all at
/// once when it finishes.
fn run_with_timeout(cmd: &mut Command, timeout: Duration, sink: Sink) -> io::Result<Captured> {
    use std::io::Read;
    use std::sync::Mutex;
    use std::thread;

    let mut child = cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let tail = Arc::new(Mutex::new(Vec::<u8>::new()));

    let pump = |mut src: Box<dyn Read + Send>, sink: Sink, tail: Arc<Mutex<Vec<u8>>>| {
        thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match src.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        let chunk = &buf[..n];
                        sink(chunk);
                        if let Ok(mut t) = tail.lock() {
                            t.extend_from_slice(chunk);
                            if t.len() > TAIL_BUFFER {
                                let drop_to = t.len() - TAIL_BUFFER;
                                t.drain(..drop_to);
                            }
                        }
                    }
                }
            }
        })
    };

    let a = pump(Box::new(stdout), sink.clone(), tail.clone());
    let b = pump(Box::new(stderr), sink, tail.clone());

    let deadline = Instant::now() + timeout;
    let mut timed_out = false;
    let status = loop {
        match child.try_wait()? {
            Some(s) => break s,
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                timed_out = true;
                break child.wait()?;
            }
            None => thread::sleep(Duration::from_millis(200)),
        }
    };

    let _ = a.join();
    let _ = b.join();

    let combined = tail.lock().map(|t| t.clone()).unwrap_or_default();
    Ok(Captured {
        combined,
        success: status.success() && !timed_out,
        code: status.code(),
        timed_out,
    })
}

fn tail(bytes: &[u8], max: usize) -> String {
    let text = String::from_utf8_lossy(bytes);
    let trimmed = text.trim_end();
    if trimmed.len() <= max {
        return trimmed.to_string();
    }
    let cut = trimmed.len() - max;
    let start = trimmed
        .char_indices()
        .find(|(i, _)| *i >= cut)
        .map(|(i, _)| i)
        .unwrap_or(0);
    let slice = &trimmed[start..];
    match slice.find('\n') {
        Some(nl) => format!("[...]\n{}", &slice[nl + 1..]),
        None => format!("[...]\n{slice}"),
    }
}

/// What the node is actually running, so a deploy made from a laptop with
/// uncommitted changes shows up as drift rather than as a stale line.
pub fn current_system() -> Option<String> {
    std::fs::read_link("/run/current-system")
        .ok()
        .map(|p| p.to_string_lossy().into_owned())
}

fn current_generation() -> Option<u64> {
    let link = std::fs::read_link("/nix/var/nix/profiles/system").ok()?;
    let name = link.file_name()?.to_str()?;
    name.strip_prefix("system-")?
        .strip_suffix("-link")?
        .parse()
        .ok()
}

/// Run one deployment to a terminal state.
///
/// Build and activate are separate phases on purpose. `nixos-rebuild switch`
/// conflates them, so a Nix typo and a broken activation script arrive as the
/// same failure. Split, a build failure is reported with the running system
/// provably untouched.
pub fn run(cfg: &Config, dir: &StateDir, id: u64) -> Outcome {
    let run = Run { cfg, dir, id };

    let rev = match dir
        .load()
        .ok()
        .and_then(|s| s.get(id).map(|d| d.rev.clone()))
    {
        Some(rev) => rev,
        None => {
            return Outcome {
                state: State::Failed,
                error: Some(DeployError {
                    phase: Phase::Fetch,
                    exit_code: None,
                    message: format!("no record for deployment {id}"),
                }),
            };
        }
    };

    run.log(&format!("hived deploying {rev} on {}\n", cfg.host));

    let mut before: Snapshot = Snapshot::new();

    for phase in Phase::all() {
        let started = Instant::now();
        set_phase(dir, id, phase);
        run.banner(phase.label());

        let result = match phase {
            Phase::Fetch => fetch(&run, &rev),
            Phase::Verify => verify(&run, &rev),
            Phase::Build => build(&run),
            Phase::Activate => {
                before = health::snapshot().unwrap_or_default();
                record_pre_failed(dir, id, &before);
                activate(&run)
            }
            Phase::Check => {
                let outcome = check(&run, &before);
                record_duration(dir, id, phase, started);
                return outcome;
            }
        };

        record_duration(dir, id, phase, started);

        if let Err(error) = result {
            run.log(&format!("\n{} failed\n", phase.label()));
            let state = if phase == Phase::Verify {
                State::Rejected
            } else {
                State::Failed
            };
            return Outcome {
                state,
                error: Some(error),
            };
        }
    }

    Outcome {
        state: State::Succeeded,
        error: None,
    }
}

fn set_phase(dir: &StateDir, id: u64, phase: Phase) {
    let _ = dir.update(|s| {
        if let Some(d) = s.get_mut(id) {
            d.phase = Some(phase);
        }
    });
}

fn record_duration(dir: &StateDir, id: u64, phase: Phase, started: Instant) {
    let secs = started.elapsed().as_secs_f64();
    let _ = dir.update(|s| {
        if let Some(d) = s.get_mut(id) {
            d.durations.insert(phase, secs);
        }
    });
}

fn record_pre_failed(dir: &StateDir, id: u64, before: &Snapshot) {
    let failed = health::failed_units(before);
    let _ = dir.update(|s| {
        if let Some(d) = s.get_mut(id) {
            d.preexisting_failed_units = failed.clone();
            d.previous_generation = current_generation();
        }
    });
}

fn git(repo: &Path, args: &[&str]) -> Command {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(repo).args(args);
    cmd
}

fn fetch(run: &Run, rev: &str) -> Result<(), DeployError> {
    let repo = &run.cfg.repo_dir;
    if !repo.join(".git").is_dir() {
        run.log("cloning\n");
        let mut cmd = Command::new("git");
        cmd.arg("clone").arg(&run.cfg.repo_url).arg(repo);
        run.exec(Phase::Fetch, &mut cmd, run.cfg.timeout_fetch)?;
    }

    run.exec(
        Phase::Fetch,
        &mut git(repo, &["fetch", "--prune", "origin", &run.cfg.branch]),
        run.cfg.timeout_fetch,
    )?;

    // The commit subject makes the history table readable at a glance.
    if let Ok(subject) = run.capture(&mut git(repo, &["log", "-1", "--format=%s", rev]))
        && !subject.is_empty()
    {
        let _ = run.dir.update(|s| {
            if let Some(d) = s.get_mut(run.id) {
                d.subject = Some(subject.clone());
            }
        });
    }
    Ok(())
}

/// The token proves who is asking. This proves what they are allowed to ask
/// for: a commit that is actually on the branch we deploy.
fn verify(run: &Run, rev: &str) -> Result<(), DeployError> {
    let repo = &run.cfg.repo_dir;
    let upstream = format!("origin/{}", run.cfg.branch);

    run.exec(
        Phase::Verify,
        &mut git(repo, &["merge-base", "--is-ancestor", rev, &upstream]),
        run.cfg.timeout_verify,
    )
    .map_err(|mut e| {
        if e.message.is_empty() {
            e.message = format!("{rev} is not an ancestor of {upstream}");
        }
        e
    })?;

    run.exec(
        Phase::Verify,
        &mut git(repo, &["checkout", "--detach", "--force", rev]),
        run.cfg.timeout_verify,
    )?;
    run.exec(
        Phase::Verify,
        &mut git(repo, &["clean", "-xffd", "-e", ".gcroots"]),
        run.cfg.timeout_verify,
    )
}

fn upgrade(run: &Run, action: &str) -> Command {
    let mut cmd = Command::new("./upgrade.sh");
    cmd.current_dir(&run.cfg.repo_dir)
        .arg(&run.cfg.host)
        .arg(action);
    cmd
}

fn build(run: &Run) -> Result<(), DeployError> {
    run.exec(
        Phase::Build,
        &mut upgrade(run, "build"),
        run.cfg.timeout_build,
    )
}

fn activate(run: &Run) -> Result<(), DeployError> {
    run.exec(
        Phase::Activate,
        &mut upgrade(run, "activate"),
        run.cfg.timeout_activate,
    )
}

/// Poll across a settle window rather than sampling once.
///
/// `switch-to-configuration` returns as soon as it has issued the restarts, so
/// a service that dies five seconds later is still `activating` at that moment
/// and a single sample would call it healthy.
/// Consecutive quiet samples required before calling the system settled.
const SETTLE_QUIET_SAMPLES: u32 = 3;

fn check(run: &Run, before: &Snapshot) -> Outcome {
    let started_check = Instant::now();
    // The settle window is a ceiling, not a fixed wait. Sitting out the whole
    // thing added half a minute to every deploy for no information, so instead
    // wait for systemd to run out of queued jobs and then take a few more
    // samples to catch anything that dies immediately.
    let deadline = Instant::now() + run.cfg.settle;
    let mut broken: Vec<String> = Vec::new();
    let mut quiet = 0;

    loop {
        if let Ok(after) = health::snapshot() {
            for unit in health::newly_failed(before, &after, &run.cfg.ignored_units) {
                // Our own runner is not part of the system we are judging.
                if unit.starts_with(&run.cfg.run_unit_prefix) {
                    continue;
                }
                if !broken.contains(&unit) {
                    broken.push(unit);
                }
            }
        }

        if health::system_state() == "starting" {
            quiet = 0;
        } else {
            quiet += 1;
        }

        if quiet >= SETTLE_QUIET_SAMPLES || Instant::now() >= deadline {
            break;
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    run.log(&format!(
        "\nsettled after {}s\n",
        started_check.elapsed().as_secs()
    ));

    let generation = current_generation();
    let toplevel = current_system();
    let broken_for_record = broken.clone();
    let _ = run.dir.update(|s| {
        if let Some(d) = s.get_mut(run.id) {
            d.new_failed_units = broken_for_record.clone();
            d.generation = generation;
            d.toplevel = toplevel.clone();
        }
    });

    if broken.is_empty() {
        run.log("\nno new failed units\n");
        Outcome {
            state: State::Succeeded,
            error: None,
        }
    } else {
        let message = format!("newly failed units: {}", broken.join(", "));
        run.log(&format!("\n{message}\n"));
        Outcome {
            state: State::Degraded,
            error: Some(DeployError {
                phase: Phase::Check,
                exit_code: None,
                message,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tail_keeps_the_end_which_is_where_nix_puts_the_error() {
        let text = b"line one\nline two\nerror: attribute 'sakura' missing\n";
        assert_eq!(tail(text, 4096), String::from_utf8_lossy(text).trim_end());

        let short = tail(text, 20);
        assert!(short.starts_with("[...]"));
        assert!(short.ends_with("missing"));
    }

    #[test]
    fn tail_does_not_split_a_multibyte_character() {
        let text = "héllo wörld ünicode tail".repeat(20);
        let cut = tail(text.as_bytes(), 32);
        assert!(cut.len() <= 64);
    }

    #[test]
    fn a_command_that_overruns_is_killed() {
        let mut cmd = Command::new("sleep");
        cmd.arg("30");
        let out = run_with_timeout(&mut cmd, Duration::from_millis(300), Arc::new(|_| {})).unwrap();
        assert!(out.timed_out);
        assert!(!out.success);
    }

    /// The old version buffered everything and wrote it after the process
    /// exited, so a long build showed nothing until it finished.
    #[test]
    fn output_reaches_the_sink_while_the_command_is_still_running() {
        use std::sync::Mutex;
        let seen = Arc::new(Mutex::new(Vec::<u8>::new()));
        let probe = seen.clone();
        let sink: Sink = Arc::new(move |c: &[u8]| {
            probe.lock().unwrap().extend_from_slice(c);
        });

        let mut cmd = Command::new("sh");
        cmd.args(["-c", "echo early; sleep 2; echo late"]);
        let handle = std::thread::spawn(move || {
            run_with_timeout(&mut cmd, Duration::from_secs(20), sink).unwrap()
        });

        // Half a second in, the first line must already be visible.
        std::thread::sleep(Duration::from_millis(600));
        let mid = String::from_utf8_lossy(&seen.lock().unwrap().clone()).to_string();
        assert!(mid.contains("early"), "nothing streamed yet: {mid:?}");
        assert!(!mid.contains("late"), "should not have the later line yet");

        let out = handle.join().unwrap();
        assert!(out.success);
        let all = String::from_utf8_lossy(&out.combined);
        assert!(all.contains("early") && all.contains("late"), "{all}");
    }

    #[test]
    fn output_is_captured_from_both_streams() {
        let mut cmd = Command::new("sh");
        cmd.args(["-c", "echo out; echo err >&2; exit 3"]);
        let out = run_with_timeout(&mut cmd, Duration::from_secs(10), Arc::new(|_| {})).unwrap();
        assert!(!out.success);
        assert_eq!(out.code, Some(3));
        let text = String::from_utf8_lossy(&out.combined);
        assert!(text.contains("out"), "stdout missing: {text}");
        assert!(text.contains("err"), "stderr missing: {text}");
    }
}
