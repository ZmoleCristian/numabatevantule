use delaunator::{triangulate, Point};
use egui::{Color32, Mesh, Pos2, Rect, Response, Shape, Stroke};

use crate::color;
use crate::domain::{Alert, Reading, Severity};
use crate::geo::{Camera, County, CountyMap};
use crate::heat::HeatMesh;
use crate::markers::{self, MapPoint};
use crate::theme;
use crate::viewstate::{Datasets, Overlays, Picked, Remote, Selection};
use crate::wind::WindSim;

const SEA_TOP: Color32 = Color32::from_rgb(15, 22, 36);
const SEA_BOTTOM: Color32 = Color32::from_rgb(6, 9, 16);
const LAND: Color32 = Color32::from_rgb(26, 34, 50);
const COAST: Color32 = Color32::from_rgba_premultiplied(150, 168, 198, 205);

pub struct Scene<'a> {
    pub map: &'a CountyMap,
    pub camera: &'a Camera,
    pub data: &'a Datasets,
    pub heat: &'a HeatMesh,
    pub wind: &'a WindSim,
    pub day: usize,
    pub selection: &'a Selection,
    pub overlays: &'a Overlays,
}

pub fn draw(ui: &egui::Ui, response: &Response, rect: Rect, scene: Scene<'_>) -> Picked {
    let painter = ui.painter_at(rect);
    paint_sea(&painter, rect);

    if scene.heat.triangles.is_empty() {
        paint_land(&painter, rect, scene.map, scene.camera);
    } else {
        paint_fill(&painter, rect, scene.map, scene.camera, scene.heat, !scene.overlays.heat);
    }
    paint_outline(&painter, rect, scene.map, scene.camera);

    if scene.overlays.warnings {
        paint_alerts(&painter, rect, scene.map, scene.camera, &scene.data.alerts);
    }
    if scene.overlays.wind {
        scene.wind.draw(&painter, rect, scene.map, scene.camera);
    }

    let points = build_points(scene.camera, rect, scene.map, scene.data, scene.day);
    let selected = selected_name(scene.selection);
    markers::draw(&painter, &points, selected, scene.camera.zoom);

    resolve(&painter, response, &points)
}

fn paint_sea(painter: &egui::Painter, rect: Rect) {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.left_top(), SEA_TOP);
    mesh.colored_vertex(rect.right_top(), SEA_TOP);
    mesh.colored_vertex(rect.right_bottom(), SEA_BOTTOM);
    mesh.colored_vertex(rect.left_bottom(), SEA_BOTTOM);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(Shape::mesh(mesh));
}

fn paint_land(painter: &egui::Painter, rect: Rect, map: &CountyMap, camera: &Camera) {
    for county in &map.counties {
        for ring in &county.rings {
            let mut points = Vec::with_capacity(ring.len());
            for vertex in ring {
                points.push(camera.project_lonlat(vertex[0], vertex[1], rect, map));
            }
            painter.add(Shape::convex_polygon(points, LAND, Stroke::NONE));
        }
    }
}

fn paint_fill(painter: &egui::Painter, rect: Rect, map: &CountyMap, camera: &Camera, heat: &HeatMesh, flat: bool) {
    let mut mesh = Mesh::default();
    mesh.vertices.reserve(heat.triangles.len());
    for vertex in &heat.triangles {
        let tint = if flat { LAND } else { vertex.color };
        mesh.colored_vertex(camera.project(vertex.coord, rect, map), tint);
    }
    let mut start = 0;
    while start + 3 <= heat.triangles.len() {
        mesh.add_triangle(start as u32, start as u32 + 1, start as u32 + 2);
        start += 3;
    }
    painter.add(Shape::mesh(mesh));
}

fn paint_outline(painter: &egui::Painter, rect: Rect, map: &CountyMap, camera: &Camera) {
    for ring in &map.outline {
        let mut points = Vec::with_capacity(ring.len());
        for corner in ring {
            points.push(camera.project_lonlat(corner[0], corner[1], rect, map));
        }
        painter.add(Shape::closed_line(points, Stroke::new(1.8_f32, COAST)));
    }
}

fn paint_alerts(painter: &egui::Painter, rect: Rect, map: &CountyMap, camera: &Camera, remote: &Remote<Vec<Alert>>) {
    let list = match remote {
        Remote::Ready(list) => list,
        Remote::Loading => return,
        Remote::Failed(_cause) => return,
    };
    for county in &map.counties {
        let mut best = Severity::Green;
        let mut hit = false;
        for alert in list {
            for marked in &alert.counties {
                if marked.name == county.name {
                    hit = true;
                    if severity_rank(marked.severity) > severity_rank(best) {
                        best = marked.severity;
                    }
                }
            }
        }
        if !hit {
            continue;
        }
        fill_county(painter, rect, map, camera, county, color::severity(best));
    }
}

