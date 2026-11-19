use core::fmt::Write;
use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use dsp_core::{
    engines::{Engine, singing_materials},
    parameter::{
        Parameter,
        ParameterChange::{Decrement, Increment},
        UserParameters,
    },
};
use futures::StreamExt;
use heapless::String;
use ratatui::prelude::Widget;
use ratatui::symbols;
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Style, Stylize},
    symbols::border,
    text::Line,
    widgets::{Axis, Block, Chart, Dataset, GraphType, Paragraph},
};
use std::time::Duration;

use crate::{SAMPLE_HISTORY_SIZE, SampleHistory, SharedEngine, log, utils::get_mag_spectrum};

const FRAME_INTERVAL: Duration = Duration::from_millis(33);
const PARAM_WIDTH: u16 = 9;
const PARAM_HEIGHT: u16 = 4;

pub async fn run(
    engine: &'static SharedEngine,
    sample_history: &'static SampleHistory,
) -> std::io::Result<()> {
    let mut terminal = ratatui::init();
    let result = event_loop(&mut terminal, engine, sample_history).await;
    ratatui::restore();
    result
}

async fn event_loop(
    terminal: &mut DefaultTerminal,
    engine: &'static SharedEngine,
    sample_history: &'static SampleHistory,
) -> std::io::Result<()> {
    let mut events = EventStream::new();
    let mut ticker = tokio::time::interval(FRAME_INTERVAL);
    let mut selected = 0;

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                let engine_state = get_engine_state(engine);
                let history = get_sample_history(sample_history);
                terminal.draw(|frame| draw(frame, engine_state, selected, history))?;
            }
            event = events.next() => match event {
                Some(Ok(Event::Key(key))) if key.kind == KeyEventKind::Press => {
                    if is_quit(key) {
                        return Ok(());
                    }
                    handle_control_key(engine, key.code, &mut selected);
                }
                Some(Ok(_)) => {}
                Some(Err(err)) => return Err(err),
                None => return Ok(()),
            },
        }
    }
}

fn is_quit(key: KeyEvent) -> bool {
    let ctrl_c = key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL);
    ctrl_c || matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
}

#[derive(Clone)]
struct EngineState {
    material: String<32>,
    temp: String<8>,
    species: String<8>,
    parameters: Vec<Parameter>,
    sample_rate: u32,
}

fn get_engine_state(engine: &SharedEngine) -> EngineState {
    let mut material: String<32> = String::new();
    let mut temp: String<8> = String::new();
    let mut species: String<8> = String::new();

    let engine = engine.lock().unwrap();

    write!(material, "{}", engine.get_material_name()).unwrap();
    write!(temp, "{}", engine.get_current_temp()).unwrap();
    write!(species, "{}", engine.get_current_species()).unwrap();

    EngineState {
        material,
        temp,
        species,
        parameters: engine
            .get_parameters()
            .iter()
            .chain(engine.get_envelope_parameters())
            .cloned()
            .collect(),
        sample_rate: engine.get_sample_rate(),
    }
}

fn get_sample_history(sample_history: &SampleHistory) -> Vec<f32> {
    let history = sample_history.lock().unwrap();
    history.clone().into_iter().collect()
}

fn handle_control_key(engine: &SharedEngine, code: KeyCode, selected: &mut usize) {
    let mut engine = engine.lock().unwrap();
    let count = engine.get_parameters().len() + engine.get_envelope_parameters().len();

    match code {
        KeyCode::Left => *selected = selected.saturating_sub(1),
        KeyCode::Right => *selected = (*selected + 1).min(count.saturating_sub(1)),
        KeyCode::Up => engine.update_parameter(*selected, Increment),
        KeyCode::Down => engine.update_parameter(*selected, Decrement),
        _ => {}
    }
}

