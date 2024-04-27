use streamer::gui::Streamer;

#[tokio::main]
async fn main() -> iced::Result{
    println!("Hello, world!");
    iced::program("Streamer GUI", Streamer::update, Streamer::view)
    .load(Streamer::load_config)
    .run()
}
