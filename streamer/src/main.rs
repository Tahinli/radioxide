use streamer::gui::Streamer;

#[tokio::main]
async fn main() {
    println!("Hello, world!");

    iced::application("Streamer GUI", Streamer::update, Streamer::view)
        .centered()
        .window_size((350.0, 650.0))
        .antialiasing(true)
        .subscription(Streamer::subscription)
        .exit_on_close_request(false)
        .run_with(|| Streamer::new_with_load())
        .unwrap()

    // tokio::task::spawn_blocking(|| {
    //     iced::application("Streamer GUI", Streamer::update, Streamer::view)
    //         .centered()
    //         .window_size((350.0, 650.0))
    //         .antialiasing(true)
    //         .subscription(Streamer::subscription)
    //         .exit_on_close_request(false)
    //         .run_with(|| Streamer::new_with_load())
    //         .unwrap()
    // });
    // tokio::task::block_in_place(|| {
    //     iced::application("Streamer GUI", Streamer::update, Streamer::view)
    //         .centered()
    //         .window_size((350.0, 650.0))
    //         .antialiasing(true)
    //         .subscription(Streamer::subscription)
    //         .exit_on_close_request(false)
    //         .run_with(|| Streamer::new_with_load())
    //         .unwrap()
    // });
}
