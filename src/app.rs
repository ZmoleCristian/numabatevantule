use egui::Sense;

use crate::domain::Station;
use crate::error::trace;
use crate::fetch::{self, Inbox, Incoming};
use crate::geo::{Camera, CountyMap};
use crate::heat::{self, HeatMesh};
use crate::history::{self, History, HistoryInbox, HistoryIncoming, Snapshot};
use crate::hud::{self, Hud};
use crate::render::{self, Scene};
use crate::theme;
use crate::viewstate::{Datasets, Overlays, Picked, Remote, Selection};
use crate::wind::WindSim;

pub struct MeteoApp {
    map: CountyMap,
    camera: Camera,
    inbox: Inbox,
    data: Datasets,
    overlays: Overlays,
    heat: HeatMesh,
    wind: WindSim,
    day: usize,
    selection: Selection,
    history_inbox: HistoryInbox,
    history: History,
}

impl MeteoApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> MeteoApp {
        theme::install(&cc.egui_ctx);
        let inbox = fetch::new_inbox();
        fetch::refresh(&cc.egui_ctx, &inbox);
        MeteoApp {
            map: CountyMap::load(),
            camera: Camera::new(),
            inbox,
            data: Datasets::loading(),
            overlays: Overlays::defaults(),
            heat: HeatMesh::empty(),
            wind: WindSim::new(),
            day: 0,
            selection: Selection::Nothing,
            history_inbox: history::new_inbox(),
            history: History::new(),
        }
    }

    fn drain(&mut self) {
        let drained = match self.inbox.lock() {
            Ok(mut guard) => guard.drain(..).collect::<Vec<Incoming>>(),
            Err(poison) => panic!("inbox mutex poisoned: {poison}"),
        };
        for message in drained {
            self.apply(message);
        }
    }

    fn apply(&mut self, message: Incoming) {
        match message {
            Incoming::Stations(Ok(list)) => self.set_stations(list),
            Incoming::Stations(Err(cause)) => {
                trace(&cause);
                self.data.stations = Remote::Failed(cause);
            }
            Incoming::Forecasts(Ok(list)) => self.data.forecasts = Remote::Ready(list),
            Incoming::Forecasts(Err(cause)) => {
                trace(&cause);
                self.data.forecasts = Remote::Failed(cause);
            }
            Incoming::Alerts(Ok(list)) => self.data.alerts = Remote::Ready(list),
            Incoming::Alerts(Err(cause)) => {
                trace(&cause);
                self.data.alerts = Remote::Failed(cause);
            }
        }
    }

    fn begin_refresh(&mut self, ctx: &egui::Context) {
        self.data.stations.begin();
        self.data.forecasts.begin();
        self.data.alerts.begin();
        fetch::refresh(ctx, &self.inbox);
    }

    fn set_stations(&mut self, list: Vec<Station>) {
        match heat::build(&list, &self.map) {
            Ok(mesh) => self.heat = mesh,
            Err(cause) => {
                trace(&cause);
                self.heat = HeatMesh::empty();
            }
        }
        self.wind.rebuild(&list, &self.map);
        self.data.stations = Remote::Ready(list);
    }

    fn drain_history(&mut self) {
        let drained = match self.history_inbox.lock() {
            Ok(mut guard) => guard.drain(..).collect::<Vec<HistoryIncoming>>(),
            Err(poison) => panic!("history inbox poisoned: {poison}"),
        };
        for message in drained {
            self.apply_history(message);
        }
    }

    fn apply_history(&mut self, message: HistoryIncoming) {
        match message {
            HistoryIncoming::Recent(Ok(list)) => self.seed_recent(list),
            HistoryIncoming::Recent(Err(cause)) => {
                trace(&cause);
                self.history.loading = false;
            }
            HistoryIncoming::Day { key, snaps } => {
                self.history.loading = false;
                match snaps {
                    Ok(list) => {
                        if key == self.history.day {
                            self.store_day(list);
                        }
                    }
                    Err(cause) => trace(&cause),
                }
            }
            HistoryIncoming::Frame { sha, stations } => match stations {
                Ok(list) => {
                    self.history.frames.insert(sha, list);
                }
                Err(cause) => trace(&cause),
            },
        }
    }

    fn seed_recent(&mut self, list: Vec<Snapshot>) {
        self.history.loading = false;
        self.history.have_newest = true;
        let Some(top) = list.first() else {
            self.history.snapshots = Vec::new();
            self.history.have_day = true;
            return;
        };
        let key = history::day_of(&top.timestamp);
        self.history.newest = key;
        self.history.day = key;
        self.history.view = key;
        let slice: Vec<Snapshot> =
            list.into_iter().filter(|snap| history::day_of(&snap.timestamp) == key).collect();
        self.store_day(slice);
    }

    fn store_day(&mut self, mut list: Vec<Snapshot>) {
        list.reverse();
        self.history.selected = list.len().saturating_sub(1);
        self.history.snapshots = list;
        self.history.have_day = true;
        self.history.shown = String::new();
    }

    fn sync_history(&mut self, ctx: &egui::Context) {
        if !self.history.active {
            if self.history.started {
                self.history.started = false;
                self.history.have_day = false;
                self.history.have_newest = false;
                self.history.calendar_open = false;
                self.history.shown = String::new();
                self.begin_refresh(ctx);
            }
            return;
        }
        if !self.history.started {
            self.history.started = true;
            self.history.loading = true;
            history::load_recent(ctx, &self.history_inbox);
            return;
        }
        if self.history.loading {
            return;
        }
        if !self.history.have_day {
            self.history.loading = true;
            let day = self.history.day;
            history::load_day(ctx, &self.history_inbox, day);
            return;
        }
        let Some(snapshot) = self.history.snapshots.get(self.history.selected) else {
            return;
        };
        let sha = snapshot.sha.clone();
        if self.history.shown == sha {
            return;
        }
        let Some(frame) = self.history.frames.get(&sha).cloned() else {
            if self.history.inflight != sha {
                self.history.inflight = sha.clone();
                history::load_frame(ctx, &self.history_inbox, sha);
            }
            return;
        };
        self.history.shown = sha;
        self.set_stations(frame);
    }

    fn map_panel(&mut self, ui: &mut egui::Ui) {
        let rect = ui.max_rect();
        let response = ui.allocate_rect(rect, Sense::click_and_drag());

        if response.dragged() {
            self.camera.pan += response.drag_delta();
        }
        let scroll = ui.input(|input| input.smooth_scroll_delta.y);
        self.apply_zoom(scroll, &response, rect);
        self.camera.clamp_pan(rect, &self.map);

        let picked = render::draw(
            ui,
            &response,
            rect,
            Scene {
                map: &self.map,
                camera: &self.camera,
                data: &self.data,
                heat: &self.heat,
                wind: &self.wind,
                day: self.day,
                selection: &self.selection,
                overlays: &self.overlays,
            },
        );
        self.apply_pick(picked);
    }

    fn apply_zoom(&mut self, scroll: f32, response: &egui::Response, rect: egui::Rect) {
        if scroll == 0.0 {
            return;
        }
        let Some(cursor) = response.hover_pos() else {
            return;
        };
        self.camera.zoom_to(1.0 + scroll * 0.0015, cursor, rect, &self.map);
    }

    fn apply_pick(&mut self, picked: Picked) {
        match picked {
            Picked::Untouched => {}
            Picked::Empty => self.selection = Selection::Nothing,
            Picked::Hit(name) => self.selection = Selection::Station(name),
        }
    }
}

impl eframe::App for MeteoApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.drain();
        self.drain_history();
        let ctx = ui.ctx().clone();
        self.sync_history(&ctx);

        if self.overlays.wind {
            let dt = ui.input(|input| input.stable_dt).clamp(0.001, 0.05);
            self.wind.update(dt, &self.map);
            ctx.request_repaint();
        }

        self.map_panel(ui);

        let mut refresh = false;
        hud::show(
            &ctx,
            Hud {
                data: &self.data,
                overlays: &mut self.overlays,
                selection: &mut self.selection,
                day: &mut self.day,
                refresh: &mut refresh,
                history: &mut self.history,
            },
        );
        if refresh {
            self.begin_refresh(&ctx);
        }
    }
}
