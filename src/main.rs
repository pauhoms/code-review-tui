// Punto de entrada: wiring de terminal + loop de la TUI.
// Toda la lógica vive en la lib; este archivo solo hace I/O de terminal.

use std::io;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use reviewv2::app::{App, Outcome};
use reviewv2::diff;

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run() -> io::Result<()> {
    let cwd = std::env::current_dir()?;

    let diff = diff::collect(&cwd).map_err(|e| io::Error::other(e.to_string()))?;

    let date = today_date_string();
    let mut app = App::with_output(diff, cwd.clone(), date);

    // Preparar terminal.
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = event_loop(&mut terminal, &mut app);

    // Siempre restaurar el terminal.
    crossterm::terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    match result? {
        Outcome::Saved(path) => println!("Reporte escrito en: {}", path.display()),
        Outcome::Quit => {}
        Outcome::Continue => {}
    }

    Ok(())
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<Outcome> {
    loop {
        terminal.draw(|frame| app.render(frame))?;

        if event::poll(Duration::from_millis(200))?
            && let Event::Key(key) = event::read()?
        {
            // Solo procesar eventos Press para evitar duplicados en Windows.
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match app.handle_key(key) {
                Outcome::Quit => return Ok(Outcome::Quit),
                Outcome::Saved(p) => return Ok(Outcome::Saved(p)),
                Outcome::Continue => {}
            }
        }
    }
}

/// Calcula la fecha de hoy en formato YYYY-MM-DD sin dependencias externas.
/// Usa el algoritmo civil_from_days (Howard Hinnant).
fn today_date_string() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Días desde epoch Unix (1970-01-01).
    let days = (secs / 86400) as i64;

    // Algoritmo de Howard Hinnant: civil_from_days.
    // http://howardhinnant.github.io/date_algorithms.html
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{y:04}-{m:02}-{d:02}")
}
