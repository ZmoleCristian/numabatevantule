use crate::dto;
use crate::error::MeteoError;

const MERCATOR_SEMI: f64 = 20037508.34;

const CITIES: [(&str, f64, f64); 10] = [
    ("Arad", 46.17, 21.32),
    ("Botosani", 47.75, 26.67),
    ("Bucuresti", 44.43, 26.10),
    ("Cluj-Napoca", 46.77, 23.60),
    ("Constanta", 44.18, 28.65),
    ("Craiova", 44.32, 23.80),
    ("Iasi", 47.16, 27.59),
    ("Rm. Valcea", 45.10, 24.37),
    ("Sibiu", 45.79, 24.15),
    ("Sulina", 45.16, 29.65),
];

#[derive(Clone, Copy)]
pub struct LatLon {
    pub lat: f64,
    pub lon: f64,
}

#[derive(Clone, Copy)]
pub enum Reading {
    Missing,
    Celsius(f32),
}

#[derive(Clone, Copy)]
pub enum WindDir {
    Unknown,
    Vector { degrees: f32, speed: f32 },
}

enum Bearing {
    Unknown,
    Degrees(f32),
}

#[derive(Clone, Copy)]
pub enum Severity {
    Green,
    Yellow,
    Orange,
    Red,
}

#[derive(Clone)]
pub struct Station {
    pub name: String,
    pub coord: LatLon,
    pub temp: Reading,
    pub humidity: String,
    pub wind: String,
    pub wind_dir: WindDir,
    pub clouds: String,
    pub pressure: String,
    pub phenomenon: String,
    pub snow: String,
    pub water: String,
    pub updated: String,
}

pub struct CityDay {
    pub date: String,
    pub tmin: String,
    pub tmin_value: Reading,
    pub tmax_value: Reading,
    pub tmax_text: String,
    pub description: String,
}

pub struct CityForecast {
    pub name: String,
    pub coord: LatLon,
    pub days: Vec<CityDay>,
}

pub struct CountyHit {
    pub name: String,
    pub severity: Severity,
}

pub struct Alert {
    pub kind: String,
    pub interval: String,
    pub appeared: String,
    pub expires: String,
    pub phenomena: String,
    pub zone_text: String,
    pub message: String,
    pub severity: Severity,
    pub counties: Vec<CountyHit>,
}

fn reading(raw: &str) -> Result<Reading, MeteoError> {
    if raw == "indisponibil" {
        return Ok(Reading::Missing);
    }
    let Ok(value) = raw.parse::<f32>() else {
        return Err(MeteoError::BadReading(raw.to_string()));
    };
    Ok(Reading::Celsius(value))
}

fn coordinate(raw: &str) -> Result<f64, MeteoError> {
    let Ok(value) = raw.parse::<f64>() else {
        return Err(MeteoError::BadCoordinate(raw.to_string()));
    };
    Ok(value)
}

fn mercator_point(x: f64, y: f64) -> LatLon {
    let lon = x / MERCATOR_SEMI * 180.0;
    let lat_linear = y / MERCATOR_SEMI * 180.0;
    let lat = lat_linear.to_radians().sinh().atan().to_degrees();
    LatLon { lat, lon }
}

fn epsg3857_to_latlon(coords: &[String; 2]) -> Result<LatLon, MeteoError> {
    let x = coordinate(&coords[0])?;
    let y = coordinate(&coords[1])?;
    Ok(mercator_point(x, y))
}

fn severity(code: &str) -> Severity {
    match code {
        "1" => Severity::Yellow,
        "2" => Severity::Orange,
        "3" => Severity::Red,
        _level => Severity::Green,
    }
}

fn rank(level: Severity) -> u8 {
    match level {
        Severity::Green => 0,
        Severity::Yellow => 1,
        Severity::Orange => 2,
        Severity::Red => 3,
    }
}

fn decode_entities(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    loop {
        let Some(amp) = rest.find('&') else {
            out.push_str(rest);
            return out;
        };
        out.push_str(&rest[..amp]);
        let after = &rest[amp + 1..];
        let Some(semi) = after.find(';') else {
            out.push('&');
            rest = after;
            continue;
        };
        out.push_str(&decode_one(&after[..semi]));
        rest = &after[semi + 1..];
    }
}

