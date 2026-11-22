use super::{base, head, list, Action, PageContext, Row};
use crate::{ClientState, SiteState};
use axum::extract::{Extension, State};
use maud::{html, Markup};
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn index(
    State(state): State<Arc<RwLock<SiteState>>>,
    Extension(client_state): Extension<ClientState>,
) -> Markup {
    let state = state.read().await;

    let rows: Vec<Row> = state
        .things
        .iter()
        .map(|thing| Row {
            date: thing.date.format("%Y-%m-%d").to_string(),
            href: &thing.link,
            title: &thing.title,
            description: thing.description.as_deref(),
            external: true,
        })
        .collect();

    let content = html! {
        (head(&["things"], "Things I've Made", None, &[Action::new("Home", "/"), Action::primary("Words", "/posts/")]))
        main {
            section class="panel span" {
                header { h2 class="label" { "Things I've made" } }
                (list(&rows))
            }
        }
    };

    base(
        PageContext {
            title: "Things".to_string(),
            canonical: "/things/".to_string(),
        },
        content,
        state.clone(),
        client_state,
    )
}
