use super::{base, head, list, Action, PageContext, Row};
use crate::{ClientState, SiteState};
use axum::extract::{Extension, State};
use maud::{html, Markup};
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn home(
    State(state): State<Arc<RwLock<SiteState>>>,
    Extension(client_state): Extension<ClientState>,
) -> Markup {
    let state = state.read().await;

    let word_count = state.words.len();

    let things: Vec<Row> = state.things[0..5]
        .iter()
        .map(|thing| Row {
            date: thing.date.format("%Y-%m").to_string(),
            href: &thing.link,
            title: &thing.title,
            description: thing.description.as_deref(),
            external: true,
        })
        .collect();

    let words: Vec<Row> = state
        .words
        .iter()
        .take(5)
        .map(|post| Row {
            date: post.date.format("%Y").to_string(),
            href: &post.link,
            title: &post.title,
            description: Some(&post.description),
            external: false,
        })
        .collect();

    let content = html! {
        (head(
            &["home"],
            state.name.uppercase_full_str(),
            None,
            &[Action::new("GitHub", "https://github.com/haylinmoore"), Action::new("LinkedIn", "https://www.linkedin.com/in/haylinmoore/")],
        ))
        main {
            section class="panel span" {
                div class="id" {
                    div class="mark" {
                        img src="/assets/img/haylin.webp" alt="avatar";
                    }
                    table class="kv" {
                        tbody {
                            tr { th scope="row" { "Role" } td { "Software and network engineer" } }
                            tr { th scope="row" { "Location" } td { "West coast" } }
                            tr {
                                th scope="row" { "Now" }
                                td {
                                    a target="_blank" href="https://qumulo.com" { "Qumulo" } ". Building a petabyte-scale CDN with linearizable reads and writes at every edge."
                                }
                            }
                            tr {
                                th scope="row" { "Before" }
                                td {
                                    a target="_blank" href="https://arista.com" { "Arista Networks" } ". Network automation, large-scale digital-twin validation, and SMPTE 2110 routing."
                                }
                            }
                        }
                    }
                }
            }

            section class="panel col" {
                header { h2 class="label" { "Things I've made" } }
                (list(&things))
                div class="more" { a href="/things/" { "See more things" } }
            }

            section class="panel col" {
                header { h2 class="label" { "Words I've written" } }
                (list(&words))
                @if word_count > 5 {
                    div class="more" { a href="/posts/" { "See more words" } }
                }
            }

            section class="panel span" {
                div class="badges" {
                    @for badge in &state.badges {
                        @if let Some(link) = &badge.link {
                            a class="b88" href=(link) target="_blank" {
                                img loading="lazy" src=(badge.src) alt=(badge.alt) width="88" height="31";
                            }
                        } @else {
                            span class="b88" {
                                img loading="lazy" src=(badge.src) alt=(badge.alt) width="88" height="31";
                            }
                        }
                    }
                }
            }

            @if let Some(webring) = &state.webring {
                section class="panel span" {
                    header { h2 class="label" { "UMass Ring" } }
                    nav class="ring" {
                        a href=(webring.prev.url) {
                            span class="label" { "← Previous" }
                            b { (webring.prev.name) }
                        }
                        a class="hub" href="https://github.com/umaring/umaring" {
                            span class="label" { "Hub" }
                            b { "UMass Ring" }
                        }
                        a class="nx" href=(webring.next.url) {
                            span class="label" { "Next →" }
                            b { (webring.next.name) }
                        }
                    }
                }
            }
        }
    };

    base(
        PageContext {
            title: "Home".to_string(),
            canonical: "/".to_string(),
        },
        content,
        state.clone(),
        client_state,
    )
}
