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

/// Claims a GitHub OIDC token must carry to be allowed to deploy.
#[derive(Debug, Clone)]
pub struct AllowedClaims {
    pub audience: String,
    pub repository: String,
    pub repository_owner_id: Option<String>,
    pub reference: String,
    pub workflow_ref: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub bind: String,
    pub state_dir: PathBuf,
    pub repo_dir: PathBuf,
    pub repo_url: String,
    pub branch: String,
    pub run_unit_prefix: String,
    pub claims: AllowedClaims,
    pub jwks_url: String,
    pub issuer: String,

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

        Config {
            host: env_or("HIVED_HOST", "unknown"),
            bind: env_or("HIVED_BIND", "127.0.0.1:15680"),
            state_dir,
            repo_dir,
            repo_url: env_or("HIVED_REPO_URL", "https://github.com/haylinmoore/hive.git"),
            branch: env_or("HIVED_BRANCH", "main"),
            run_unit_prefix: env_or("HIVED_RUN_UNIT_PREFIX", "hived-run@"),

            claims: AllowedClaims {
                audience: env_or("HIVED_AUDIENCE", "hived"),
                repository: env_or("HIVED_ALLOWED_REPOSITORY", "haylinmoore/hive"),
                repository_owner_id: env("HIVED_ALLOWED_OWNER_ID"),
                reference: env_or("HIVED_ALLOWED_REF", "refs/heads/main"),
                workflow_ref: env("HIVED_ALLOWED_WORKFLOW_REF"),
            },
            jwks_url: env_or(
                "HIVED_JWKS_URL",
                "https://token.actions.githubusercontent.com/.well-known/jwks",
            ),
            issuer: env_or(
                "HIVED_ISSUER",
                "https://token.actions.githubusercontent.com",
            ),

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
}