fn decode_one(name: &str) -> String {
    let glyph = match name {
        "nbsp" => " ",
        "amp" => "&",
        "lt" => "<",
        "gt" => ">",
        "quot" => "\"",
        "apos" => "'",
        "ndash" => "–",
        "mdash" => "—",
        "hellip" => "…",
        "deg" => "°",
        "acirc" => "â",
        "Acirc" => "Â",
        "icirc" => "î",
        "Icirc" => "Î",
        "abreve" => "ă",
        "Abreve" => "Ă",
        "scedil" | "scaron" => "ș",
        "Scedil" | "Scaron" => "Ș",
        "tcedil" | "tcaron" => "ț",
        "Tcedil" | "Tcaron" => "Ț",
        other => return decode_numeric(other),
    };
    glyph.to_string()
}

fn decode_numeric(name: &str) -> String {
    let Some(digits) = name.strip_prefix('#') else {
        return format!("&{name};");
    };
    let code = if digits.starts_with('x') {
        u32::from_str_radix(&digits[1..], 16)
    } else {
        digits.parse::<u32>()
    };
    let Ok(value) = code else {
        return format!("&{name};");
    };
    let Some(glyph) = char::from_u32(value) else {
        return format!("&{name};");
    };
    glyph.to_string()
}

fn compass(direction: &str) -> Bearing {
    let degrees = match direction {
        "n" => 0.0,
        "nne" => 22.5,
        "ne" => 45.0,
        "ene" => 67.5,
        "e" => 90.0,
        "ese" => 112.5,
        "se" => 135.0,
        "sse" => 157.5,
        "s" => 180.0,
        "ssv" => 202.5,
        "sv" => 225.0,
        "vsv" => 247.5,
        "v" => 270.0,
        "vnv" => 292.5,
        "nv" => 315.0,
        "nnv" => 337.5,
        _unknown => return Bearing::Unknown,
    };
    Bearing::Degrees(degrees)
}

fn wind_speed(lower: &str) -> f32 {
    let Some(marker) = lower.find("m/s") else {
        return 0.0;
    };
    let head = lower[..marker].trim();
    let Some(token) = head.split_whitespace().last() else {
        return 0.0;
    };
    let Ok(value) = token.parse::<f32>() else {
        return 0.0;
    };
    value
}

fn wind_direction(text: &str) -> WindDir {
    let lower = text.to_lowercase();
    let Some(marker) = lower.find("directia") else {
        return WindDir::Unknown;
    };
    let tail = &lower[marker..];
    let Some(colon) = tail.find(':') else {
        return WindDir::Unknown;
    };
    let degrees = match compass(tail[colon + 1..].trim()) {
        Bearing::Degrees(value) => value,
        Bearing::Unknown => return WindDir::Unknown,
    };
    WindDir::Vector {
        degrees,
        speed: wind_speed(&lower),
    }
}

fn strip_html(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut inside_tag = false;
    for ch in raw.chars() {
        match ch {
            '<' => inside_tag = true,
            '>' => inside_tag = false,
            kept => {
                if !inside_tag {
                    out.push(kept);
                }
            }
        }
    }
    decode_entities(&out).trim().to_string()
}

fn station(feature: dto::Feature) -> Result<Station, MeteoError> {
    let coord = epsg3857_to_latlon(&feature.geometry.coordinates)?;
    let temp = reading(&feature.properties.tempe)?;
    let wind_dir = wind_direction(&feature.properties.vant);
    Ok(Station {
        name: decode_entities(&feature.properties.nume),
        coord,
        temp,
        humidity: feature.properties.umezeala.to_string(),
        wind: decode_entities(&feature.properties.vant),
        wind_dir,
        clouds: decode_entities(&feature.properties.nebulozitate),
        pressure: decode_entities(&feature.properties.presiunetext),
        phenomenon: decode_entities(&feature.properties.fenomen_e),
        snow: decode_entities(&feature.properties.zapada),
        water: decode_entities(&feature.properties.tempapa),
        updated: decode_entities(&feature.properties.actualizat),
    })
}

fn city_coord(name: &str) -> Result<LatLon, MeteoError> {
    let Some(found) = CITIES.iter().find(|entry| entry.0.eq_ignore_ascii_case(name)) else {
        return Err(MeteoError::UnknownCity(name.to_string()));
    };
    Ok(LatLon {
        lat: found.1,
        lon: found.2,
    })
}

