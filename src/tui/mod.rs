pub mod app;
mod input;
mod tasks;
mod ui;

use std::io::{ Stdout, stdout };

use crossterm::{ event::{ DisableBracketedPaste, EnableBracketedPaste, Event }, execute };
use tokio::sync::mpsc::UnboundedSender;

use crate::error::{ Context, Result };

pub async fn run() -> Result<()> {
    let mut terminal = ratatui::try_init().context("инициализация терминала")?;
    let mut out = stdout();
    enable_paste(&mut out)?;

    let result = app::run(&mut terminal).await;

    let _ = execute!(out, DisableBracketedPaste);
    ratatui::restore();
    result
}

fn enable_paste(out: &mut Stdout) -> Result<()> {
    execute!(out, EnableBracketedPaste).context("включение вставки из буфера обмена")
}

pub fn spawn_input_reader(sender: UnboundedSender<Event>) {
    std::thread::spawn(move || {
        while let Ok(event) = crossterm::event::read() {
            if sender.send(event).is_err() {
                break;
            }
        }
    });
}
