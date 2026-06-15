mod alertbar;
mod app;
mod calendar;
mod color;
mod config;
mod detail;
mod domain;
mod dto;
mod error;
mod fetch;
mod fmt;
mod forecastbar;
mod geo;
mod heat;
mod history;
mod hud;
mod markers;
mod render;
mod theme;
mod viewstate;
mod wind;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1240.0, 840.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Meteo România",
        options,
        Box::new(|cc| Ok(Box::new(app::MeteoApp::new(cc)))),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    let options = eframe::WebOptions::default();
    wasm_bindgen_futures::spawn_local(async move {
        let document = match web_sys::window() {
            Some(window) => match window.document() {
                Some(document) => document,
                None => panic!("browser has no document"),
            },
            None => panic!("browser has no window"),
        };
        let element = match document.get_element_by_id("the_canvas_id") {
            Some(element) => element,
            None => panic!("missing canvas element id the_canvas_id"),
        };
        let canvas = match element.dyn_into::<web_sys::HtmlCanvasElement>() {
            Ok(canvas) => canvas,
            Err(other) => panic!("element is not a canvas: {other:?}"),
        };
        let runner = eframe::WebRunner::new();
        let outcome = runner
            .start(canvas, options, Box::new(|cc| Ok(Box::new(app::MeteoApp::new(cc)))))
            .await;
        match outcome {
            Ok(started) => started,
            Err(cause) => panic!("failed to start eframe: {cause:?}"),
        }
    });
}
