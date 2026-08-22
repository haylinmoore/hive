use crate::config::AllowedClaims;
use jsonwebtoken::jwk::{AlgorithmParameters, JwkSet};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use serde::Deserialize;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// The subset of a GitHub Actions OIDC token we care about.
#[derive(Debug, Clone, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub repository: String,
    #[serde(default)]
    pub repository_owner_id: Option<String>,
    #[serde(rename = "ref", default)]
    pub reference: Option<String>,
    #[serde(default)]
    pub workflow_ref: Option<String>,
    #[serde(default)]
    pub sha: Option<String>,
    #[serde(default)]
    pub run_id: Option<String>,
}

#[derive(Debug)]
pub enum AuthError {
    /// Malformed or badly signed, the caller is not who they say.
    Invalid(String),
    /// Genuine token, but not one allowed to deploy here.
    Forbidden(String),
    /// We could not check, so we must not admit.
    Unavailable(String),
}

impl AuthError {
    pub fn status(&self) -> u16 {
        match self {
            AuthError::Invalid(_) => 401,
            AuthError::Forbidden(_) => 403,
            AuthError::Unavailable(_) => 503,
        }
    }

    pub fn message(&self) -> &str {
        match self {
            AuthError::Invalid(m) | AuthError::Forbidden(m) | AuthError::Unavailable(m) => m,
        }
    }
}

/// JWKS cache. Refreshed when a token names a key we have not seen, with a
/// floor on how often that can happen so an attacker cannot use unknown key
/// ids to make us hammer GitHub.
pub struct Jwks {
    url: String,
    cache_path: std::path::PathBuf,
    inner: Mutex<JwksInner>,
}

struct JwksInner {
    keys: Option<JwkSet>,
    last_fetch: Option<Instant>,
}

const MIN_REFRESH: Duration = Duration::from_secs(60);

impl Jwks {
    pub fn new(url: String, cache_path: std::path::PathBuf) -> Self {
        Jwks {
            url,
            cache_path,
            inner: Mutex::new(JwksInner {
                keys: None,
                last_fetch: None,
            }),
        }
    }

    fn load_cached(&self) -> Option<JwkSet> {
        let raw = std::fs::read_to_string(&self.cache_path).ok()?;
        serde_json::from_str(&raw).ok()
    }

    async fn fetch(&self) -> Result<JwkSet, AuthError> {
        let body = reqwest::Client::new()
            .get(&self.url)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| AuthError::Unavailable(format!("jwks fetch: {e}")))?
            .text()
            .await
            .map_err(|e| AuthError::Unavailable(format!("jwks read: {e}")))?;

        let set: JwkSet = serde_json::from_str(&body)
            .map_err(|e| AuthError::Unavailable(format!("jwks parse: {e}")))?;
        // Survive a cold start during a network blip.
        let _ = std::fs::write(&self.cache_path, &body);
        Ok(set)
    }

    async fn key_for(&self, kid: &str) -> Result<DecodingKey, AuthError> {
        let (cached, may_refresh) = {
            let mut inner = self.inner.lock().unwrap();
            if inner.keys.is_none() {
                inner.keys = self.load_cached();
            }
            let stale = inner
                .last_fetch
                .map(|t| t.elapsed() >= MIN_REFRESH)
                .unwrap_or(true);
            (inner.keys.clone(), stale)
        };

        if let Some(set) = &cached
            && let Some(jwk) = set.find(kid)
        {
            return to_key(jwk);
        }

        if !may_refresh {
            return Err(AuthError::Invalid(format!("unknown key id {kid}")));
        }

        let set = self.fetch().await?;
        {
            let mut inner = self.inner.lock().unwrap();
            inner.keys = Some(set.clone());
            inner.last_fetch = Some(Instant::now());
        }

        match set.find(kid) {
            Some(jwk) => to_key(jwk),
            None => Err(AuthError::Invalid(format!("unknown key id {kid}"))),
        }
    }

    /// Verify signature, standard claims, then the allowlist.
    pub async fn verify(
        &self,
        token: &str,
        allowed: &AllowedClaims,
        issuer: &str,
    ) -> Result<Claims, AuthError> {
        let header =
            decode_header(token).map_err(|e| AuthError::Invalid(format!("bad header: {e}")))?;
        let kid = header
            .kid
            .ok_or_else(|| AuthError::Invalid("no key id".into()))?;
        let key = self.key_for(&kid).await?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&[&allowed.audience]);
        validation.set_issuer(&[issuer]);
        validation.leeway = 60;

        let data = decode::<Claims>(token, &key, &validation)
            .map_err(|e| AuthError::Invalid(format!("bad token: {e}")))?;

        check_allowed(&data.claims, allowed)?;
        Ok(data.claims)
    }
}

fn to_key(jwk: &jsonwebtoken::jwk::Jwk) -> Result<DecodingKey, AuthError> {
    match &jwk.algorithm {
        AlgorithmParameters::RSA(rsa) => DecodingKey::from_rsa_components(&rsa.n, &rsa.e)
            .map_err(|e| AuthError::Invalid(format!("bad key: {e}"))),
        _ => Err(AuthError::Invalid("unsupported key type".into())),
    }
}

