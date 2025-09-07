use streamer::gui::Streamer;

#[tokio::main]
async fn main() {
    println!("Hello, world!");
    tokio::task::block_in_place(|| {
        iced::program("Streamer GUI", Streamer::update, Streamer::view)
            .centered()
            .window_size((350.0, 650.0))
            .load(Streamer::load_config)
            .load(Streamer::list_files)
            .antialiasing(true)
            .subscription(Streamer::subscription)
            .exit_on_close_request(false)
            .run()
            .unwrap()
    });
}
