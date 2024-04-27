use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use tokio::sync::broadcast::{Receiver, Sender};

pub async fn record(
    sound_stream_producer: Sender<f32>,
    mut stop_recording_consumer: Receiver<bool>,
) {
    let host = cpal::default_host();
    let input_device = host.default_input_device().unwrap();

    let config: cpal::StreamConfig = input_device.default_input_config().unwrap().into();

    let input_data_fn = move |data: &[f32], _: &cpal::InputCallbackInfo| {
        for &sample in data {
            match sound_stream_producer.send(sample) {
                Ok(_) => {}
                Err(_) => {}
            }
        }
    };

    let input_stream = input_device
        .build_input_stream(&config, input_data_fn, err_fn, None)
        .unwrap();

    input_stream.play().unwrap();
    println!("Recording Started");
    while let Err(_) = stop_recording_consumer.try_recv() {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    input_stream.pause().unwrap();
    println!("Recording Stopped");
}
fn err_fn(err: cpal::StreamError) {
    eprintln!("Something Happened: {}", err);
}
