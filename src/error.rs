use std::fmt;

pub enum MeteoError {
    Transport(String),
    HttpStatus(u16),
    Decode(String),
    BadCoordinate(String),
    BadReading(String),
    BadGeometry(String),
    UnknownCity(String),
}

impl fmt::Display for MeteoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            MeteoError::Transport(detail) => write!(f, "transport: {detail}"),
            MeteoError::HttpStatus(code) => write!(f, "http status {code}"),
            MeteoError::Decode(detail) => write!(f, "decode: {detail}"),
            MeteoError::BadCoordinate(raw) => write!(f, "bad coordinate: {raw}"),
            MeteoError::BadReading(raw) => write!(f, "bad reading: {raw}"),
            MeteoError::BadGeometry(raw) => write!(f, "bad geometry: {raw}"),
            MeteoError::UnknownCity(name) => write!(f, "unknown city: {name}"),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn trace(err: &MeteoError) {
    eprintln!("[meteo] {err}");
}

#[cfg(target_arch = "wasm32")]
pub fn trace(err: &MeteoError) {
    web_sys::console::error_1(&format!("[meteo] {err}").into());
}
