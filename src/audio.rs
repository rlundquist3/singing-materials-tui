use cpal::{
    BufferSize, StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use dsp_core::{DEFAULT_SAMPLE_RATE, engines::singing_materials::SingingMaterials};

use crate::{SharedEngine, log};

pub async fn audio_handler(engine: &'static SharedEngine) {
    let Some(device) = cpal::default_host().default_output_device() else {
        log::push("No default audio output device found");
        return;
    };

    let default_config = device.default_output_config();
    let sample_rate = match default_config {
        Ok(config) => config.sample_rate(),
        Err(err) => {
            log::push(format!("error reading output config: {err}"));
            log::push(format!(
                "attempting to initialize at {DEFAULT_SAMPLE_RATE}Hz"
            ));
            DEFAULT_SAMPLE_RATE
        }
    };

    let config = StreamConfig {
        channels: 2,
        sample_rate,
        buffer_size: BufferSize::Default,
    };
    let channels = config.channels as usize;

    let stream = device.build_output_stream(
        config,
        move |output: &mut [f32], _| {
            let mut engine = engine.lock().unwrap();
            audio_output(&mut engine, output, channels);
        },
        |err| log::push(format!("audio stream error: {err}")),
        None,
    );

    let stream = match stream {
        Ok(stream) => stream,
        Err(err) => {
            log::push(format!(
                "Audio device does not support {sample_rate}Hz f32 output: {err}"
            ));
            return;
        }
    };

    if let Err(err) = stream.play() {
        log::push(format!("Error starting audio stream: {err}"));
        return;
    }
    engine.lock().unwrap().set_sample_rate(sample_rate);
    log::push(format!("Audio initialized at {sample_rate}Hz"));

    // Hold the stream open for the life of the task
    std::future::pending::<()>().await;
}

fn audio_output(engine: &mut SingingMaterials, output: &mut [f32], channels: usize) {
    output.chunks_mut(channels).for_each(|frame| {
        frame.fill(engine.next().unwrap_or(0.0));
    });
}
