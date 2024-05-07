use std::fs::File;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};
use symphonia::core::{
    audio::{AudioBufferRef, Signal},
    codecs::{DecoderOptions, CODEC_TYPE_NULL},
    formats::FormatOptions,
    io::MediaSourceStream,
    meta::MetadataOptions,
    probe::Hint,
};
use tokio::{
    sync::broadcast::{Receiver, Sender},
    task,
};

use crate::gui::Player;

pub async fn play(
    audio_stream_sender: Sender<f32>,
    file: File,
    decoded_to_playing_sender: Sender<f32>,
    playing_to_base_sender: Sender<Player>,
    mut base_to_playing_receiver: Receiver<Player>,
) {
    let host = cpal::default_host();
    let output_device = host.default_output_device().unwrap();
    let output_device_config: cpal::StreamConfig =
        output_device.default_output_config().unwrap().into();

    let output_device_sample_rate = output_device_config.sample_rate.0;

    let (mut audio_resampled_left, mut audio_resampled_right) =
        decode_audio(output_device_sample_rate, file);

    let mut decoded_to_playing_receiver = decoded_to_playing_sender.subscribe();
    for _ in 0..audio_resampled_left.clone().len() {
        decoded_to_playing_sender
            .send(audio_resampled_left.pop().unwrap() as f32)
            .unwrap();
        decoded_to_playing_sender
            .send(audio_resampled_right.pop().unwrap() as f32)
            .unwrap();
    }

    let output_data_fn = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
        for sample in data {
            if decoded_to_playing_receiver.len() > 0 {
                let single = match decoded_to_playing_receiver.blocking_recv() {
                    Ok(single) => single,
                    Err(_) => 0.0,
                };
                if audio_stream_sender.receiver_count() > 0 {
                    let _ = audio_stream_sender.send(single);
                }
                *sample = single;
            }
        }
    };

    let output_stream = output_device
        .build_output_stream(&output_device_config, output_data_fn, err_fn, None)
        .unwrap();

    output_stream.play().unwrap();
    tokio::spawn(let_the_base_know(
        playing_to_base_sender.clone(),
        Player::Play,
    ));

    task::block_in_place(|| loop {
        match base_to_playing_receiver.blocking_recv() {
            Ok(state) => match state {
                Player::Play => {
                    output_stream.play().unwrap();
                    tokio::spawn(let_the_base_know(
                        playing_to_base_sender.clone(),
                        Player::Play,
                    ));
                }
                Player::Pause => match output_stream.pause() {
                    Ok(_) => {
                        tokio::spawn(let_the_base_know(
                            playing_to_base_sender.clone(),
                            Player::Pause,
                        ));
                    }
                    //todo when pause error, do software level stop
                    Err(_) => todo!(),
                },
                Player::Stop => break,
            },
            Err(_) => break,
        }
    });
    drop(output_stream);
    tokio::spawn(let_the_base_know(playing_to_base_sender, Player::Stop));
}

fn err_fn(err: cpal::StreamError) {
    eprintln!("Something Happened: {}", err);
}
async fn let_the_base_know(playing_to_base_sender: Sender<Player>, action: Player) {
    let _ = playing_to_base_sender.send(action);
}
fn decode_audio(output_device_sample_rate: u32, file: File) -> (Vec<f64>, Vec<f64>) {
    let mut audio_decoded_left = vec![];
    let mut audio_decoded_right = vec![];
    let media_source_stream = MediaSourceStream::new(Box::new(file), Default::default());

    let hint = Hint::new();

    let metadata_options = MetadataOptions::default();
    let format_options = FormatOptions::default();

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            media_source_stream,
            &format_options,
            &metadata_options,
        )
        .unwrap();

    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .unwrap();

    let audio_sample_rate = track.codec_params.sample_rate.unwrap();

    DecoderOptions::default();
    let decoder_options = DecoderOptions::default();
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &decoder_options)
        .unwrap();

    let track_id = track.id;

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(_) => {
                break;
            }
        };

        while !format.metadata().is_latest() {
            format.metadata().pop();
        }

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => match decoded {
                AudioBufferRef::F32(buf) => {
                    for (left, right) in buf.chan(0).iter().zip(buf.chan(1).iter()) {
                        audio_decoded_left.push(*left as f64);
                        audio_decoded_right.push(*right as f64);
                    }
                }
                _ => {}
            },
            Err(_) => {
                //eprintln!("Error: Sample Decode | {}", err_val);
                println!("End ?");
            }
        }
    }

    let params = SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: 0.95,
        interpolation: SincInterpolationType::Linear,
        oversampling_factor: 256,
        window: WindowFunction::BlackmanHarris2,
    };
    let mut resampler = SincFixedIn::<f64>::new(
        output_device_sample_rate as f64 / audio_sample_rate as f64,
        2.0,
        params,
        audio_decoded_left.len(),
        2,
    )
    .unwrap();

    let audio_decoded_channes_combined =
        vec![audio_decoded_left.clone(), audio_decoded_right.clone()];
    let audio_resampled = resampler
        .process(&audio_decoded_channes_combined, None)
        .unwrap();

    let mut audio_resampled_left = vec![];
    let mut audio_resampled_right = vec![];

    for sample in &audio_resampled[0] {
        audio_resampled_left.push(*sample);
    }

    for sample in &audio_resampled[1] {
        audio_resampled_right.push(*sample);
    }

    audio_resampled_left.reverse();
    audio_resampled_right.reverse();

    (audio_resampled_left, audio_resampled_right)
}
