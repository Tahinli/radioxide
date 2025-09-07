use streamer::{gui::Streamer, WINDOW_SIZE_HEIGHT, WINDOW_SIZE_WIDTH};

#[tokio::main]
async fn main() {
    println!("Hello, world!");

    iced::application("Streamer GUI", Streamer::update, Streamer::view)
        .centered()
        .window_size((WINDOW_SIZE_WIDTH as f32, WINDOW_SIZE_HEIGHT as f32))
        .antialiasing(true)
        .subscription(Streamer::subscription)
        .exit_on_close_request(false)
        .run_with(|| Streamer::new_with_load())
        .unwrap()
}
