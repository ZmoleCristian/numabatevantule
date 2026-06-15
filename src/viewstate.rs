use crate::domain::{Alert, CityForecast, Station};
use crate::error::MeteoError;

pub enum Remote<T> {
    Loading,
    Ready(T),
    Failed(MeteoError),
}

impl<T> Remote<T> {
    pub fn begin(&mut self) {
        *self = Remote::Loading;
    }
}

pub struct Datasets {
    pub stations: Remote<Vec<Station>>,
    pub forecasts: Remote<Vec<CityForecast>>,
    pub alerts: Remote<Vec<Alert>>,
}

impl Datasets {
    pub fn loading() -> Datasets {
        Datasets {
            stations: Remote::Loading,
            forecasts: Remote::Loading,
            alerts: Remote::Loading,
        }
    }
}

pub enum Selection {
    Nothing,
    Station(String),
    Warning(usize),
}

pub enum Picked {
    Untouched,
    Empty,
    Hit(String),
}

pub struct Overlays {
    pub warnings: bool,
    pub wind: bool,
    pub heat: bool,
}

impl Overlays {
    pub fn defaults() -> Overlays {
        Overlays {
            warnings: false,
            wind: true,
            heat: true,
        }
    }
}
