use egui::{Pos2, Rect, Vec2};
use serde::Deserialize;

const GEOJSON: &str = include_str!("../assets/romania-counties.geojson");
const OUTLINE_GEOJSON: &str = include_str!("../assets/romania-outline.geojson");

#[derive(Deserialize)]
struct FeatureCollection {
    features: Vec<GjFeature>,
}

#[derive(Deserialize)]
struct GjFeature {
    geometry: Geometry,
    properties: GjProps,
}

#[derive(Deserialize)]
struct GjProps {
    name: String,
}

#[derive(Deserialize)]
#[serde(tag = "type", content = "coordinates")]
enum Geometry {
    Polygon(Vec<Vec<[f64; 2]>>),
    MultiPolygon(Vec<Vec<Vec<[f64; 2]>>>),
}

pub struct County {
    pub name: String,
    pub rings: Vec<Vec<[f64; 2]>>,
}

pub struct CountyMap {
    pub counties: Vec<County>,
    pub outline: Vec<Vec<[f64; 2]>>,
    pub bounds: Bounds,
    merc_min: Vec2,
    merc_max: Vec2,
}

#[derive(Clone, Copy)]
pub struct Bounds {
    pub lon_min: f64,
    pub lon_max: f64,
    pub lat_min: f64,
    pub lat_max: f64,
}

impl CountyMap {
    pub fn contains(&self, lat: f64, lon: f64) -> bool {
        let mut inside = false;
        for ring in &self.outline {
            if ring_contains(ring, lat, lon) {
                inside = !inside;
            }
        }
        inside
    }
}

fn ring_contains(ring: &[[f64; 2]], lat: f64, lon: f64) -> bool {
    let mut inside = false;
    let count = ring.len();
    let mut j = count - 1;
    for i in 0..count {
        let a = ring[i];
        let b = ring[j];
        let straddles = (a[1] > lat) != (b[1] > lat);
        if straddles {
            let cross = (b[0] - a[0]) * (lat - a[1]) / (b[1] - a[1]) + a[0];
            if lon < cross {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

fn parse_rings(source: &str) -> Vec<Vec<[f64; 2]>> {
    let collection: FeatureCollection = match serde_json::from_str(source) {
        Ok(parsed) => parsed,
        Err(cause) => panic!("embedded geojson corrupt: {cause}"),
    };
    let mut rings = Vec::new();
    for feature in collection.features {
        match feature.geometry {
            Geometry::Polygon(polygon) => rings.extend(polygon),
            Geometry::MultiPolygon(polygons) => {
                for polygon in polygons {
                    rings.extend(polygon);
                }
            }
        }
    }
    rings
}

fn merc(lon: f64, lat: f64) -> (f64, f64) {
    let x = lon.to_radians();
    let y = (std::f64::consts::FRAC_PI_4 + lat.to_radians() / 2.0).tan().ln();
    (x, y)
}

impl CountyMap {
    pub fn load() -> CountyMap {
        let collection: FeatureCollection = match serde_json::from_str(GEOJSON) {
            Ok(parsed) => parsed,
            Err(cause) => panic!("embedded geojson corrupt: {cause}"),
        };
        let mut counties = Vec::with_capacity(collection.features.len());
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        let mut lon_min = f64::MAX;
        let mut lon_max = f64::MIN;
        let mut lat_min = f64::MAX;
        let mut lat_max = f64::MIN;
        for feature in collection.features {
            let name = feature.properties.name;
            let rings = match feature.geometry {
                Geometry::Polygon(rings) => rings,
                Geometry::MultiPolygon(polygons) => polygons.into_iter().flatten().collect(),
            };
            for ring in &rings {
                for point in ring {
                    let (mx, my) = merc(point[0], point[1]);
                    min_x = min_x.min(mx);
                    min_y = min_y.min(my);
                    max_x = max_x.max(mx);
                    max_y = max_y.max(my);
                    lon_min = lon_min.min(point[0]);
                    lon_max = lon_max.max(point[0]);
                    lat_min = lat_min.min(point[1]);
                    lat_max = lat_max.max(point[1]);
                }
            }
            counties.push(County { name, rings });
        }
        CountyMap {
            counties,
            outline: parse_rings(OUTLINE_GEOJSON),
            bounds: Bounds {
                lon_min,
                lon_max,
                lat_min,
                lat_max,
            },
            merc_min: Vec2::new(min_x as f32, min_y as f32),
            merc_max: Vec2::new(max_x as f32, max_y as f32),
        }
    }
}

pub struct Camera {
    pub zoom: f32,
    pub pan: Vec2,
}

impl Camera {
    pub fn new() -> Camera {
        Camera {
            zoom: 1.0,
            pan: Vec2::ZERO,
        }
    }

    fn base_scale(&self, rect: Rect, map: &CountyMap) -> f32 {
        let span = map.merc_max - map.merc_min;
        (rect.width() / span.x).min(rect.height() / span.y) * 0.92
    }

    pub fn project(&self, point: crate::domain::LatLon, rect: Rect, map: &CountyMap) -> Pos2 {
        let (mx, my) = merc(point.lon, point.lat);
        let center = (map.merc_min + map.merc_max) * 0.5;
        let scale = self.base_scale(rect, map) * self.zoom;
        let sx = rect.center().x + (mx as f32 - center.x) * scale + self.pan.x;
        let sy = rect.center().y - (my as f32 - center.y) * scale + self.pan.y;
        Pos2::new(sx, sy)
    }

    pub fn project_lonlat(&self, lon: f64, lat: f64, rect: Rect, map: &CountyMap) -> Pos2 {
        self.project(crate::domain::LatLon { lat, lon }, rect, map)
    }

    pub fn zoom_to(&mut self, factor: f32, cursor: Pos2, rect: Rect, map: &CountyMap) {
        let screen_center = rect.center();
        let base = self.base_scale(rect, map);
        let old_scale = base * self.zoom;
        let new_zoom = (self.zoom * factor).clamp(1.0, 9.0);
        let new_scale = base * new_zoom;
        let anchor_x = (cursor.x - screen_center.x - self.pan.x) / old_scale;
        let anchor_y = (screen_center.y + self.pan.y - cursor.y) / old_scale;
        self.zoom = new_zoom;
        self.pan.x = cursor.x - screen_center.x - anchor_x * new_scale;
        self.pan.y = cursor.y - screen_center.y + anchor_y * new_scale;
    }

    pub fn clamp_pan(&mut self, rect: Rect, map: &CountyMap) {
        let span = map.merc_max - map.merc_min;
        let scale = self.base_scale(rect, map) * self.zoom;
        let half = Vec2::new(span.x * scale * 0.5, span.y * scale * 0.5);
        let limit = half + rect.size() * 0.5;
        self.pan.x = self.pan.x.clamp(-limit.x, limit.x);
        self.pan.y = self.pan.y.clamp(-limit.y, limit.y);
    }
}
