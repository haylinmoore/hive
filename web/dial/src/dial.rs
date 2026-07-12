use axum::extract::{FromRef, FromRequestParts, Query, State};
use axum::http::request::Parts;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use base64::{Engine, engine::general_purpose::STANDARD};
use hmac::{Hmac, KeyInit, Mac};
use serde::Deserialize;

use sha1::Sha1;
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

pub struct TwilioVerified;

impl<S> FromRequestParts<S> for TwilioVerified
where
    Arc<Config>: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let config = Arc::<Config>::from_ref(state);

        if !config.get_bool(ConfigKeyspace::ValidateDial) {
            return Ok(TwilioVerified);
        }

        let Some(signature) = parts
            .headers
            .get("X-Twilio-Signature")
            .and_then(|v| v.to_str().ok())
        else {
            return Err(Twiml(VoiceResponse::new().say("Request Lacks Signature")).into_response());
        };

        let path_and_query = parts
            .uri
            .path_and_query()
            .map(|pq| pq.as_str())
            .unwrap_or("/");
        let url = format!(
            "{}{}",
            config.get_string(ConfigKeyspace::BaseDialUrl),
            path_and_query
        );

        let mut mac = Hmac::<Sha1>::new_from_slice(
            config
                .get_string(ConfigKeyspace::TwilioAuthToken)
                .as_bytes(),
        )
        .expect("HMAC accepts any key length");
        mac.update(url.as_bytes());
        let expected = STANDARD.encode(mac.finalize().into_bytes());

        if expected.as_bytes() == signature.as_bytes() {
            Ok(TwilioVerified)
        } else {
            Err(Twiml(VoiceResponse::new().say("Invalid Signature")).into_response())
        }
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
    _: TwilioVerified,
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

    Twiml(
        VoiceResponse::new()
            .gather(gather)
            .dial(config.get_string(ConfigKeyspace::DialPhoneNumber)),
    )
}

pub async fn dial_doorking_gather(
    _: TwilioVerified,
    State(config): State<Arc<Config>>,
    params: Query<TwiMLParameters>,
) -> Twiml {
    println!("doorking_gather: {:?}", params);

    if config.get_string(ConfigKeyspace::DoorkingCallerId) != params.From {
        return Twiml(VoiceResponse::new());
    }

    if let Some(digits) = &params.Digits {
        let digits = if config.check_door_code(digits.to_string()) {
            "9999999999999".to_string()
        } else {
            "############".to_string()
        };

        let play = Play::new().digits(digits);
        return Twiml(VoiceResponse::new().play_with(play));
    }

    Twiml(VoiceResponse::new())
}
