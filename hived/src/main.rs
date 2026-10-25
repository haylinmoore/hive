mod agent;
mod certs;
mod config;
mod dash;
mod health;
mod hub;
mod job;
mod runner;
mod state;
mod store;
mod ygg;

use crate::agent::{App, is_unit_active, now, start, start_unit, unit_active};
use crate::config::Config;
use crate::state::State;
use crate::store::StateDir;
use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

const RECONCILE_GRACE_SECS: i64 = 30;

fn usage() -> ! {
    eprintln!("usage: hived serve | hived run <id> | hived job <id> | hived certs");
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
        Some("certs") => certs_once(cfg),
        Some("job") => {
            let id = args.next().and_then(|v| v.parse::<u64>().ok());
            match id {
                Some(id) => job_one(cfg, dir, id),
                None => usage(),
            }
        }
        _ => usage(),
    }
}

fn run_one(cfg: Config, dir: StateDir, id: u64) -> ! {
    let outcome = runner::run(&cfg, &dir, id);
    let error = outcome.error.clone();
    let state = outcome.state;

    let log_bytes = dir.log_size(id);
    let recorded = dir
        .update(|s| {
            let existed = s.get(id).is_some();
            let mut rev = None;
            if let Some(d) = s.get_mut(id) {
                d.error = error.clone();
                d.log_bytes = log_bytes;
                rev = Some(d.rev.clone());
            }
            if state == State::Unchanged && s.available == rev {
                s.available = None;
            }
            s.finish(id, state, now());
            existed
        })
        .unwrap_or(false);

    // A non-zero exit would leave the runner in systemctl --failed for good.
    if recorded {
        std::process::exit(0)
    }
    eprintln!("no record for deployment {id}");
    std::process::exit(1)
}

async fn listen(addr: &str, router: Router) -> tokio::task::JoinHandle<()> {
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("cannot bind {addr}: {e}");
            std::process::exit(1)
        }
    };
    eprintln!("listening on {addr}");
    tokio::spawn(async move {
        let service = router.into_make_service_with_connect_info::<SocketAddr>();
        if let Err(e) = axum::serve(listener, service).await {
            eprintln!("server failed: {e}");
            std::process::exit(1)
        }
    })
}

fn serve(cfg: Config, dir: StateDir) -> ! {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    let app = Arc::new(App { cfg, dir });

    runtime.block_on(async move {
        supervise_once(&app);
        let bg = app.clone();
        tokio::spawn(async move {
            loop {
                supervise_once(&bg);
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        });

        let mut servers = vec![listen(&app.cfg.agent_bind, agent::router(app.clone())).await];
        if app.cfg.hub {
            let hub = hub::Hub::new(app.clone());
            hub::spawn(hub.clone());
            servers.push(listen(&app.cfg.public_bind, hub::public_router(hub.clone())).await);
            servers.push(listen(&app.cfg.dash_bind, hub::dash_router(hub)).await);
        }
        for s in servers {
            let _ = s.await;
        }
    });
    std::process::exit(0)
}

fn supervise_once(app: &App) {
    let cfg = &app.cfg;

    let interrupted = app
        .dir
        .update(|s| s.reconcile(now(), RECONCILE_GRACE_SECS, |id| is_unit_active(cfg, id)))
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

    supervise_jobs(app);
    reap(app);
}

fn supervise_jobs(app: &App) {
    let cfg = &app.cfg;
    if cfg.jobs.is_empty() {
        return;
    }

    let lost = app
        .dir
        .update(|s| {
            s.reconcile_jobs(now(), RECONCILE_GRACE_SECS, |id| {
                unit_active(&cfg.job_unit(id))
            })
        })
        .unwrap_or_default();
    for id in lost {
        eprintln!("job {id} was interrupted");
    }

    if let Ok(Some(id)) = app.dir.update(|s| s.take_job(now()))
        && let Err(e) = start(&cfg.job_unit(id))
    {
        eprintln!("could not start job {id}: {e}");
        let _ = app
            .dir
            .update(|s| s.finish_job(id, false, None, Some(e.to_string()), now()));
    }

    let _ = app.dir.update(|s| s.prune_jobs(200));
}

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

fn job_one(cfg: Config, dir: StateDir, id: u64) -> ! {
    let outcome = job::run(&cfg, &dir, id);
    let recorded = dir
        .update(|s| {
            let existed = s.job(id).is_some();
            s.finish_job(
                id,
                outcome.ok,
                outcome.changes,
                outcome.error.clone(),
                now(),
            );
            existed
        })
        .unwrap_or(false);
    if recorded {
        std::process::exit(0)
    }
    eprintln!("no record for job {id}");
    std::process::exit(1)
}

fn certs_once(cfg: Config) -> ! {
    let path = cfg.state_dir.join("certs.json");
    let previous = std::fs::read(&path)
        .ok()
        .and_then(|raw| serde_json::from_slice(&raw).ok())
        .unwrap_or_default();
    let file = certs::collect(&cfg, &previous, now());
    let tmp = cfg.state_dir.join("certs.json.tmp");
    let written = serde_json::to_vec_pretty(&file)
        .map_err(std::io::Error::other)
        .and_then(|body| std::fs::write(&tmp, body))
        .and_then(|()| std::fs::rename(&tmp, &path));
    match written {
        Ok(()) => std::process::exit(0),
        Err(e) => {
            eprintln!("cannot write {}: {e}", path.display());
            std::process::exit(1)
        }
    }
}
