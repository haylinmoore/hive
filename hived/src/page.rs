use crate::api::Shared;
use axum::extract::State as AxumState;
use axum::http::{StatusCode, header};
use axum::response::{Html, IntoResponse, Response};

/// The console is the same file GitHub Pages publishes, served from the same
/// derivation. The only difference is the config injected below: a node names
/// only itself and an empty base, so the page talks to the node serving it
/// instead of to every node in the hive.
const PLACEHOLDER: &str = "<!--HIVED_CONFIG-->";

fn local_config(host: &str) -> String {
    // serde_json does not escape '/', so a value containing "</script>" would
    // close the tag it sits in. Escaping it keeps the value inside the string.
    let name = serde_json::to_string(host)
        .unwrap_or_else(|_| "\"\"".into())
        .replace("</", "<\\/");
    format!("<script>window.HIVED_CONFIG={{hosts:[{{name:{name},base:\"\"}}]}}</script>")
}

async fn index(AxumState(app): AxumState<Shared>) -> Response {
    let Some(path) = &app.cfg.dashboard else {
        return (
            StatusCode::NOT_FOUND,
            "no dashboard configured, set HIVED_DASHBOARD",
        )
            .into_response();
    };

    match std::fs::read_to_string(path) {
        Ok(body) => {
            Html(body.replacen(PLACEHOLDER, &local_config(&app.cfg.host), 1)).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("cannot read {}: {e}", path.display()),
        )
            .into_response(),
    }
}

async fn robots() -> Response {
    (
        [(header::CONTENT_TYPE, "text/plain")],
        "User-agent: *\nDisallow: /\n",
    )
        .into_response()
}

pub fn routes() -> axum::Router<Shared> {
    axum::Router::new()
        .route("/", axum::routing::get(index))
        .route("/robots.txt", axum::routing::get(robots))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_injection_replaces_the_placeholder() {
        let page = format!("<body>{PLACEHOLDER}</body>");
        let out = page.replacen(PLACEHOLDER, &local_config("zoe"), 1);
        assert!(out.contains(r#"name:"zoe""#), "{out}");
        assert!(out.contains(r#"base:"""#), "{out}");
        assert!(!out.contains(PLACEHOLDER));
    }

    #[test]
    fn a_hostile_hostname_cannot_break_out_of_the_script() {
        let injected = local_config("zoe\"</script><script>alert(1)");
        assert!(!injected.contains("</script><script>alert"), "{injected}");
    }
}
