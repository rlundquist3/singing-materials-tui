mod audio;
mod log;
mod midi;
mod ui;

use dsp_core::engines::singing_materials::SingingMaterials;
use std::sync::Mutex;

use crate::{
    audio::audio_handler,
    midi::{midi_input_task, tasks::midi_buffer_handler},
};

pub type SharedEngine = Mutex<SingingMaterials<'static>>;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Lives for the whole program; shared by the audio callback and MIDI handler
    let engine: &'static SharedEngine = Box::leak(Box::new(Mutex::new(SingingMaterials::new())));

    tokio::spawn(audio_handler(engine));
    tokio::spawn(midi_input_task());
    tokio::spawn(midi_buffer_handler(engine));

    // The UI runs on the main task; the program exits when it returns
    ui::run().await
}
