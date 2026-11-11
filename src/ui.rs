use std::time::Duration;

use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use dsp_core::{
    engines::Engine,
    engines::singing_materials::material::MATERIALS,
    parameter::{
        Parameter,
        ParameterChange::{Decrement, Increment},
        UserParameters,
    },
};
use futures::StreamExt;
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Flex, Layout, Rect},
    style::Stylize,
    symbols::border,
    text::Line,
    widgets::{Block, Paragraph},
};

use crate::{SharedEngine, log};

const FRAME_INTERVAL: Duration = Duration::from_millis(33);
const PARAM_WIDTH: u16 = 9;
const PARAM_HEIGHT: u16 = 4;

pub async fn run(engine: &'static SharedEngine) -> std::io::Result<()> {
    let mut terminal = ratatui::init();
    let result = event_loop(&mut terminal, engine).await;
    ratatui::restore();
    result
}

async fn event_loop(
    terminal: &mut DefaultTerminal,
    engine: &'static SharedEngine,
) -> std::io::Result<()> {
    let mut events = EventStream::new();
    let mut ticker = tokio::time::interval(FRAME_INTERVAL);
    let mut selected = 0;

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                let params = get_parameters(engine);
                terminal.draw(|frame| draw(frame, &params, selected))?;
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

fn get_parameters(engine: &SharedEngine) -> Vec<Parameter> {
    let engine = engine.lock().unwrap();
    engine
        .get_parameters()
        .iter()
        .chain(engine.get_envelope_parameters())
        .cloned()
        .collect()
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

fn draw(frame: &mut Frame, params: &[Parameter], selected: usize) {
    let [left, logs] =
        Layout::horizontal([Constraint::Ratio(2, 3), Constraint::Ratio(1, 3)]).areas(frame.area());
    let [top_left, bottom_left] =
        Layout::vertical([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).areas(left);

    render_controls(frame, top_left, params, selected);
    frame.render_widget(Block::bordered(), bottom_left);
    render_logs(frame, logs);
}

fn render_controls(frame: &mut Frame, area: Rect, params: &[Parameter], selected: usize) {
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
            0 => MATERIALS[param.get_value() as usize].name,
            1 => match param.get_value() {
                1.0 => "Sine",
                2.0 => "Saw",
                3.0 => "Sqr",
                4.0 => "Tri",
                _ => "",
            },
            _ => &format!("{:.1}", param.get_value()),
        };

        let text = format!("{}\n↑\n{}\n↓", param.get_name(), value);
        frame.render_widget(Paragraph::new(text).centered().block(cell_block), *cell);
    }
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
