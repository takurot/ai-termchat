use crate::avatar::loader::AvatarManager;
use crate::config::Config;
use crate::state::State;
use crate::ui;
use crate::util::Result;

use crossterm::terminal;
use crossterm::ExecutableCommand;

use tui::backend::CrosstermBackend;
use tui::Terminal;

use std::io::Write;

pub struct Renderer<W: Write> {
    terminal: Terminal<CrosstermBackend<W>>,
}

impl<W: Write> Renderer<W> {
    pub fn new(mut out: W) -> Result<Renderer<W>> {
        terminal::enable_raw_mode()?;
        out.execute(terminal::EnterAlternateScreen)?;

        Ok(Renderer { terminal: Terminal::new(CrosstermBackend::new(out))? })
    }

    pub fn render(
        &mut self,
        state: &mut State,
        config: &Config,
        avatar_manager: &AvatarManager,
    ) -> Result<()> {
        self.terminal.draw(|frame| {
            ui::draw(frame, state, frame.size(), &config.theme, &config.language, avatar_manager)
        })?;
        Ok(())
    }
}

impl<W: Write> Drop for Renderer<W> {
    fn drop(&mut self) {
        self.terminal
            .backend_mut()
            .execute(terminal::LeaveAlternateScreen)
            .expect("Could not execute to stdout");
        terminal::disable_raw_mode().expect("Terminal doesn't support to disable raw mode");
        if std::thread::panicking() {
            eprintln!(
                "triadchat panicked, redirect stderr to a file to inspect the failure, for example: triadchat 2> triadchat.log",
            );
        }
    }
}
