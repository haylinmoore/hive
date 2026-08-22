mod api;
mod auth;
mod config;
mod health;
mod page;
mod runner;
mod state;
mod store;

use crate::api::{App, is_unit_active, now, start_unit};
use crate::config::Config;
use crate::state::State;
use crate::store::StateDir;
use std::sync::Arc;
use std::time::Duration;

fn usage() -> ! {
    eprintln!("usage: hived serve | hived run <id>");
    std::process::exit(2)
}

fn main() {
    let cfg = Config::from_env();
    let dir = match StateDir::new(&cfg.state_dir) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("cannot open {}: {e}", cfg.state_dir.display());
            std::process::exit(1)
        }
    };

    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("serve") => serve(cfg, dir),
        Some("run") => {
            let id = args.next().and_then(|v| v.parse::<u64>().ok());
            match id {
                Some(id) => run_one(cfg, dir, id),
                None => usage(),
            }
        }
        _ => usage(),
    }
}

/// The privileged half. Started by the listener through a polkit rule scoped
/// to this one unit template, so a compromised listener can only ever ask for
/// a deploy of a commit already on the branch.
fn run_one(cfg: Config, dir: StateDir, id: u64) -> ! {
    let outcome = runner::run(&cfg, &dir, id);
    let error = outcome.error.clone();
    let state = outcome.state;

    let log_bytes = dir.log_size(id);
    let _ = dir.update(|s| {
        if let Some(d) = s.get_mut(id) {
            d.error = error.clone();
            d.log_bytes = log_bytes;
        }
        s.finish(id, state, now());
    });

    let code = match state {
        State::Succeeded | State::Degraded => 0,
        _ => 1,
    };
    std::process::exit(code)
}

fn serve(cfg: Config, dir: StateDir) -> ! {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    let jwks = auth::Jwks::new(cfg.jwks_url.clone(), cfg.state_dir.join("jwks.json"));
    let app = Arc::new(App { cfg, dir, jwks });

    runtime.block_on(async move {
        supervise_once(&app);
        let bg = app.clone();
        tokio::spawn(async move {
            loop {
                supervise_once(&bg);
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        });

        let listener = tokio::net::TcpListener::bind(&app.cfg.bind)
            .await
            .unwrap_or_else(|e| panic!("cannot bind {}: {e}", app.cfg.bind));
        eprintln!("hived listening on {}", app.cfg.bind);
        axum::serve(listener, api::router(app.clone()))
            .await
            .expect("serve");
    });
    std::process::exit(0)
}

/// Reconcile, retire, promote.
///
/// A record still marked running whose unit is gone was killed by a reboot,
/// and without this pass that phantom blocks admission forever. Promotion
/// lives here rather than in the runner because the runner may not survive to
/// do it.
fn supervise_once(app: &App) {
    let cfg = &app.cfg;

    let interrupted = app
        .dir
        .update(|s| s.reconcile(now(), |id| is_unit_active(cfg, id)))
        .unwrap_or_default();
    for id in interrupted {
        eprintln!("deployment {id} was interrupted");
    }

    if let Ok(Some(id)) = app
        .dir
        .update(|s| s.expire_queued(cfg.queued_max_age, now()))
    {
        eprintln!("deployment {id} waited too long and was superseded");
    }

    if let Ok(Some(id)) = app.dir.update(|s| s.take_queued(now()))
        && let Err(e) = start_unit(cfg, id)
    {
        eprintln!("could not start runner for {id}: {e}");
        let _ = app.dir.update(|s| s.finish(id, State::Failed, now()));
    }

    reap(app);
}

/// Records are cheap and logs are not, so they are kept to different depths.
/// The error tail stored on each record means a reaped deploy still explains
/// itself.
fn reap(app: &App) {
    let Ok(store) = app.dir.load() else { return };
    let keep: Vec<u64> = store
        .deployments
        .iter()
        .take(app.cfg.max_logs)
        .map(|d| d.id)
        .collect();

    if let Ok(reaped) = app.dir.reap_logs(&keep)
        && !reaped.is_empty()
    {
        let _ = app.dir.update(|s| {
            for id in &reaped {
                if let Some(d) = s.get_mut(*id) {
                    d.log_reaped = true;
                }
            }
            s.prune(app.cfg.max_records);
        });
    }
}
