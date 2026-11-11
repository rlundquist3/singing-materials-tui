use std::time::Duration;

use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind, KeyModifiers};
use futures::StreamExt;
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout, Rect},
    text::Line,
    widgets::{Block, Paragraph},
};

use crate::log;

const FRAME_INTERVAL: Duration = Duration::from_millis(33);

pub async fn run() -> std::io::Result<()> {
    let mut terminal = ratatui::init();
    let result = event_loop(&mut terminal).await;
    ratatui::restore();
    result
}

async fn event_loop(terminal: &mut DefaultTerminal) -> std::io::Result<()> {
    let mut events = EventStream::new();
    let mut ticker = tokio::time::interval(FRAME_INTERVAL);

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                terminal.draw(draw)?;
            }
            event = events.next() => match event {
                Some(Ok(Event::Key(key))) if key.kind == KeyEventKind::Press => {
                    let ctrl_c = key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL);
                    if ctrl_c || matches!(key.code, KeyCode::Char('q') | KeyCode::Esc) {
                        return Ok(());
                    }
                }
                Some(Ok(_)) => {}
                Some(Err(err)) => return Err(err),
                None => return Ok(()),
            },
        }
    }
}

fn draw(frame: &mut Frame) {
    let [left, logs] =
        Layout::horizontal([Constraint::Ratio(2, 3), Constraint::Ratio(1, 3)]).areas(frame.area());
    let [top_left, bottom_left] =
        Layout::vertical([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).areas(left);

    frame.render_widget(Block::bordered(), top_left);
    frame.render_widget(Block::bordered(), bottom_left);
    render_logs(frame, logs);
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
