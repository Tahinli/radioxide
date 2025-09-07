use dioxus::prelude::*;
use front::components::{coin_status_renderer, listen_renderer, server_status_renderer};

fn main() {
    println!("Hello, world!");
    wasm_logger::init(wasm_logger::Config::default());
    launch(app);
}

fn app() -> Element {
    let server_address = "https://tahinli.com.tr:2323".to_string();
    rsx! {
        page_base {}
        listen_renderer {}
        coin_status_renderer {server_address:server_address.clone()}
        server_status_renderer {server_address:server_address.clone()}
    }
}

fn page_base() -> Element {
    rsx! {
        h1 {
            "Radioxide"
        }
        div {
            div {
                class: "flex items-center",
                span {
                }
            }

        }
    }
}
