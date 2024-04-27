use streamer::gui::Streamer;

#[tokio::main]
async fn main() -> iced::Result {
    println!("Hello, world!");
    iced::program("Streamer GUI", Streamer::update, Streamer::view)
        .centered()
        .window_size((250.0, 250.0))
        .load(Streamer::load_config)
        .run()
}
