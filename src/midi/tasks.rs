use dsp_core::{
    engines::Engine,
    midi::{MidiMessage, get_pitch_bend_value},
};

use crate::{SharedEngine, log, midi::MIDI_BUFFER};

const NOTE_OFF: u8 = 0x80;
const NOTE_ON: u8 = 0x90;
const PITCH_BEND: u8 = 0xE0;

pub async fn midi_buffer_handler(engine: &'static SharedEngine) {
    let mut receiver = MIDI_BUFFER.receiver();

    loop {
        let Some(message) = receiver.recv().await else {
            log::push("MIDI buffer closed");
            return;
        };

        let MidiMessage(status, note, velocity) = message;

        // Mask off the channel so input on any channel is accepted
        match status & 0xF0 {
            NOTE_OFF => note_off(engine, note),
            NOTE_ON if velocity == 0 => note_off(engine, note),
            NOTE_ON => note_on(engine, note, velocity),
            PITCH_BEND => engine
                .lock()
                .unwrap()
                .set_pitch_bend(get_pitch_bend_value(message)),
            _ => log::push(format!("unsupported status {status:#04x}")),
        };
    }
}

fn note_on(engine: &'static SharedEngine, note: u8, velocity: u8) {
    log::push(format!("note on {note} vel {velocity}"));
    engine.lock().unwrap().note_on(note);
}

fn note_off(engine: &'static SharedEngine, note: u8) {
    log::push(format!("note off {note}"));
    engine.lock().unwrap().note_off(note);
}
