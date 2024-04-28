use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use tokio::sync::broadcast::{Receiver, Sender};

pub async fn record(
    sound_stream_sender: Sender<f32>,
    mut base_to_recording: Receiver<bool>,
    recording_to_base: Sender<bool>,
) {
    let _ = recording_to_base.send(true);

    let host = cpal::default_host();
    let input_device = host.default_input_device().unwrap();

    let config: cpal::StreamConfig = input_device.default_input_config().unwrap().into();

    let input_data_fn = move |data: &[f32], _: &cpal::InputCallbackInfo| {
        for &sample in data {
            if sound_stream_sender.receiver_count() > 0 {
                match sound_stream_sender.send(sample) {
                    Ok(_) => {}
                    Err(_) => {}
                }
            }
        }
    };
    let input_stream = input_device
        .build_input_stream(&config, input_data_fn, err_fn, None)
        .unwrap();
    input_stream.play().unwrap();
    println!("Recording Started");
    tokio::spawn(let_the_base_know(recording_to_base.clone()));
    while let Err(_) = base_to_recording.try_recv() {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    input_stream.pause().unwrap();
    tokio::spawn(let_the_base_know(recording_to_base.clone()));
    println!("Recording Stopped");
}
fn err_fn(err: cpal::StreamError) {
    eprintln!("Something Happened: {}", err);
}

async fn let_the_base_know(recording_to_base: Sender<bool>) {
    let _ = recording_to_base.send(true);
}
