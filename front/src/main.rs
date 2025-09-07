use dioxus::prelude::*;


fn main() {
    println!("Hello, world!");
    launch(app);
}

fn app() -> Element {
    rsx!{
        h1 {
            "Radioxide"
        }
        div {
            audio{ autoplay:true, controls:true, muted:false, src:"https://playerservices.streamtheworld.com/api/livestream-redirect/METRO_FM128AAC.aac?/;stream.mp3" }
        }
    }
}