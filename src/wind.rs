use egui::{Color32, Rect, Stroke};
use noise::{NoiseFn, Perlin};

use crate::domain::{LatLon, Station, WindDir};
use crate::geo::{Bounds, Camera, CountyMap};

const COLS: usize = 64;
const ROWS: usize = 40;
const PARTICLES: usize = 640;
const TRAIL: usize = 96;
const SPEED: f64 = 0.085;
const CURL: f32 = 0.42;
const DIFFUSE_PASSES: usize = 20;

struct Flow {
    lat: f64,
    lon: f64,
    vx: f32,
    vy: f32,
}

struct Field {
    bounds: Bounds,
    vx: Vec<f32>,
    vy: Vec<f32>,
    alive: bool,
}

struct Particle {
    lat: f64,
    lon: f64,
    trail: Vec<LatLon>,
    age: f32,
    life: f32,
}

struct Rng {
    state: u32,
}

impl Rng {
    fn new() -> Rng {
        Rng { state: 0x1234_5678 }
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
}

pub struct WindSim {
    field: Field,
    particles: Vec<Particle>,
    perlin: Perlin,
    rng: Rng,
    clock: f64,
}

impl WindSim {
    pub fn new() -> WindSim {
        WindSim {
            field: Field {
                bounds: Bounds {
                    lon_min: 0.0,
                    lon_max: 1.0,
                    lat_min: 0.0,
                    lat_max: 1.0,
                },
                vx: Vec::new(),
                vy: Vec::new(),
                alive: false,
            },
            particles: Vec::new(),
            perlin: Perlin::new(7),
            rng: Rng::new(),
            clock: 0.0,
        }
    }

    pub fn rebuild(&mut self, stations: &[Station], map: &CountyMap) {
        self.field = build_field(stations, map.bounds);
        self.particles.clear();
        for _index in 0..PARTICLES {
            let mut particle = Particle {
                lat: 0.0,
                lon: 0.0,
                trail: Vec::with_capacity(TRAIL),
                age: 0.0,
                life: 1.0,
            };
            respawn(&mut particle, &mut self.rng, map);
            particle.age = self.rng.unit() * particle.life;
            self.particles.push(particle);
        }
    }

    pub fn update(&mut self, dt: f32, map: &CountyMap) {
        if !self.field.alive {
            return;
        }
        self.clock += dt as f64;
        let WindSim {
            field,
            particles,
            perlin,
            rng,
            clock,
        } = self;
        for particle in particles.iter_mut() {
            let (bx, by) = field.sample(particle.lat, particle.lon);
            let (cx, cy) = curl(perlin, particle.lon, particle.lat, *clock);
            let magnitude = (bx * bx + by * by).sqrt();
            let swirl = CURL * (0.3 + magnitude);
            let vx = bx + cx * swirl;
            let vy = by + cy * swirl;

            particle.trail.push(LatLon {
                lat: particle.lat,
                lon: particle.lon,
            });
            if particle.trail.len() > TRAIL {
                particle.trail.remove(0);
            }

            particle.lon += vx as f64 * dt as f64 * SPEED;
            particle.lat += vy as f64 * dt as f64 * SPEED;
            particle.age += dt;
            if particle.age >= particle.life || !map.contains(particle.lat, particle.lon) {
                respawn(particle, rng, map);
            }
        }
    }

