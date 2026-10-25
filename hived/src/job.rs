use crate::config::Config;
use crate::runner::{self, Run};
use crate::state::{JobPhase, Phase};
use crate::store::{Log, StateDir};
use std::process::Command;

pub const KNOWN: [&str; 2] = ["dns", "arin"];

pub struct Outcome {
    pub ok: bool,
    pub changes: Option<u64>,
    pub error: Option<String>,
}

fn failed(error: impl Into<String>) -> Outcome {
    Outcome {
        ok: false,
        changes: None,
        error: Some(error.into()),
    }
}

pub fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' && chars.peek() == Some(&'[') {
            chars.next();
            for c in chars.by_ref() {
                if ('@'..='~').contains(&c) {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

pub fn count_changes(job: &str, output: &str) -> Option<u64> {
    let text = strip_ansi(output);
    match job {
        "dns" => {
            let numbered = |l: &&str| {
                l.strip_prefix('#')
                    .and_then(|r| r.split_once(':'))
                    .is_some_and(|(n, _)| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
            };
            let corrections = text.lines().map(str::trim_start).filter(numbered).count() as u64;
            let finished = text.lines().any(|l| l.trim_start().starts_with("Done."));
            finished.then_some(corrections)
        }
        "arin" => text.lines().rev().find_map(|l| {
            let (n, rest) = l.trim().split_once(' ')?;
            rest.starts_with("changes,").then(|| n.parse().ok())?
        }),
        _ => None,
    }
}

fn upgrade(run: &Run, job: &str, args: &[&str]) -> Command {
    let mut cmd = Command::new("./upgrade.sh");
    cmd.current_dir(&run.repo).arg(job).args(args);
    cmd
}

pub fn run(cfg: &Config, dir: &StateDir, id: u64) -> Outcome {
    let Some(record) = dir.load().ok().and_then(|s| s.job(id).cloned()) else {
        return failed(format!("no record for job {id}"));
    };
    if !KNOWN.contains(&record.job.as_str()) || !cfg.jobs.contains(&record.job) {
        return failed(format!("{} is not a job on {}", record.job, cfg.host));
    }

    let run = Run {
        cfg,
        dir,
        id,
        log: Log::Job(id),
        repo: cfg.state_dir.join("jobs-repo"),
    };
    let phase = match record.phase {
        JobPhase::Preview => "preview",
        JobPhase::Push => "push",
    };
    run.log(&format!(
        "hived {phase} of {} at {} on {}\n",
        record.job, record.rev, cfg.host
    ));

    run.banner("fetch");
    if let Err(e) = runner::fetch(&run, &record.rev) {
        return failed(e.message);
    }
    run.banner("verify");
    if let Err(e) = runner::verify(&run, &record.rev) {
        return failed(e.message);
    }

    run.banner(phase);
    match record.phase {
        JobPhase::Preview => {
            match run.exec_output(
                Phase::Build,
                &mut upgrade(&run, &record.job, &["preview"]),
                cfg.timeout_build,
            ) {
                Ok(out) => {
                    let changes = count_changes(&record.job, &String::from_utf8_lossy(&out));
                    match changes {
                        Some(n) => run.log(&format!("\n{n} changes\n")),
                        None => run.log("\ncould not tell how many changes, treating it as some\n"),
                    }
                    Outcome {
                        ok: true,
                        changes,
                        error: None,
                    }
                }
                Err(e) => failed(e.message),
            }
        }
        JobPhase::Push => match run.exec(
            Phase::Activate,
            &mut upgrade(&run, &record.job, &["push", "-y"]),
            cfg.timeout_build,
        ) {
            Ok(()) => Outcome {
                ok: true,
                changes: record.changes,
                error: None,
            },
            Err(e) => failed(e.message),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colour_codes_are_stripped() {
        assert_eq!(strip_ansi("\x1b[32m+ CREATE\x1b[0m x"), "+ CREATE x");
    }

    #[test]
    fn dns_changes_are_counted_from_the_numbered_corrections() {
        let out = "******************** Domain: hayl.in\n\x1b[32m#1: + CREATE dash.hive.hayl.in AAAA 200::1 ttl=60\x1b[0m\n#2: + CREATE dash.hive.hayl.in AAAA 200::2 ttl=60\n******************** Domain: aconite.systems\n#1: - DELETE ns2.aconite.systems A 23.144.156.0 ttl=3600\nDone. 3 corrections.\n";
        assert_eq!(count_changes("dns", out), Some(3));
    }

    #[test]
    fn a_dns_preview_with_nothing_to_do_is_zero() {
        let out = "******************** Domain: hayl.in\nDone. 0 corrections.\n";
        assert_eq!(count_changes("dns", out), Some(0));
    }

    #[test]
    fn a_dns_preview_that_never_finished_is_not_zero() {
        assert_eq!(count_changes("dns", "error: something broke\n"), None);
    }

    #[test]
    fn arin_changes_come_from_its_summary_line() {
        let out = "ROAs\n  x\n+ route 23.144.156.0/24 AS402213\n1 changes, 0 errors\n";
        assert_eq!(count_changes("arin", out), Some(1));
        assert_eq!(count_changes("arin", "0 changes, 0 errors\n"), Some(0));
        assert_eq!(count_changes("arin", "garbage\n"), None);
    }
}
