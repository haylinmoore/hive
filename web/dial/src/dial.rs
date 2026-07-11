use axum::extract::Query;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use std::sync::Arc;
use twiml_rust::{
    TwiML, VoiceResponse,
    voice::{Gather, Play},
};

use crate::config::{Config, ConfigKeyspace};

pub struct Twiml(VoiceResponse);

impl IntoResponse for Twiml {
    fn into_response(self) -> Response {
        let body = self.0.to_xml();
        (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/xml")],
            body,
        )
            .into_response()
    }
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case, dead_code)]
pub struct TwiMLParameters {
    To: String,
    From: String,
    Digits: Option<String>,
}

pub async fn dial_inbound(
    State(config): State<Arc<Config>>,
    params: Query<TwiMLParameters>,
) -> Twiml {
    println!("inbound: {:?}", params);

    config.log_call(params.From.clone());

    if config.get_bool(ConfigKeyspace::AllCallsDial) {
        return Twiml(
            VoiceResponse::new().dial(config.get_string(ConfigKeyspace::DialPhoneNumber)),
        );
    }

    if config.get_string(ConfigKeyspace::LandlordCallerId) == params.From {
        return Twiml(
            VoiceResponse::new().dial(config.get_string(ConfigKeyspace::DialPhoneNumber)),
        );
    }

    if config.get_string(ConfigKeyspace::DoorkingCallerId) != params.From {
        return Twiml(
            VoiceResponse::new().dial(config.get_string(ConfigKeyspace::DialPhoneNumber)),
        );
    }

    let gather = Gather::new()
        .input(vec!["dtmf".to_string()])
        .action("/doorking_gather")
        .method("GET")
        .timeout(8)
        .num_digits(6)
        .finish_on_key("#");

    Twiml(VoiceResponse::new().gather(gather).dial(config.get_string(ConfigKeyspace::DialPhoneNumber)))
}

pub async fn dial_doorking_gather(
    State(config): State<Arc<Config>>,
    params: Query<TwiMLParameters>,
) -> Twiml {
    println!("doorking_gather: {:?}", params);

    if config.get_string(ConfigKeyspace::DoorkingCallerId) != params.From {
        return Twiml(VoiceResponse::new());
    }

    if let Some(digits) = &params.Digits {
        if config.check_door_code(digits.to_string()) {
            let play = Play::new().digits("9999999999999");

            return Twiml(VoiceResponse::new().play_with(play));
        }

        return Twiml(VoiceResponse::new().say("Incorrect code"));
    }

    Twiml(VoiceResponse::new())
}