    pub fn draw(&self, painter: &egui::Painter, rect: Rect, map: &CountyMap, camera: &Camera) {
        for particle in &self.particles {
            let length = particle.trail.len();
            if length < 2 {
                continue;
            }
            let life_left = 1.0 - (particle.age / particle.life).clamp(0.0, 1.0);
            for index in 1..length {
                let tail = camera.project(particle.trail[index - 1], rect, map);
                let head = camera.project(particle.trail[index], rect, map);
                let along = index as f32 / length as f32;
                let alpha = (along * life_left * 235.0) as u8;
                let color = Color32::from_rgba_unmultiplied(175, 215, 255, alpha);
                painter.line_segment([tail, head], Stroke::new(2.4 + along * 3.2, color));
            }
            let tip = camera.project(
                LatLon {
                    lat: particle.lat,
                    lon: particle.lon,
                },
                rect,
                map,
            );
            let tip_alpha = (life_left * 235.0) as u8;
            painter.circle_filled(tip, 2.8, Color32::from_rgba_unmultiplied(200, 230, 255, tip_alpha));
        }
    }
}

fn build_flows(stations: &[Station]) -> Vec<Flow> {
    let mut out = Vec::new();
    for station in stations {
        match station.wind_dir {
            WindDir::Vector { degrees, speed } => {
                let heading = (degrees + 180.0).to_radians();
                out.push(Flow {
                    lat: station.coord.lat,
                    lon: station.coord.lon,
                    vx: heading.sin() * speed,
                    vy: heading.cos() * speed,
                });
            }
            WindDir::Unknown => {}
        }
    }
    out
}

fn build_field(stations: &[Station], bounds: Bounds) -> Field {
    let mut vx = vec![0.0_f32; COLS * ROWS];
    let mut vy = vec![0.0_f32; COLS * ROWS];
    let flows = build_flows(stations);
    if flows.is_empty() {
        return Field {
            bounds,
            vx,
            vy,
            alive: false,
        };
    }
    for row in 0..ROWS {
        let lat = lerp(bounds.lat_min, bounds.lat_max, (row as f64 + 0.5) / ROWS as f64);
        for col in 0..COLS {
            let lon = lerp(bounds.lon_min, bounds.lon_max, (col as f64 + 0.5) / COLS as f64);
            let (sx, sy) = idw(&flows, lat, lon);
            vx[row * COLS + col] = sx;
            vy[row * COLS + col] = sy;
        }
    }
    for _pass in 0..DIFFUSE_PASSES {
        diffuse(&mut vx);
        diffuse(&mut vy);
    }
    Field {
        bounds,
        vx,
        vy,
        alive: true,
    }
}

fn diffuse(grid: &mut Vec<f32>) {
    let mut next = grid.clone();
    for row in 0..ROWS {
        for col in 0..COLS {
            let left = grid[row * COLS + col.saturating_sub(1)];
            let right = grid[row * COLS + (col + 1).min(COLS - 1)];
            let up = grid[row.saturating_sub(1) * COLS + col];
            let down = grid[(row + 1).min(ROWS - 1) * COLS + col];
            let neighbors = (left + right + up + down) * 0.25;
            next[row * COLS + col] = grid[row * COLS + col] * 0.45 + neighbors * 0.55;
        }
    }
    *grid = next;
}

impl Field {
    fn sample(&self, lat: f64, lon: f64) -> (f32, f32) {
        let gx = ((lon - self.bounds.lon_min) / (self.bounds.lon_max - self.bounds.lon_min) * COLS as f64 - 0.5)
            .clamp(0.0, (COLS - 1) as f64);
        let gy = ((lat - self.bounds.lat_min) / (self.bounds.lat_max - self.bounds.lat_min) * ROWS as f64 - 0.5)
            .clamp(0.0, (ROWS - 1) as f64);
        let x0 = gx.floor() as usize;
        let y0 = gy.floor() as usize;
        let x1 = (x0 + 1).min(COLS - 1);
        let y1 = (y0 + 1).min(ROWS - 1);
        let tx = (gx - x0 as f64) as f32;
        let ty = (gy - y0 as f64) as f32;
        let vx = bilerp(self.vx[y0 * COLS + x0], self.vx[y0 * COLS + x1], self.vx[y1 * COLS + x0], self.vx[y1 * COLS + x1], tx, ty);
        let vy = bilerp(self.vy[y0 * COLS + x0], self.vy[y0 * COLS + x1], self.vy[y1 * COLS + x0], self.vy[y1 * COLS + x1], tx, ty);
        (vx, vy)
    }
}

fn idw(flows: &[Flow], lat: f64, lon: f64) -> (f32, f32) {
    let mut nx = 0.0_f64;
    let mut ny = 0.0_f64;
    let mut total = 0.0_f64;
    for flow in flows {
        let dlat = lat - flow.lat;
        let dlon = lon - flow.lon;
        let distance = dlat * dlat + dlon * dlon + 0.05;
        let weight = 1.0 / distance;
        nx += weight * flow.vx as f64;
        ny += weight * flow.vy as f64;
        total += weight;
    }
    if total <= 0.0 {
        return (0.0, 0.0);
    }
    ((nx / total) as f32, (ny / total) as f32)
}

fn curl(perlin: &Perlin, x: f64, y: f64, clock: f64) -> (f32, f32) {
    let scale = 1.7;
    let epsilon = 0.12;
    let drift = clock * 0.25;
    let up = perlin.get([x * scale, (y + epsilon) * scale, drift]);
    let down = perlin.get([x * scale, (y - epsilon) * scale, drift]);
    let right = perlin.get([(x + epsilon) * scale, y * scale, drift]);
    let left = perlin.get([(x - epsilon) * scale, y * scale, drift]);
    let cx = (up - down) / (2.0 * epsilon);
    let cy = -(right - left) / (2.0 * epsilon);
    (cx as f32, cy as f32)
}

fn respawn(particle: &mut Particle, rng: &mut Rng, map: &CountyMap) {
    let bounds = map.bounds;
    particle.trail.clear();
    for _attempt in 0..8 {
        let lat = bounds.lat_min + (bounds.lat_max - bounds.lat_min) * rng.unit() as f64;
        let lon = bounds.lon_min + (bounds.lon_max - bounds.lon_min) * rng.unit() as f64;
        if map.contains(lat, lon) {
            particle.lat = lat;
            particle.lon = lon;
            particle.age = 0.0;
            particle.life = 1.8 + rng.unit() * 2.6;
            return;
        }
    }
    particle.lat = (bounds.lat_min + bounds.lat_max) * 0.5;
    particle.lon = (bounds.lon_min + bounds.lon_max) * 0.5;
    particle.age = 0.0;
    particle.life = 2.5;
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

fn bilerp(c00: f32, c10: f32, c01: f32, c11: f32, tx: f32, ty: f32) -> f32 {
    let top = c00 + (c10 - c00) * tx;
    let bottom = c01 + (c11 - c01) * tx;
    top + (bottom - top) * ty
}
