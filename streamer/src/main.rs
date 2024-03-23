use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::HeapRb;



#[tokio::main]
async fn main() {
    println!("Hello, world!");
    let host = cpal::default_host();
    let input_device = host.default_input_device().unwrap();
    let output_device = host.default_output_device().unwrap();

    println!("Input Device: {}", input_device.name().unwrap());
    println!("Output Device: {}", output_device.name().unwrap());

    let config:cpal::StreamConfig = input_device.default_input_config().unwrap().into();

    let latency_frames = 0.1*config.sample_rate.0 as f32;
    let latency_samples = latency_frames as usize * config.channels as usize;

    let ring = HeapRb::<f32>::new(latency_samples*2);
    let (mut producer, mut consumer) = ring.split();

    for _ in 0..latency_samples {
        producer.push(0.0).unwrap();
    }

    let input_data_fn = move |data: &[f32], _:&cpal::InputCallbackInfo| {
        let mut output_fell_behind = false;
        for &sample in data {
            if producer.push(sample).is_err() {
                output_fell_behind = true;
            }
        }
        if output_fell_behind {
            eprintln!("Output consumed all, increase delay");
        }
    };

    let output_data_fn = move |data: &mut [f32], _:&cpal::OutputCallbackInfo| {
        let mut input_fell_behind = false;
        for sample in data {
            *sample = match consumer.pop() {
                Some(s) => s,
                None => {
                    input_fell_behind = true;
                    0.0
                }
            };
        }
        if input_fell_behind {
            eprintln!("Input can't be fast enough, increase delay");
        }
    };

    let input_stream = input_device.build_input_stream(&config, input_data_fn, err_fn, None).unwrap();
    let output_stream = output_device.build_output_stream(&config, output_data_fn, err_fn, None).unwrap();

    println!("STREAMIN");
    input_stream.play().unwrap();
    output_stream.play().unwrap();

    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    //std::thread::sleep(std::time::Duration::from_secs(10));
    println!("DONE I HOPE");
}

fn err_fn(err: cpal::StreamError) {
    eprintln!("Something Happened: {}", err);
}
