pub mod balancer_blueprint;
pub mod balancer_layout;
pub mod model;

#[cfg(target_arch = "wasm32")]
mod app;

#[cfg(target_arch = "wasm32")]
mod balancer_app;

#[cfg(target_arch = "wasm32")]
mod balancer_canvas;

#[cfg(target_arch = "wasm32")]
mod graph_viewport;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    let balancers = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.body())
        .is_some_and(|body| body.has_attribute("data-balancers"));
    if balancers {
        yew::Renderer::<balancer_app::Balancers>::new().render();
    } else {
        yew::Renderer::<app::App>::new().render();
    }
}
