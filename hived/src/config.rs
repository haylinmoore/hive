use std::net::Ipv6Addr;
use std::path::PathBuf;
use std::time::Duration;

fn env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.is_empty())
}

fn env_or(key: &str, default: &str) -> String {
    env(key).unwrap_or_else(|| default.to_string())
}

fn env_secs(key: &str, default: u64) -> Duration {
    Duration::from_secs(env(key).and_then(|v| v.parse().ok()).unwrap_or(default))
}

fn env_list(key: &str) -> Vec<String> {
    env(key)
        .map(|v| {
            v.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

fn env_addrs(key: &str) -> Vec<Ipv6Addr> {
    env_list(key)
        .iter()
        .filter_map(|a| a.parse().ok())
        .collect()
}

/// A mesh member as the registry in nixos/mesh/hosts.nix describes it.
#[derive(Debug, Clone)]
pub struct Node {
    pub name: String,
    pub address: Ipv6Addr,
    pub key: String,
}

/// `name=address=key`, comma separated.
fn parse_nodes(raw: &[String]) -> Vec<Node> {
    raw.iter()
        .filter_map(|entry| {
            let mut parts = entry.splitn(3, '=');
            Some(Node {
                name: parts.next()?.to_string(),
                address: parts.next()?.parse().ok()?,
                key: parts.next()?.to_string(),
            })
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub hub: bool,
    pub auto_deploy: bool,

    pub agent_bind: String,
    pub agent_port: u16,
    pub public_bind: String,
    pub dash_bind: String,

    pub nodes: Vec<Node>,
    pub hubs: Vec<Ipv6Addr>,
    pub operators: Vec<Ipv6Addr>,
    pub ygg_socket: PathBuf,

    pub state_dir: PathBuf,
    pub repo_dir: PathBuf,
    pub repo_url: String,
    pub branch: String,
    pub run_unit_prefix: String,
    pub job_unit_prefix: String,
    pub jobs: Vec<String>,

    pub poll_every: Duration,
    pub main_every: Duration,

    pub ignored_units: Vec<String>,
    pub settle: Duration,
    pub queued_max_age: i64,

    pub timeout_fetch: Duration,
    pub timeout_verify: Duration,
    pub timeout_build: Duration,
    pub timeout_activate: Duration,

    pub max_records: usize,
    pub max_logs: usize,
    pub error_tail_bytes: usize,
}

impl Config {
    pub fn from_env() -> Config {
        let state_dir = PathBuf::from(env_or("HIVED_STATE_DIR", "/var/lib/hived"));
        let repo_dir = env("HIVED_REPO_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| state_dir.join("repo"));
        let agent_port = env("HIVED_AGENT_PORT")
            .and_then(|v| v.parse().ok())
            .unwrap_or(15680);

        Config {
            host: env_or("HIVED_HOST", "unknown"),
            hub: env("HIVED_HUB").as_deref() == Some("1"),
            auto_deploy: env("HIVED_AUTO_DEPLOY").as_deref() != Some("0"),

            agent_bind: env_or("HIVED_AGENT_BIND", &format!("[::1]:{agent_port}")),
            agent_port,
            public_bind: env_or("HIVED_PUBLIC_BIND", "127.0.0.1:15681"),
            dash_bind: env_or("HIVED_DASH_BIND", "[::1]:15682"),

            nodes: parse_nodes(&env_list("HIVED_NODES")),
            hubs: env_addrs("HIVED_HUBS"),
            operators: env_addrs("HIVED_OPERATORS"),
            ygg_socket: PathBuf::from(env_or("HIVED_YGG_SOCKET", "/run/yggdrasil/yggdrasil.sock")),

            state_dir,
            repo_dir,
            repo_url: env_or("HIVED_REPO_URL", "https://github.com/haylinmoore/hive.git"),
            branch: env_or("HIVED_BRANCH", "main"),
            run_unit_prefix: env_or("HIVED_RUN_UNIT_PREFIX", "hived-run@"),
            job_unit_prefix: env_or("HIVED_JOB_UNIT_PREFIX", "hived-job@"),
            jobs: env_list("HIVED_JOBS"),

            poll_every: env_secs("HIVED_POLL_SECS", 5),
            main_every: env_secs("HIVED_MAIN_SECS", 300),

            ignored_units: env_list("HIVED_IGNORED_UNITS"),
            settle: env_secs("HIVED_SETTLE_SECS", 30),
            queued_max_age: env("HIVED_QUEUED_MAX_AGE_SECS")
                .and_then(|v| v.parse().ok())
                .unwrap_or(1800),

            timeout_fetch: env_secs("HIVED_TIMEOUT_FETCH_SECS", 120),
            timeout_verify: env_secs("HIVED_TIMEOUT_VERIFY_SECS", 30),
            timeout_build: env_secs("HIVED_TIMEOUT_BUILD_SECS", 2700),
            timeout_activate: env_secs("HIVED_TIMEOUT_ACTIVATE_SECS", 600),

            max_records: env("HIVED_MAX_RECORDS")
                .and_then(|v| v.parse().ok())
                .unwrap_or(500),
            max_logs: env("HIVED_MAX_LOGS")
                .and_then(|v| v.parse().ok())
                .unwrap_or(50),
            error_tail_bytes: env("HIVED_ERROR_TAIL_BYTES")
                .and_then(|v| v.parse().ok())
                .unwrap_or(4096),
        }
    }

    pub fn run_unit(&self, id: u64) -> String {
        format!("{}{}.service", self.run_unit_prefix, id)
    }

    pub fn job_unit(&self, id: u64) -> String {
        format!("{}{}.service", self.job_unit_prefix, id)
    }

    pub fn name_for_key(&self, key: &str) -> Option<&str> {
        self.nodes
            .iter()
            .find(|n| n.key == key)
            .map(|n| n.name.as_str())
    }

    /// Commit links on the dashboard point at the same place we clone from.
    pub fn commit_url(&self, rev: &str) -> String {
        format!("{}/commit/{rev}", self.repo_url.trim_end_matches(".git"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nodes_parse_from_the_registry_string() {
        let raw = vec![
            "athena=200:cd3c:bbf8:d20a:93db:c98a:b39a:a955=9961".to_string(),
            "broken".to_string(),
        ];
        let nodes = parse_nodes(&raw);
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].name, "athena");
        assert_eq!(nodes[0].key, "9961");
        assert_eq!(
            nodes[0].address,
            "200:cd3c:bbf8:d20a:93db:c98a:b39a:a955"
                .parse::<Ipv6Addr>()
                .unwrap()
        );
    }
}
