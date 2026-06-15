use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::Deserialize;

use crate::config;
use crate::domain::{self, Station};
use crate::dto;
use crate::error::MeteoError;
use crate::fmt;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DayKey {
    pub y: i64,
    pub m: i64,
    pub d: i64,
}

impl DayKey {
    pub const ZERO: DayKey = DayKey { y: 0, m: 0, d: 0 };
}

pub struct Snapshot {
    pub sha: String,
    pub timestamp: String,
}

pub enum HistoryIncoming {
    Recent(Result<Vec<Snapshot>, MeteoError>),
    Day {
        key: DayKey,
        snaps: Result<Vec<Snapshot>, MeteoError>,
    },
    Frame {
        sha: String,
        stations: Result<Vec<Station>, MeteoError>,
    },
}

pub type HistoryInbox = Arc<Mutex<Vec<HistoryIncoming>>>;

pub fn new_inbox() -> HistoryInbox {
    Arc::new(Mutex::new(Vec::new()))
}

pub struct History {
    pub active: bool,
    pub started: bool,
    pub snapshots: Vec<Snapshot>,
    pub selected: usize,
    pub frames: HashMap<String, Vec<Station>>,
    pub shown: String,
    pub inflight: String,
    pub day: DayKey,
    pub view: DayKey,
    pub newest: DayKey,
    pub have_newest: bool,
    pub have_day: bool,
    pub loading: bool,
    pub calendar_open: bool,
}

impl History {
    pub fn new() -> History {
        History {
            active: false,
            started: false,
            snapshots: Vec::new(),
            selected: 0,
            frames: HashMap::new(),
            shown: String::new(),
            inflight: String::new(),
            day: DayKey::ZERO,
            view: DayKey::ZERO,
            newest: DayKey::ZERO,
            have_newest: false,
            have_day: false,
            loading: false,
            calendar_open: false,
        }
    }
}

pub fn day_of(iso: &str) -> DayKey {
    let Some((date, _rest)) = iso.split_once('T') else {
        return DayKey::ZERO;
    };
    let parts: Vec<&str> = date.split('-').collect();
    let [year, month, day] = parts.as_slice() else {
        return DayKey::ZERO;
    };
    let (Ok(y), Ok(m), Ok(d)) = (year.parse::<i64>(), month.parse::<i64>(), day.parse::<i64>()) else {
        return DayKey::ZERO;
    };
    DayKey { y, m, d }
}

fn push(inbox: &HistoryInbox, message: HistoryIncoming) {
    match inbox.lock() {
        Ok(mut guard) => guard.push(message),
        Err(poison) => panic!("history inbox poisoned: {poison}"),
    }
}

fn github_request(url: String) -> ehttp::Request {
    let mut request = ehttp::Request::get(url);
    request.headers.insert("User-Agent", "meteo_ro");
    request.headers.insert("Accept", "application/vnd.github+json");
    request
}

fn recent_url() -> String {
    format!(
        "{}/repos/{}/commits?path=data/starea-vremii.json&per_page=100",
        config::API_BASE,
        config::REPO_SLUG
    )
}

fn iso_midnight(key: DayKey) -> String {
    format!("{:04}-{:02}-{:02}T00:00:00Z", key.y, key.m, key.d)
}

fn day_url(key: DayKey) -> String {
    let (ny, nm, nd) = fmt::next_day(key.y, key.m, key.d);
    let since = iso_midnight(key);
    let until = iso_midnight(DayKey { y: ny, m: nm, d: nd });
    format!(
        "{}/repos/{}/commits?path=data/starea-vremii.json&per_page=100&since={since}&until={until}",
        config::API_BASE,
        config::REPO_SLUG
    )
}

fn raw_url(sha: &str) -> String {
    format!("{}/{}/{}/data/starea-vremii.json", config::RAW_BASE, config::REPO_SLUG, sha)
}

pub fn load_recent(ctx: &egui::Context, inbox: &HistoryInbox) {
    let ctx = ctx.clone();
    let inbox = Arc::clone(inbox);
    ehttp::fetch(github_request(recent_url()), move |result| {
        let outcome = match result {
            Ok(response) => decode_index(response),
            Err(transport) => Err(MeteoError::Transport(transport)),
        };
        push(&inbox, HistoryIncoming::Recent(outcome));
        ctx.request_repaint();
    });
}

pub fn load_day(ctx: &egui::Context, inbox: &HistoryInbox, key: DayKey) {
    let ctx = ctx.clone();
    let inbox = Arc::clone(inbox);
    ehttp::fetch(github_request(day_url(key)), move |result| {
        let snaps = match result {
            Ok(response) => decode_index(response),
            Err(transport) => Err(MeteoError::Transport(transport)),
        };
        push(&inbox, HistoryIncoming::Day { key, snaps });
        ctx.request_repaint();
    });
}

pub fn load_frame(ctx: &egui::Context, inbox: &HistoryInbox, sha: String) {
    let ctx = ctx.clone();
    let inbox = Arc::clone(inbox);
    ehttp::fetch(github_request(raw_url(&sha)), move |result| {
        let stations = match result {
            Ok(response) => decode_stations(response),
            Err(transport) => Err(MeteoError::Transport(transport)),
        };
        push(&inbox, HistoryIncoming::Frame { sha, stations });
        ctx.request_repaint();
    });
}

fn decode_index(response: ehttp::Response) -> Result<Vec<Snapshot>, MeteoError> {
    if !response.ok {
        return Err(MeteoError::HttpStatus(response.status));
    }
    let rows: Vec<CommitRow> = match serde_json::from_slice(&response.bytes) {
        Ok(rows) => rows,
        Err(cause) => return Err(MeteoError::Decode(cause.to_string())),
    };
    let mut snapshots = Vec::with_capacity(rows.len());
    for row in rows {
        snapshots.push(Snapshot {
            sha: row.sha,
            timestamp: row.commit.committer.date,
        });
    }
    Ok(snapshots)
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

#[derive(Deserialize)]
struct CommitRow {
    sha: String,
    commit: CommitMeta,
}

#[derive(Deserialize)]
struct CommitMeta {
    committer: CommitWho,
}

#[derive(Deserialize)]
struct CommitWho {
    date: String,
}
