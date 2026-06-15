use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
pub struct CurrentWeather {
    pub features: Vec<Feature>,
}

#[derive(Deserialize)]
pub struct Feature {
    pub geometry: Geometry,
    pub properties: StationProps,
}

#[derive(Deserialize)]
pub struct Geometry {
    pub coordinates: [String; 2],
}

#[derive(Deserialize)]
pub struct StationProps {
    pub nume: String,
    pub tempe: String,
    pub umezeala: Value,
    pub vant: String,
    pub nebulozitate: String,
    pub presiunetext: String,
    pub fenomen_e: String,
    pub zapada: String,
    pub tempapa: String,
    pub actualizat: String,
}

#[derive(Deserialize)]
pub struct ForecastRoot {
    pub tara: Tara,
}

#[derive(Deserialize)]
pub struct Tara {
    pub localitate: Vec<Localitate>,
}

#[derive(Deserialize)]
pub struct Localitate {
    #[serde(rename = "@attributes")]
    pub attrs: NameAttr,
    pub prognoza: Vec<DayForecast>,
}

#[derive(Deserialize)]
pub struct NameAttr {
    pub nume: String,
}

#[derive(Deserialize)]
pub struct DayForecast {
    #[serde(rename = "@attributes")]
    pub attrs: DateAttr,
    pub temp_min: String,
    pub temp_max: String,
    pub fenomen_descriere: String,
}

#[derive(Deserialize)]
pub struct DateAttr {
    pub data: String,
}

#[derive(Deserialize)]
pub struct WarningsRoot {
    pub avertizare: Vec<Warning>,
}

#[derive(Deserialize)]
pub struct Warning {
    #[serde(rename = "@attributes")]
    pub attrs: WarningAttrs,
    pub judet: Vec<Area>,
    pub zona: Vec<Area>,
}

#[derive(Deserialize)]
pub struct WarningAttrs {
    #[serde(rename = "numeTipMesaj")]
    pub tip: String,
    #[serde(rename = "intervalul")]
    pub interval: String,
    #[serde(rename = "culoare")]
    pub severity_code: String,
    #[serde(rename = "dataAparitiei")]
    pub appeared: String,
    #[serde(rename = "dataExpirarii")]
    pub expires: String,
    #[serde(rename = "fenomeneVizate")]
    pub phenomena: String,
    #[serde(rename = "zonaAfectata")]
    pub zone_text: String,
    #[serde(rename = "mesaj")]
    pub message: String,
}

#[derive(Deserialize)]
pub struct Area {
    #[serde(rename = "@attributes")]
    pub attrs: AreaAttrs,
}

#[derive(Deserialize)]
pub struct AreaAttrs {
    pub cod: String,
    pub culoare: String,
}
