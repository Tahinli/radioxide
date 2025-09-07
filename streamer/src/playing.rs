use std::{
    fs::File,
    sync::{Arc, Mutex},
    time::Duration,
};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};
use symphonia::core::{
    audio::{AudioBufferRef, Signal},
    codecs::{Decoder, DecoderOptions, CODEC_TYPE_NULL},
    formats::{FormatOptions, FormatReader},
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
    should_decode_now_sender: Sender<bool>,
    playing_to_base_sender: Sender<Player>,
    mut base_to_playing_receiver: Receiver<Player>,
    audio_volume: Arc<Mutex<f32>>,
) {
    let host = cpal::default_host();
    let output_device = host.default_output_device().unwrap();
    let output_device_config: cpal::StreamConfig =
        output_device.default_output_config().unwrap().into();

    let output_device_sample_rate = output_device_config.sample_rate.0;

    let mut decoded_to_playing_receiver = decoded_to_playing_sender.subscribe();
    let should_decode_now_receiver = should_decode_now_sender.subscribe();
    let audio_process_task = tokio::spawn(process_audio(
        output_device_sample_rate,
        file,
        decoded_to_playing_sender,
        should_decode_now_receiver,
    ));
    while decoded_to_playing_receiver.is_empty() {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    let output_data_fn = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
        for sample in data {
            if should_decode_now_sender.receiver_count() > 0 {
                if should_decode_now_sender.len() == 0 {
                    if crate::AUDIO_BUFFER_SIZE / 2 > decoded_to_playing_receiver.len() {
                        if let Err(err_val) = should_decode_now_sender.send(true) {
                            eprintln!("Error: Decode Order | {}", err_val);
                        }
                    }
                }
            }

            if decoded_to_playing_receiver.len() > 0 {
                let single = match decoded_to_playing_receiver.blocking_recv() {
                    Ok(single) => single * *audio_volume.lock().unwrap(),
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
                Player::Stop => {
                    if !audio_process_task.is_finished() {
                        audio_process_task.abort();
                    }
                    break;
                }
            },
            Err(_) => {
                if !audio_process_task.is_finished() {
                    audio_process_task.abort();
                }
                break;
            }
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

fn decode_audio(
    format: &mut Box<dyn FormatReader>,
    track_id: u32,
    decoder: &mut Box<dyn Decoder>,
) -> Option<(Vec<f64>, Vec<f64>)> {
    let mut audio_decoded_left = vec![];
    let mut audio_decoded_right = vec![];
    let packet = match format.next_packet() {
        Ok(packet) => packet,
        Err(_) => return None,
    };

    while !format.metadata().is_latest() {
        format.metadata().pop();
    }

    if packet.track_id() != track_id {
        return None;
    }

    if let Ok(decoded) = decoder.decode(&packet) {
        if let AudioBufferRef::F32(buf) = decoded {
            for (left, right) in buf.chan(0).iter().zip(buf.chan(1).iter()) {
                audio_decoded_left.push(*left as f64);
                audio_decoded_right.push(*right as f64);
            }
        }
    }
    Some((audio_decoded_left, audio_decoded_right))
}

fn resample_audio(
    audio_decoded_left: Vec<f64>,
    audio_decoded_right: Vec<f64>,
    resampler: &mut SincFixedIn<f64>,
) -> (Vec<f64>, Vec<f64>) {
    let audio_decoded_channels_combined = vec![audio_decoded_left, audio_decoded_right];
    let audio_resampled = resampler
        .process(&audio_decoded_channels_combined, None)
        .unwrap();

    (audio_resampled[0].clone(), audio_resampled[1].clone())
}

async fn process_audio(
    output_device_sample_rate: u32,
    file: File,
    decoded_to_playing_sender: Sender<f32>,
    mut should_decode_now_receiver: Receiver<bool>,
) {
    let media_source_stream = MediaSourceStream::new(Box::new(file), Default::default());

    let hint = Hint::new();

    let metadata_options = MetadataOptions::default();
    let format_options = FormatOptions::default();

    let mut probed = symphonia::default::get_probe().format(
        &hint,
        media_source_stream,
        &format_options,
        &metadata_options,
    );

    match probed {
        Ok(probed_safe) => probed = Ok(probed_safe),
        Err(_) => return,
    }

    let mut format = probed.unwrap().format;

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

    let params = SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: 0.95,
        interpolation: SincInterpolationType::Linear,
        oversampling_factor: 128,
        window: WindowFunction::BlackmanHarris2,
    };

    let (chunk_size, audio_decoded_left, audio_decoded_right) =
        match decode_audio(&mut format, track_id, &mut decoder) {
            Some((audio_decoded_left_channel, audio_decoded_right_channel)) => (
                audio_decoded_left_channel.len(),
                audio_decoded_left_channel,
                audio_decoded_right_channel,
            ),
            None => return,
        };
    let mut resampler = SincFixedIn::<f64>::new(
        output_device_sample_rate as f64 / audio_sample_rate as f64,
        2.0,
        params,
        chunk_size,
        2,
    )
    .unwrap();

    let (audio_resampled_left, audio_resampled_right) =
        resample_audio(audio_decoded_left, audio_decoded_right, &mut resampler);

    for (single_left, single_right) in audio_resampled_left.iter().zip(&audio_resampled_right) {
        let _ = decoded_to_playing_sender.send(*single_left as f32);
        let _ = decoded_to_playing_sender.send(*single_right as f32);
    }

    while let Ok(true) = should_decode_now_receiver.recv().await {
        let (mut audio_decoded_left, mut audio_decoded_right) = (vec![], vec![]);

        match decode_audio(&mut format, track_id, &mut decoder) {
            Some((audio_decoded_left_channel, audio_decoded_right_channel)) => {
                for (single_left, single_right) in audio_decoded_left_channel
                    .iter()
                    .zip(&audio_decoded_right_channel)
                {
                    audio_decoded_left.push(*single_left);
                    audio_decoded_right.push(*single_right);
                }
            }
            None => break,
        };
        let (audio_resampled_left, audio_resampled_right) =
            resample_audio(audio_decoded_left, audio_decoded_right, &mut resampler);

        for (single_left, single_right) in audio_resampled_left.iter().zip(&audio_resampled_right) {
            let _ = decoded_to_playing_sender.send(*single_left as f32);
            let _ = decoded_to_playing_sender.send(*single_right as f32);
        }
    }
}