fn fill_county(painter: &egui::Painter, rect: Rect, map: &CountyMap, camera: &Camera, county: &County, tint: Color32) {
    let fill = Color32::from_rgba_unmultiplied(tint.r(), tint.g(), tint.b(), 58);
    for ring in &county.rings {
        if ring.len() < 3 {
            continue;
        }
        let mut points = Vec::with_capacity(ring.len());
        for corner in ring {
            points.push(Point {
                x: corner[0],
                y: corner[1],
            });
        }
        let triangulation = triangulate(&points);
        let mut mesh = Mesh::default();
        let mut index = 0u32;
        let mut cursor = 0;
        while cursor + 3 <= triangulation.triangles.len() {
            let a = &points[triangulation.triangles[cursor]];
            let b = &points[triangulation.triangles[cursor + 1]];
            let c = &points[triangulation.triangles[cursor + 2]];
            cursor += 3;
            let center_lat = (a.y + b.y + c.y) / 3.0;
            let center_lon = (a.x + b.x + c.x) / 3.0;
            if !ring_pip(ring, center_lat, center_lon) {
                continue;
            }
            mesh.colored_vertex(camera.project_lonlat(a.x, a.y, rect, map), fill);
            mesh.colored_vertex(camera.project_lonlat(b.x, b.y, rect, map), fill);
            mesh.colored_vertex(camera.project_lonlat(c.x, c.y, rect, map), fill);
            mesh.add_triangle(index, index + 1, index + 2);
            index += 3;
        }
        painter.add(Shape::mesh(mesh));
        let mut edge = Vec::with_capacity(ring.len());
        for corner in ring {
            edge.push(camera.project_lonlat(corner[0], corner[1], rect, map));
        }
        painter.add(Shape::closed_line(edge, Stroke::new(1.6_f32, tint)));
    }
}

fn ring_pip(ring: &[[f64; 2]], lat: f64, lon: f64) -> bool {
    let mut inside = false;
    let count = ring.len();
    let mut j = count - 1;
    for i in 0..count {
        let a = ring[i];
        let b = ring[j];
        let straddles = (a[1] > lat) != (b[1] > lat);
        if straddles {
            let boundary = (b[0] - a[0]) * (lat - a[1]) / (b[1] - a[1]) + a[0];
            if lon < boundary {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

fn severity_rank(level: Severity) -> u8 {
    match level {
        Severity::Green => 0,
        Severity::Yellow => 1,
        Severity::Orange => 2,
        Severity::Red => 3,
    }
}

fn selected_name(selection: &Selection) -> &str {
    match selection {
        Selection::Station(name) => name.as_str(),
        Selection::Nothing => "",
        Selection::Warning(_index) => "",
    }
}

fn build_points(camera: &Camera, rect: Rect, map: &CountyMap, data: &Datasets, day: usize) -> Vec<MapPoint> {
    if day == 0 {
        return station_points(camera, rect, map, data);
    }
    forecast_points(camera, rect, map, data, day)
}

fn station_points(camera: &Camera, rect: Rect, map: &CountyMap, data: &Datasets) -> Vec<MapPoint> {
    let mut out = Vec::new();
    match &data.stations {
        Remote::Ready(list) => {
            for station in list {
                out.push(MapPoint {
                    pos: camera.project(station.coord, rect, map),
                    name: station.name.clone(),
                    reading: station.temp,
                    icon: color::condition_icon(&station.clouds, &station.phenomenon),
                });
            }
        }
        Remote::Loading => {}
        Remote::Failed(_cause) => {}
    }
    out
}

fn forecast_points(camera: &Camera, rect: Rect, map: &CountyMap, data: &Datasets, day: usize) -> Vec<MapPoint> {
    let mut out = Vec::new();
    match &data.forecasts {
        Remote::Ready(list) => {
            for city in list {
                let Some(entry) = city.days.get(day - 1) else {
                    continue;
                };
                out.push(MapPoint {
                    pos: camera.project(city.coord, rect, map),
                    name: city.name.clone(),
                    reading: entry.tmax_value,
                    icon: color::forecast_icon(&entry.description),
                });
            }
        }
        Remote::Loading => {}
        Remote::Failed(_cause) => {}
    }
    out
}

fn resolve(painter: &egui::Painter, response: &Response, points: &[MapPoint]) -> Picked {
    let cursor = match response.hover_pos() {
        Some(cursor) => cursor,
        None => return Picked::Untouched,
    };
    let index = markers::hit_test(points, cursor);
    let on_point = index < points.len();
    if on_point {
        draw_tooltip(painter, cursor, &points[index]);
    }
    if !response.clicked() {
        return Picked::Untouched;
    }
    if on_point {
        return Picked::Hit(points[index].name.clone());
    }
    Picked::Empty
}

fn draw_tooltip(painter: &egui::Painter, cursor: Pos2, point: &MapPoint) {
    let temp = match point.reading {
        Reading::Celsius(value) => format!("{value:.1} °C"),
        Reading::Missing => "indisponibil".to_string(),
    };
    let text = format!("{}\n{}", point.name, temp);
    let galley = painter.layout_no_wrap(text, egui::FontId::proportional(13.0), theme::INK);
    let pad = egui::Vec2::splat(7.0);
    let origin = cursor + egui::Vec2::new(15.0, 15.0);
    let box_rect = Rect::from_min_size(origin, galley.size() + pad * 2.0);
    painter.rect_filled(box_rect, egui::CornerRadius::same(6), Color32::from_black_alpha(235));
    painter.galley(origin + pad, galley, theme::INK);
}
