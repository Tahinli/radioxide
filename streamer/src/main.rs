use streamer::gui::Streamer;

#[tokio::main]
async fn main() -> iced::Result {
    println!("Hello, world!");
    iced::program("Streamer GUI", Streamer::update, Streamer::view)
        .centered()
        .window_size((350.0, 400.0))
        .load(Streamer::load_config)
        .run()
}
