use axum::extract::{Json, State};
use chrono::DateTime;
use chrono::offset::Utc;
use maud::{Markup, html};
use serde::Deserialize;
use std::sync::Arc;

use crate::config::{Config, ConfigKeyspace, DoorCode};

fn panel_route(config: Arc<Config>) -> Markup {
    html! {
        head {
            link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/sakura.css@1.5.1/css/sakura-dark.css" {}
            script src="https://cdn.jsdelivr.net/npm/htmx.org@2.0.10/dist/htmx.min.js" integrity="sha384-H5SrcfygHmAuTDZphMHqBJLc3FhssKjG7w/CeCpFReSfwBWDTKpkzPP8c+cLsK+V" crossorigin="anonymous" {}
            script src="https://unpkg.com/htmx-ext-json-enc@2.0.1/json-enc.js" {}
        }

        h2 "dial.uwu.estate management pane" {}

        table {
            tr {
                th { "Option" }
                th { "Value" }
                th { "Manage" }
            }

            @for option in [(ConfigKeyspace::AllCallsDial, "All Calls Dial"), (ConfigKeyspace::ValidateDial, "Validate Dials")] {
                @let value = config.get_bool(option.0.clone());
                tr {
                    td { (option.1) }
                    td {
                        (value)
                    }
                    td {
                        button hx-post="./" hx-ext="json-enc" hx-target="body" hx-include="inherit" hx-vals=(format!("{{\"type\": \"SetBool\", \"id\": {}, \"val\": {}}}", option.0 as u8, !value )) { "Toggle" }
                    }
                }
            }

            @for option in [(ConfigKeyspace::DialPhoneNumber, "Dial Phone Number"), (ConfigKeyspace::DoorkingCallerId, "DoorKing Caller ID"), (ConfigKeyspace::LandlordCallerId, "Landlord Caller ID"), (ConfigKeyspace::BaseDialUrl, "Base Dial URL"), (ConfigKeyspace::TwilioAuthToken, "Twilio Auth Token")] {
                tr hx-include="this" {
                    td { (option.1) }
                    td {
                        input name="val" value=(config.get_string(option.0.clone())) autocomplete="off";
                    }
                    td hx-include="inherit" {
                        button hx-post="./" hx-ext="json-enc" hx-target="body" hx-include="inherit" hx-vals=(format!("{{\"type\": \"SetString\", \"id\": {}}}", option.0 as u8)) { "Set" }
                    }
                }
            }
        }

        table {
            tr {
                th { "Name" }
                th { "Status" }
                th { "Code" }
                th { "Manage" }
            }

            @for code in config.get_door_codes() {
                tr hx-include="this" {
                    td {
                        input name="name" value=(code.name);
                    }
                    td hx-include="inherit" {
                        @if code.enabled {
                            input name="enabled" type="checkbox" checked;
                        } @else {
                            input name="enabled" type="checkbox";
                        }
                    }
                    td {
                        input name="code" value=(code.code);
                    }
                    td hx-include="inherit" {
                        button hx-post="./" hx-ext="json-enc" hx-target="body" hx-include="inherit" hx-vals=(format!("{{\"type\": \"UpdateCode\", \"id\": {}}}", code.id)) { "Update" }
                        button hx-post="./" hx-ext="json-enc" hx-target="body" hx-include="inherit" hx-vals=(format!("{{\"type\": \"DeleteCode\", \"id\": {}}}", code.id)) { "Delete" }
                    }
                };
            }

            tr hx-include="this" {
                td {
                    input id="new.name" name="name" placeholder="Name" autocomplete="off";
                }
                td {
                    input id="new.enabled" name="enabled" type="checkbox" checked autocomplete="off";
                }
                td {
                    input id="new.code" name="code" placeholder="123456" minlength="6" maxlength="6"  autocomplete="off";
                }
                td hx-include="inherit" {
                    button hx-post="./" hx-ext="json-enc" hx-target="body" hx-include="inherit" hx-vals="{\"type\": \"CreateCode\"}" { "Create" }
                }
            }
        }

        table {
            tr {
                th { "Date" }
                th { "Code" }
                th { "Name" }
                th { "Success" }
            }

            @for call in config.get_pin_log(48) {
                @let datetime: DateTime<Utc> = call.date.into();

                tr {
                    td {(datetime.format("%d/%m/%Y %T"))}
                    td {(call.code)}
                    td {(call.name)}
                    td {(call.success)}
                }
            }
        }

        table {
            tr {
                th { "Date" }
                th { "From" }
            }

            @for call in config.get_call_log(48) {
                @let datetime: DateTime<Utc> = call.date.into();

                tr {
                    td {(datetime.format("%d/%m/%Y %T"))}
                    td {(call.from)}
                }
            }
        }
    }
}

pub async fn get_panel_route(State(config): State<Arc<Config>>) -> Markup {
    return panel_route(config);
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum PostBody {
    CreateCode {
        name: String,
        code: String,
        enabled: Option<String>,
    },
    UpdateCode {
        id: u64,
        name: String,
        code: String,
        enabled: Option<String>,
    },
    DeleteCode {
        id: u64,
    },
    SetString {
        id: ConfigKeyspace,
        val: String,
    },
    SetBool {
        id: ConfigKeyspace,
        val: bool,
    },
}
pub async fn post_panel_route(
    State(config): State<Arc<Config>>,
    Json(body): Json<PostBody>,
) -> Markup {
    match body {
        PostBody::CreateCode {
            name,
            code,
            enabled,
        } => {
            config.add_door_code(DoorCode {
                id: 0,
                name,
                code,
                enabled: enabled.is_some(),
            });
        }
        PostBody::UpdateCode {
            id,
            name,
            code,
            enabled,
        } => {
            config.update_door_code(DoorCode {
                id,
                name,
                code,
                enabled: enabled.is_some(),
            });
        }
        PostBody::DeleteCode { id } => {
            config.delete_door_code(id);
        }
        PostBody::SetString { id, val } => config.set_string(id, val),
        PostBody::SetBool { id, val } => config.set_bool(id, val),
    }

    return panel_route(config);
}
