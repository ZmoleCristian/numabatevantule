use std::sync::{Arc, Mutex};

use crate::domain::{self, Alert, CityForecast, Station};
use crate::dto;
use crate::error::MeteoError;

pub enum Incoming {
    Stations(Result<Vec<Station>, MeteoError>),
    Forecasts(Result<Vec<CityForecast>, MeteoError>),
    Alerts(Result<Vec<Alert>, MeteoError>),
}

pub type Inbox = Arc<Mutex<Vec<Incoming>>>;

pub fn new_inbox() -> Inbox {
    Arc::new(Mutex::new(Vec::new()))
}

#[cfg(target_arch = "wasm32")]
fn endpoint(name: &str) -> String {
    format!("./data/{name}.json")
}

#[cfg(not(target_arch = "wasm32"))]
fn endpoint(name: &str) -> String {
    format!("https://www.meteoromania.ro/wp-json/meteoapi/v2/{name}")
}

fn push(inbox: &Inbox, message: Incoming) {
    match inbox.lock() {
        Ok(mut guard) => guard.push(message),
        Err(poison) => panic!("inbox mutex poisoned: {poison}"),
    }
}

pub fn refresh(ctx: &egui::Context, inbox: &Inbox) {
    fetch_stations(ctx, inbox);
    fetch_forecasts(ctx, inbox);
    fetch_alerts(ctx, inbox);
}

fn fetch_stations(ctx: &egui::Context, inbox: &Inbox) {
    let ctx = ctx.clone();
    let inbox = Arc::clone(inbox);
    let request = ehttp::Request::get(endpoint("starea-vremii"));
    ehttp::fetch(request, move |result| {
        let outcome = match result {
            Ok(response) => decode_stations(response),
            Err(transport) => Err(MeteoError::Transport(transport)),
        };
        push(&inbox, Incoming::Stations(outcome));
        ctx.request_repaint();
    });
}

fn fetch_forecasts(ctx: &egui::Context, inbox: &Inbox) {
    let ctx = ctx.clone();
    let inbox = Arc::clone(inbox);
    let request = ehttp::Request::get(endpoint("prognoza-orase"));
    ehttp::fetch(request, move |result| {
        let outcome = match result {
            Ok(response) => decode_forecasts(response),
            Err(transport) => Err(MeteoError::Transport(transport)),
        };
        push(&inbox, Incoming::Forecasts(outcome));
        ctx.request_repaint();
    });
}

fn fetch_alerts(ctx: &egui::Context, inbox: &Inbox) {
    let ctx = ctx.clone();
    let inbox = Arc::clone(inbox);
    let request = ehttp::Request::get(endpoint("avertizari-generale"));
    ehttp::fetch(request, move |result| {
        let outcome = match result {
            Ok(response) => decode_alerts(response),
            Err(transport) => Err(MeteoError::Transport(transport)),
        };
        push(&inbox, Incoming::Alerts(outcome));
        ctx.request_repaint();
    });
}

fn decode_stations(response: ehttp::Response) -> Result<Vec<Station>, MeteoError> {
    if !response.ok {
        return Err(MeteoError::HttpStatus(response.status));
    }
    let parsed: dto::CurrentWeather = match serde_json::from_slice(&response.bytes) {
        Ok(parsed) => parsed,
        Err(cause) => return Err(MeteoError::Decode(cause.to_string())),
    };
    domain::stations(parsed)
}

fn decode_forecasts(response: ehttp::Response) -> Result<Vec<CityForecast>, MeteoError> {
    if !response.ok {
        return Err(MeteoError::HttpStatus(response.status));
    }
    let parsed: dto::ForecastRoot = match serde_json::from_slice(&response.bytes) {
        Ok(parsed) => parsed,
        Err(cause) => return Err(MeteoError::Decode(cause.to_string())),
    };
    domain::forecasts(parsed)
}

fn decode_alerts(response: ehttp::Response) -> Result<Vec<Alert>, MeteoError> {
    if !response.ok {
        return Err(MeteoError::HttpStatus(response.status));
    }
    let value: serde_json::Value = match serde_json::from_slice(&response.bytes) {
        Ok(value) => value,
        Err(cause) => return Err(MeteoError::Decode(cause.to_string())),
    };
    match value {
        serde_json::Value::String(_marker) => Ok(Vec::new()),
        populated => {
            let parsed: dto::WarningsRoot = match serde_json::from_value(populated) {
                Ok(parsed) => parsed,
                Err(cause) => return Err(MeteoError::Decode(cause.to_string())),
            };
            domain::alerts(parsed)
        }
    }
}