fn city_forecast(localitate: dto::Localitate) -> Result<CityForecast, MeteoError> {
    let name = localitate.attrs.nume;
    let coord = city_coord(&name)?;
    let mut days = Vec::with_capacity(localitate.prognoza.len());
    for forecast in localitate.prognoza {
        let tmin_value = reading(&forecast.temp_min)?;
        let tmax_value = reading(&forecast.temp_max)?;
        days.push(CityDay {
            date: forecast.attrs.data,
            tmin: forecast.temp_min,
            tmin_value,
            tmax_value,
            tmax_text: forecast.temp_max,
            description: forecast.fenomen_descriere,
        });
    }
    Ok(CityForecast { name, coord, days })
}

const COUNTY_CODES: [(&str, &str); 42] = [
    ("AB", "Alba"),
    ("AR", "Arad"),
    ("AG", "Arges"),
    ("BC", "Bacau"),
    ("BH", "Bihor"),
    ("BN", "Bistrita-Nasaud"),
    ("BT", "Botosani"),
    ("BV", "Brasov"),
    ("BR", "Braila"),
    ("B", "Bucuresti"),
    ("BZ", "Buzau"),
    ("CL", "Calarasi"),
    ("CS", "Caras-Severin"),
    ("CJ", "Cluj"),
    ("CT", "Constanta"),
    ("CV", "Covasna"),
    ("DB", "Dambovita"),
    ("DJ", "Dolj"),
    ("GL", "Galati"),
    ("GR", "Giurgiu"),
    ("GJ", "Gorj"),
    ("HR", "Harghita"),
    ("HD", "Hunedoara"),
    ("IS", "Iasi"),
    ("IL", "Ialomita"),
    ("IF", "Ilfov"),
    ("MM", "Maramures"),
    ("MH", "Mehedinti"),
    ("MS", "Mures"),
    ("NT", "Neamt"),
    ("OT", "Olt"),
    ("PH", "Prahova"),
    ("SJ", "Salaj"),
    ("SM", "Satu Mare"),
    ("SB", "Sibiu"),
    ("SV", "Suceava"),
    ("TR", "Teleorman"),
    ("TM", "Timis"),
    ("TL", "Tulcea"),
    ("VL", "Valcea"),
    ("VS", "Vaslui"),
    ("VN", "Vrancea"),
];

fn county_code(cod: &str) -> &str {
    let Some(head) = cod.split('_').next() else {
        return cod;
    };
    head
}

fn add_hit(hits: &mut Vec<CountyHit>, cod: &str, culoare: &str) {
    let key = county_code(cod);
    let Some(entry) = COUNTY_CODES.iter().find(|item| item.0 == key) else {
        return;
    };
    let level = severity(culoare);
    for hit in hits.iter_mut() {
        if hit.name == entry.1 {
            if rank(level) > rank(hit.severity) {
                hit.severity = level;
            }
            return;
        }
    }
    hits.push(CountyHit {
        name: entry.1.to_string(),
        severity: level,
    });
}

fn alert(warning: dto::Warning) -> Alert {
    let mut counties = Vec::new();
    for area in &warning.judet {
        add_hit(&mut counties, &area.attrs.cod, &area.attrs.culoare);
    }
    for area in &warning.zona {
        add_hit(&mut counties, &area.attrs.cod, &area.attrs.culoare);
    }
    Alert {
        kind: decode_entities(&warning.attrs.tip),
        interval: decode_entities(&warning.attrs.interval),
        appeared: warning.attrs.appeared,
        expires: warning.attrs.expires,
        phenomena: decode_entities(&warning.attrs.phenomena),
        zone_text: decode_entities(&warning.attrs.zone_text),
        message: strip_html(&warning.attrs.message),
        severity: severity(&warning.attrs.severity_code),
        counties,
    }
}

pub fn stations(raw: dto::CurrentWeather) -> Result<Vec<Station>, MeteoError> {
    let mut out = Vec::with_capacity(raw.features.len());
    for feature in raw.features {
        out.push(station(feature)?);
    }
    Ok(out)
}

pub fn forecasts(raw: dto::ForecastRoot) -> Result<Vec<CityForecast>, MeteoError> {
    let mut out = Vec::with_capacity(raw.tara.localitate.len());
    for localitate in raw.tara.localitate {
        out.push(city_forecast(localitate)?);
    }
    Ok(out)
}

pub fn alerts(raw: dto::WarningsRoot) -> Result<Vec<Alert>, MeteoError> {
    let mut out = Vec::with_capacity(raw.avertizare.len());
    for warning in raw.avertizare {
        out.push(alert(warning));
    }
    Ok(out)
}
