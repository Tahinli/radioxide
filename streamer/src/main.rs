use streamer::gui::Streamer;

#[tokio::main]
async fn main() {
    println!("Hello, world!");
    tokio::task::block_in_place(|| {
        iced::program("Streamer GUI", Streamer::update, Streamer::view)
            .centered()
            .window_size((350.0, 400.0))
            .load(Streamer::load_config)
            .antialiasing(true)
            .subscription(Streamer::subscription)
            .exit_on_close_request(false)
            .run()
            .unwrap()
    });
}