fn draw(frame: &mut Frame, engine_state: EngineState, selected: usize, sample_history: Vec<f32>) {
    let [left, logs] =
        Layout::horizontal([Constraint::Ratio(2, 3), Constraint::Ratio(1, 3)]).areas(frame.area());
    let [top_left, bottom_left] =
        Layout::vertical([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).areas(left);

    let sample_rate = engine_state.sample_rate;
    render_controls(frame, top_left, engine_state, selected);
    render_visualizations(frame, bottom_left, sample_history, sample_rate);
    frame.render_widget(Block::bordered(), bottom_left);
    render_logs(frame, logs);
}

fn render_controls(frame: &mut Frame, area: Rect, engine_state: EngineState, selected: usize) {
    let params = &engine_state.parameters;

    let title = Line::from("  Singing Materials  ").bold();
    let instructions = Line::from(vec![
        " Select Param ".into(),
        "<←/→> ".bold(),
        " Update Param ".into(),
        "<↑/↓> ".bold(),
        " Quit ".into(),
        "<Ctrl+C> ".red().bold(),
    ]);
    let block = Block::bordered()
        .title(title.centered())
        .title_bottom(instructions.right_aligned())
        .border_set(border::THICK);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let [row] = Layout::vertical([Constraint::Length(PARAM_HEIGHT)])
        .flex(Flex::Center)
        .areas(inner);
    let cells = Layout::horizontal(params.iter().enumerate().map(|(i, _)| {
        Constraint::Length(match i {
            0 => 36,
            _ => PARAM_WIDTH,
        })
    }))
    .spacing(1)
    .flex(Flex::Center)
    .split(row);

    for (i, (param, cell)) in params.iter().zip(cells.iter()).enumerate() {
        let cell_block = if i == selected {
            Block::new().on_cyan()
        } else {
            Block::new()
        };

        let value = match i {
            0 => &engine_state.material,
            1 => match param.get_value() {
                1.0 => "Sine",
                2.0 => "Saw",
                3.0 => "Sqr",
                4.0 => "Tri",
                _ => "",
            },
            2 => &engine_state.species,
            _ => &format!("{:.1}", param.get_value()),
        };

        let text = format!("{}\n↑\n{}\n↓", param.get_name(), value);
        frame.render_widget(Paragraph::new(text).centered().block(cell_block), *cell);
    }
}

fn render_visualizations(
    frame: &mut Frame,
    area: Rect,
    sample_history: Vec<f32>,
    sample_rate: u32,
) {
    let visualization_subsections =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .spacing(1)
            .split(area);

    // waveform
    let index_sample_pairs: Vec<(f64, f64)> = sample_history
        .iter()
        .enumerate()
        .map(|(i, v)| (i as f64, *v as f64))
        .collect();

    let waveform_dataset = Dataset::default()
        .marker(symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Color::Red))
        .data(&index_sample_pairs);

    let waveform_chart = Chart::new(vec![waveform_dataset])
        .block(Block::bordered())
        .x_axis(
            Axis::default()
                .bounds([0.0, SAMPLE_HISTORY_SIZE as f64])
                .style(Style::default().fg(Color::Gray)),
        )
        .y_axis(
            Axis::default()
                .bounds([-1.0, 1.0])
                .style(Style::default().fg(Color::Gray)),
        );
    frame.render_widget(waveform_chart, visualization_subsections[0]);

    // magnitude spectrum (logarithmic x-axis)
    // TODO: add x-axis labels (non-trivial to do nice octave-spaced labels in ratatui; leaving for now)
    let min_freq = sample_rate as f64 / SAMPLE_HISTORY_SIZE as f64;
    let max_freq = sample_rate as f64 / 2.0;

    let samples = sample_history.iter().map(|s| *s).collect();
    let freq_mag_pairs: Vec<(f64, f64)> = get_mag_spectrum(&samples, sample_rate)
        .into_iter()
        .filter(|(freq, _)| *freq >= min_freq)
        .map(|(freq, mag)| (freq.log10(), mag))
        .collect();
    let mag_dataset = Dataset::default()
        .marker(symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Color::Yellow))
        .data(&freq_mag_pairs);

    let mag_chart = Chart::new(vec![mag_dataset])
        .block(Block::bordered())
        .x_axis(
            Axis::default()
                .bounds([min_freq.log10(), max_freq.log10()])
                .style(Style::default().fg(Color::Gray))
                .title("freq (Hz)"),
        )
        .y_axis(
            Axis::default()
                .bounds([0.0, 30.0])
                .style(Style::default().fg(Color::Gray))
                .title("magnitude"),
        );
    frame.render_widget(mag_chart, visualization_subsections[1]);
}

fn render_logs(frame: &mut Frame, area: Rect) {
    let block = Block::bordered().title(" Logs ");
    let visible = block.inner(area).height as usize;

    let lines = log::snapshot();
    let start = lines.len().saturating_sub(visible);
    let lines: Vec<Line> = lines[start..]
        .iter()
        .map(|l| Line::raw(l.as_str()))
        .collect();

    frame.render_widget(Paragraph::new(lines).block(block), area);
}
