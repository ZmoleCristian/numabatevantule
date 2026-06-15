use egui::Color32;
use spade::{ConstrainedDelaunayTriangulation, InsertionError, Point2, Triangulation};

use crate::color;
use crate::domain::{LatLon, Reading, Station};
use crate::error::MeteoError;
use crate::geo::CountyMap;

const INTERIOR_STEP: f64 = 0.10;

pub struct HeatVertex {
    pub coord: LatLon,
    pub color: Color32,
}

pub struct HeatMesh {
    pub triangles: Vec<HeatVertex>,
}

impl HeatMesh {
    pub fn empty() -> HeatMesh {
        HeatMesh {
            triangles: Vec::new(),
        }
    }
}

struct Sample {
    lat: f64,
    lon: f64,
    temp: f32,
}

pub fn build(stations: &[Station], map: &CountyMap) -> Result<HeatMesh, MeteoError> {
    let samples = collect(stations);
    if samples.is_empty() {
        return Ok(HeatMesh::empty());
    }
    let faces = triangulate_country(map)?;
    let mut triangles = Vec::with_capacity(faces.len() * 3);
    for face in faces {
        triangles.push(vertex(&samples, face[0][0], face[0][1]));
        triangles.push(vertex(&samples, face[1][0], face[1][1]));
        triangles.push(vertex(&samples, face[2][0], face[2][1]));
    }
    Ok(HeatMesh { triangles })
}

fn triangulate_country(map: &CountyMap) -> Result<Vec<[[f64; 2]; 3]>, MeteoError> {
    let mut cdt: ConstrainedDelaunayTriangulation<Point2<f64>> = ConstrainedDelaunayTriangulation::new();
    for ring in &map.outline {
        let verts = ring_points(ring);
        if verts.len() < 3 {
            continue;
        }
        cdt.add_constraint_edges(verts, true).map_err(geometry_error)?;
    }
    let bounds = map.bounds;
    let mut lat = bounds.lat_min + INTERIOR_STEP * 0.5;
    while lat < bounds.lat_max {
        let mut lon = bounds.lon_min + INTERIOR_STEP * 0.5;
        while lon < bounds.lon_max {
            if map.contains(lat, lon) {
                cdt.insert(Point2::new(lon, lat)).map_err(geometry_error)?;
            }
            lon += INTERIOR_STEP;
        }
        lat += INTERIOR_STEP;
    }
    let mut faces = Vec::new();
    for face in cdt.inner_faces() {
        let corners = face.vertices();
        let p0 = corners[0].position();
        let p1 = corners[1].position();
        let p2 = corners[2].position();
        let center_lat = (p0.y + p1.y + p2.y) / 3.0;
        let center_lon = (p0.x + p1.x + p2.x) / 3.0;
        if map.contains(center_lat, center_lon) {
            faces.push([[p0.x, p0.y], [p1.x, p1.y], [p2.x, p2.y]]);
        }
    }
    Ok(faces)
}

fn ring_points(ring: &[[f64; 2]]) -> Vec<Point2<f64>> {
    let count = ring.len();
    let mut taken = count;
    if count > 1 {
        let first = ring[0];
        let last = ring[count - 1];
        if (first[0] - last[0]).abs() < 1e-9 && (first[1] - last[1]).abs() < 1e-9 {
            taken = count - 1;
        }
    }
    let mut out = Vec::with_capacity(taken);
    for corner in &ring[..taken] {
        out.push(Point2::new(corner[0], corner[1]));
    }
    out
}

fn geometry_error(cause: InsertionError) -> MeteoError {
    MeteoError::BadGeometry(format!("{cause:?}"))
}

fn collect(stations: &[Station]) -> Vec<Sample> {
    let mut out = Vec::new();
    for station in stations {
        match station.temp {
            Reading::Celsius(temp) => out.push(Sample {
                lat: station.coord.lat,
                lon: station.coord.lon,
                temp,
            }),
            Reading::Missing => {}
        }
    }
    out
}

fn vertex(samples: &[Sample], lon: f64, lat: f64) -> HeatVertex {
    let temp = interpolate(samples, lat, lon);
    let base = color::temperature(temp);
    HeatVertex {
        coord: LatLon { lat, lon },
        color: Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), 215),
    }
}

fn interpolate(samples: &[Sample], lat: f64, lon: f64) -> f32 {
    let mut weighted = 0.0_f64;
    let mut total = 0.0_f64;
    for sample in samples {
        let dlat = lat - sample.lat;
        let dlon = (lon - sample.lon) * 0.72;
        let distance = dlat * dlat + dlon * dlon + 0.0015;
        let weight = 1.0 / (distance * distance);
        weighted += weight * sample.temp as f64;
        total += weight;
    }
    (weighted / total) as f32
}
