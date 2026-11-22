use super::{base, four04, head, list, Action, PageContext, Row};
use crate::{ClientState, SiteState};
use axum::extract::{Extension, Path, State};
use maud::{html, Markup, PreEscaped};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::words::get;

pub async fn index(
    State(state): State<Arc<RwLock<SiteState>>>,
    Extension(client_state): Extension<ClientState>,
) -> Markup {
    let state = state.read().await;

    let rows: Vec<Row> = state
        .words
        .iter()
        .map(|post| Row {
            date: post.date.format("%Y-%m-%d").to_string(),
            href: &post.link,
            title: &post.title,
            description: Some(&post.description),
            external: false,
        })
        .collect();

    let content = html! {
        (head(&["posts"], "Words I've Written", None, &[Action::new("Home", "/"), Action::primary("Things", "/things/")]))
        main {
            section class="panel span" {
                header { h2 class="label" { "Words I've written" } }
                (list(&rows))
            }
        }
    };

    base(
        PageContext {
            title: "Words".to_string(),
            canonical: "/posts/".to_string(),
        },
        content,
        state.clone(),
        client_state,
    )
}

pub async fn post(
    State(state): State<Arc<RwLock<SiteState>>>,
    Path(slug): Path<String>,
    Extension(client_state): Extension<ClientState>,
) -> Markup {
    let state = state.read().await;
    let words = state.words.clone();

    let post = get(words, &slug);

    if post.is_none() {
        return four04(slug, state.clone(), client_state);
    }

    let post = post.unwrap();
    let date = post.date.format("%Y-%m-%d").to_string();

    let content = html! {
        (head(
            &["posts", &date, state.name.uppercase_full_str()],
            &post.title,
            Some(&post.description),
            &[Action::new("Home", "/"), Action::primary("All posts", "/posts/")],
        ))
        main {
            article class="panel span pad prose-panel" {
                div class="prose" {
                    (PreEscaped(post.body))
                }
            }
        }
    };

    base(
        PageContext {
            title: post.title,
            canonical: post.link,
        },
        content,
        state.clone(),
        client_state,
    )
}