/// Who is allowed to deploy, independent of whether the token is genuine.
pub fn check_allowed(claims: &Claims, allowed: &AllowedClaims) -> Result<(), AuthError> {
    if claims.repository != allowed.repository {
        return Err(AuthError::Forbidden(format!(
            "repository {} is not allowed",
            claims.repository
        )));
    }
    if let Some(want) = &allowed.repository_owner_id
        && claims.repository_owner_id.as_deref() != Some(want.as_str())
    {
        return Err(AuthError::Forbidden("owner id mismatch".into()));
    }
    if claims.reference.as_deref() != Some(allowed.reference.as_str()) {
        return Err(AuthError::Forbidden(format!(
            "ref {} is not allowed",
            claims.reference.clone().unwrap_or_default()
        )));
    }
    if let Some(want) = &allowed.workflow_ref
        && claims.workflow_ref.as_deref() != Some(want.as_str())
    {
        return Err(AuthError::Forbidden("workflow mismatch".into()));
    }
    Ok(())
}

/// A token may only deploy the commit whose push minted it.
///
/// This is what makes a stolen token near worthless: within its few minute
/// life it can only re-deploy a commit that is already on main, which is a
/// no-op, rather than deploy anything of the attacker's choosing.
pub fn check_rev_matches(claims: &Claims, rev: &str) -> Result<(), AuthError> {
    match claims.sha.as_deref() {
        Some(sha) if sha.eq_ignore_ascii_case(rev) => Ok(()),
        Some(sha) => Err(AuthError::Forbidden(format!(
            "token was minted for {sha}, not {rev}"
        ))),
        None => Err(AuthError::Forbidden("token carries no sha".into())),
    }
}

pub fn is_hex_sha(rev: &str) -> bool {
    rev.len() == 40 && rev.chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allowed() -> AllowedClaims {
        AllowedClaims {
            audience: "hived".into(),
            repository: "haylinmoore/hive".into(),
            repository_owner_id: Some("12345".into()),
            reference: "refs/heads/main".into(),
            workflow_ref: Some(
                "haylinmoore/hive/.github/workflows/deploy.yml@refs/heads/main".into(),
            ),
        }
    }

    fn claims() -> Claims {
        Claims {
            sub: "repo:haylinmoore/hive:ref:refs/heads/main".into(),
            repository: "haylinmoore/hive".into(),
            repository_owner_id: Some("12345".into()),
            reference: Some("refs/heads/main".into()),
            workflow_ref: Some(
                "haylinmoore/hive/.github/workflows/deploy.yml@refs/heads/main".into(),
            ),
            sha: Some("a".repeat(40)),
            run_id: Some("1".into()),
        }
    }

    #[test]
    fn a_matching_token_is_allowed() {
        assert!(check_allowed(&claims(), &allowed()).is_ok());
    }

    #[test]
    fn another_repository_is_refused() {
        let mut c = claims();
        c.repository = "someone/else".into();
        assert_eq!(check_allowed(&c, &allowed()).unwrap_err().status(), 403);
    }

    #[test]
    fn a_renamed_owner_is_caught_by_the_numeric_id() {
        let mut c = claims();
        c.repository_owner_id = Some("999".into());
        assert!(check_allowed(&c, &allowed()).is_err());
    }

    #[test]
    fn a_branch_other_than_main_is_refused() {
        let mut c = claims();
        c.reference = Some("refs/heads/wip".into());
        assert!(check_allowed(&c, &allowed()).is_err());
    }

    #[test]
    fn a_token_from_another_workflow_is_refused() {
        let mut c = claims();
        c.workflow_ref =
            Some("haylinmoore/hive/.github/workflows/other.yml@refs/heads/main".into());
        assert!(check_allowed(&c, &allowed()).is_err());
    }

    #[test]
    fn a_token_can_only_deploy_its_own_commit() {
        let c = claims();
        assert!(check_rev_matches(&c, &"a".repeat(40)).is_ok());
        assert!(
            check_rev_matches(&c, &"A".repeat(40)).is_ok(),
            "case insensitive"
        );
        assert!(check_rev_matches(&c, &"b".repeat(40)).is_err());
    }

    #[test]
    fn a_token_without_a_sha_cannot_deploy() {
        let mut c = claims();
        c.sha = None;
        assert!(check_rev_matches(&c, &"a".repeat(40)).is_err());
    }

    #[test]
    fn revs_must_look_like_revs() {
        assert!(is_hex_sha(&"a".repeat(40)));
        assert!(!is_hex_sha(&"a".repeat(39)));
        assert!(!is_hex_sha("main"));
        assert!(!is_hex_sha("../../etc/passwd"));
        assert!(!is_hex_sha(&"g".repeat(40)));
    }

    #[test]
    fn unverifiable_is_not_the_same_as_refused() {
        assert_eq!(AuthError::Invalid("x".into()).status(), 401);
        assert_eq!(AuthError::Forbidden("x".into()).status(), 403);
        assert_eq!(AuthError::Unavailable("x".into()).status(), 503);
    }
}
