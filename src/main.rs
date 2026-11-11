mod audio;
mod log;
mod midi;
mod ui;
mod utils;

use dsp_core::engines::singing_materials::SingingMaterials;
use std::{collections::VecDeque, sync::Mutex};

use crate::{
    audio::audio_handler,
    midi::{midi_input_task, tasks::midi_buffer_handler},
};

pub type SharedEngine = Mutex<SingingMaterials<'static>>;
pub type SampleHistory = Mutex<VecDeque<f32>>;

pub const SAMPLE_HISTORY_SIZE: usize = 512;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let engine: &'static SharedEngine = Box::leak(Box::new(Mutex::new(SingingMaterials::new())));
    let sample_history: &'static SampleHistory = Box::leak(Box::new(Mutex::new(VecDeque::from(
        vec![0.0; SAMPLE_HISTORY_SIZE],
    ))));

    tokio::spawn(audio_handler(engine, sample_history));
    tokio::spawn(midi_input_task());
    tokio::spawn(midi_buffer_handler(engine));

    ui::run(engine, sample_history).await
}
