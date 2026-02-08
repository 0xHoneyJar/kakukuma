mod app;
mod canvas;
mod cell;
mod export;
mod history;
mod input;
mod palette;
mod project;
mod symmetry;
mod tools;
mod ui;

use std::io;
use std::time::Duration;

use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use app::App;
use input::CanvasArea;

fn main() -> io::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Setup panic handler to restore terminal
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(panic_info);
    }));

    let result = run(&mut terminal);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    let mut app = App::new();
    let mut canvas_area = CanvasArea {
        left: 0,
        top: 0,
        width: 0,
        height: 0,
    };

    // Load file from command-line argument if provided
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        app.load_project(&args[1]);
    }

    // Check for autosave recovery on startup (only if no file was loaded)
    if app.project_path.is_none() {
        app.check_recovery();
    }

    while app.running {
        // Render
        terminal.draw(|f| {
            canvas_area = ui::render(f, &app);
        })?;

        // Poll for events with timeout for status message ticking
        if event::poll(Duration::from_millis(100))? {
            let event = event::read()?;
            input::handle_event(&mut app, event, &canvas_area);
        }

        // Tick status message timer
        app.tick_status();

        // Tick auto-save timer
        app.tick_auto_save();
    }

    Ok(())
}
