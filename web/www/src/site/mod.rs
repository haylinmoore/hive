use crate::{ClientState, SiteState};
use maud::{html, Markup, PreEscaped};
pub mod error404;
pub mod home;
pub mod nginx;
pub mod things;
pub mod words;

pub struct PageContext {
    title: String,
    canonical: String,
}

pub struct Action {
    label: &'static str,
    href: String,
    primary: bool,
}

impl Action {
    pub fn new(label: &'static str, href: impl Into<String>) -> Self {
        Action {
            label,
            href: href.into(),
            primary: false,
        }
    }

    pub fn primary(label: &'static str, href: impl Into<String>) -> Self {
        Action {
            primary: true,
            ..Action::new(label, href)
        }
    }
}

pub fn head(crumbs: &[&str], title: &str, sub: Option<&str>, actions: &[Action]) -> Markup {
    html! {
        div class="head" {
            div {
                div class="crumbs" {
                    a href="/" { "hayl.in" }
                    @for crumb in crumbs { " / " (crumb) }
                }
                h1 { (title) }
                @if let Some(sub) = sub {
                    p class="sub" { (sub) }
                }
            }
            @if !actions.is_empty() {
                div class="actions" {
                    @for action in actions {
                        a class=(if action.primary { "btn primary" } else { "btn" }) href=(action.href) { (action.label) }
                    }
                }
            }
        }
    }
}

pub fn base(
    context: PageContext,
    content: Markup,
    state: SiteState,
    client: ClientState,
) -> Markup {
    let title = format!("{} | {}", context.title, state.name.uppercase_full_str());

    html! {
        (maud::DOCTYPE)
            html lang="en" {
                head {
                    meta charset="utf-8";
                    meta name="viewport" content="width=device-width, initial-scale=1";
                    link rel="preload" href="/assets/fonts/plex-sans-400.woff2" as="font" type="font/woff2" crossorigin;
                    link rel="preload" href="/assets/fonts/plex-mono-500.woff2" as="font" type="font/woff2" crossorigin;
                    link rel="stylesheet" href="/assets/css/main.css";
                    link rel="alternate" title=(format!("{}'s Blog", state.name.uppercase_str())) type="application/rss+xml" href="/feed.xml";
                    link rel="canonical" href=(format!("https://hayl.in{}", context.canonical));
                    link rel="icon" type="image/png" href="/assets/img/favicon.png";

                    title { (title) };
                    meta name="author" content=(state.name.uppercase_full_str());

                    meta name="theme-color" content="#e5dae2";

                    meta property="og:type" content="website";
                    meta property="og:title" content=(title);
                    meta property="og:theme-color" content="#e5dae2";
                }

                body colorscheme=(client.theme) {
                    (content)
                    div class="status" {
                        span class="tag" { "AGPLv3" }
                        span {
                            a target="_blank" href="https://github.com/haylinmoore/hive/tree/main/web/www/" { "Source" }
                            " · "
                            a target="_blank" href="https://github.com/haylinmoore/hive/blob/main/web/www/COPYING" { "License" }
                        }
                        div class="themes themecolors" {
                            @for theme in ["blue", "pink", "white", "purple", "red"] {
                                button type="button" theme=(theme) aria-label=(format!("{} theme", theme)) aria-pressed=(if client.theme == theme { "true" } else { "false" }) style=(format!("background-color: var(--scheme-{})", theme)) {}
                            }
                        }
                    }
                    script type="text/javascript" {
                        (PreEscaped("
                        [...document.getElementsByClassName('themecolors')[0].children].forEach((c)=>c.onclick=()=>{
                            theme = c.getAttribute('theme');
                            document.body.setAttribute('colorscheme', theme);
                            document.cookie = `colorscheme=${theme}; expires=${(new Date(Date.now()+ 86400*365*1000)).toUTCString()}; path=/`;
                            [...c.parentNode.children].forEach((o)=>o.setAttribute('aria-pressed', o === c));
                        })
                        "))
                    }
                }
            }
    }
}

pub fn four04(path: String, state: SiteState, client: ClientState) -> Markup {
    let content = html! {
        (head(&["404"], "404", None, &[Action::primary("Home", "/")]))
        main {
            section class="panel span pad" {
                p style="margin:0" { "Page not found" }
            }
        }
    };

    base(
        PageContext {
            title: "404".to_string(),
            canonical: format!("/{}", path),
        },
        content,
        state,
        client,
    )
}

pub struct Row<'a> {
    pub date: String,
    pub href: &'a str,
    pub title: &'a str,
    pub description: Option<&'a str>,
    pub external: bool,
}

pub fn list(rows: &[Row]) -> Markup {
    html! {
        table class="list" {
            tbody {
                @for row in rows {
                    tr {
                        td class="d" { (row.date) }
                        td {
                            @if row.external {
                                a class="mono" target="_blank" href=(row.href) { (row.title) }
                            } @else {
                                a href=(row.href) { (row.title) }
                            }
                            @if let Some(description) = row.description {
                                div class="sub" { (description) }
                            }
                        }
                    }
                }
            }
        }
    }
}
