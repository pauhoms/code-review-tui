// TUI ratatui — App, eventos y render (Fase 3).
// Orquesta diff (F1) + review (F2) sin mutar los modelos.

use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line as TuiLine, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Wrap};

use crate::diff::{Diff, LineKind};
use crate::review::{Review, Side, Verdict};

// ---------------------------------------------------------------------------
// Tipos públicos de estado
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Files,
    Diff,
    Thread,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Navigate,
    EditComment,
    RangeSelect,
    Final,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Split,
    Unified,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Continue,
    Quit,
    Saved(PathBuf),
}

// ---------------------------------------------------------------------------
// Estado interno del editor / selección de rango
// ---------------------------------------------------------------------------

/// Tipos de comentario que se está redactando.
#[derive(Debug, Clone)]
enum CommentTarget {
    Line {
        file: String,
        side: Side,
        line: u32,
    },
    Range {
        file: String,
        side: Side,
        start: u32,
        end: u32,
    },
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

pub struct App {
    diff: Diff,
    review: Review,
    focus: Focus,
    mode: Mode,
    view_mode: ViewMode,
    selected_file: usize,
    cursor_line: usize,
    /// Inicio del rango visual (índice de línea aplanada).
    range_start: usize,
    /// Editor de comentario: buffer de texto.
    comment_buf: String,
    /// A qué línea/rango apunta el comentario en edición.
    comment_target: Option<CommentTarget>,
    /// Buffer para el comentario general (pantalla final).
    general_buf: String,
    /// Veredicto elegido en la pantalla final.
    final_verdict: Verdict,
    /// Índice del comentario seleccionado en la pantalla final.
    selected_comment: usize,
    /// Directorio de salida del reporte.
    out_dir: PathBuf,
    /// Fecha inyectada para el nombre del reporte.
    date: String,
}

impl App {
    /// Constructor simple: out_dir = ".", fecha placeholder.
    pub fn new(diff: Diff) -> App {
        App::with_output(diff, PathBuf::from("."), "0000-00-00".to_owned())
    }

    /// Constructor con salida controlada (directorio y fecha inyectados).
    pub fn with_output(diff: Diff, out_dir: PathBuf, date: String) -> App {
        App {
            diff,
            review: Review::new(),
            focus: Focus::Files,
            mode: Mode::Navigate,
            view_mode: ViewMode::Split,
            selected_file: 0,
            cursor_line: 0,
            range_start: 0,
            comment_buf: String::new(),
            comment_target: None,
            general_buf: String::new(),
            final_verdict: Verdict::Undecided,
            selected_comment: 0,
            out_dir,
            date,
        }
    }

    // --- Getters del contrato público ---

    pub fn focus(&self) -> Focus {
        self.focus
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn view_mode(&self) -> ViewMode {
        self.view_mode
    }

    pub fn selected_file_index(&self) -> usize {
        self.selected_file
    }

    pub fn cursor_line(&self) -> usize {
        self.cursor_line
    }

    pub fn review(&self) -> &Review {
        &self.review
    }

    pub fn selected_comment_index(&self) -> usize {
        self.selected_comment
    }

    // --- Manejador de eventos ---

    pub fn handle_key(&mut self, key: KeyEvent) -> Outcome {
        match self.mode {
            Mode::Navigate => self.handle_navigate(key),
            Mode::EditComment => self.handle_edit_comment(key),
            Mode::RangeSelect => self.handle_range_select(key),
            Mode::Final => self.handle_final(key),
        }
    }

    // ---------------------------------------------------------------------------
    // Handlers por modo
    // ---------------------------------------------------------------------------

    fn handle_navigate(&mut self, key: KeyEvent) -> Outcome {
        // Sin modificadores (o con solo shift para mayúsculas)
        if key.modifiers == KeyModifiers::NONE || key.modifiers == KeyModifiers::SHIFT {
            match key.code {
                KeyCode::Char('q') => return Outcome::Quit,
                KeyCode::Char('1') => self.focus = Focus::Files,
                KeyCode::Char('2') => self.focus = Focus::Diff,
                KeyCode::Char('t') => {
                    self.view_mode = match self.view_mode {
                        ViewMode::Split => ViewMode::Unified,
                        ViewMode::Unified => ViewMode::Split,
                    };
                }
                KeyCode::Char('g') => self.mode = Mode::Final,
                KeyCode::Char('j') => self.nav_down(),
                KeyCode::Char('k') => self.nav_up(),
                KeyCode::Char('c') => self.start_line_comment(),
                KeyCode::Char('v') => {
                    self.range_start = self.cursor_line;
                    self.mode = Mode::RangeSelect;
                }
                KeyCode::Tab => self.cycle_focus(true),
                KeyCode::BackTab => self.cycle_focus(false),
                KeyCode::Enter => self.open_thread(),
                KeyCode::Esc if self.focus == Focus::Thread => {
                    self.focus = Focus::Diff;
                }
                _ => {}
            }
        }
        Outcome::Continue
    }

    fn handle_edit_comment(&mut self, key: KeyEvent) -> Outcome {
        if key.modifiers == KeyModifiers::CONTROL
            && let KeyCode::Char('s') = key.code
        {
            self.save_comment();
            return Outcome::Continue;
        }
        if key.modifiers == KeyModifiers::NONE {
            match key.code {
                KeyCode::Esc => {
                    self.comment_buf.clear();
                    self.comment_target = None;
                    self.mode = Mode::Navigate;
                }
                KeyCode::Char(c) => {
                    self.comment_buf.push(c);
                }
                _ => {}
            }
        }
        Outcome::Continue
    }

    fn handle_range_select(&mut self, key: KeyEvent) -> Outcome {
        if key.modifiers == KeyModifiers::NONE {
            match key.code {
                KeyCode::Esc => {
                    self.mode = Mode::Navigate;
                }
                KeyCode::Char('j') => {
                    let max = self.flat_lines_count().saturating_sub(1);
                    if self.cursor_line < max {
                        self.cursor_line += 1;
                    }
                }
                KeyCode::Char('k') if self.cursor_line > self.range_start => {
                    self.cursor_line -= 1;
                }
                KeyCode::Char('c') => {
                    self.start_range_comment();
                }
                _ => {}
            }
        }
        Outcome::Continue
    }

    fn handle_final(&mut self, key: KeyEvent) -> Outcome {
        if key.modifiers == KeyModifiers::CONTROL
            && let KeyCode::Char('s') = key.code
        {
            return self.finalize_and_save();
        }
        if key.modifiers == KeyModifiers::NONE {
            match key.code {
                KeyCode::Esc => {
                    self.mode = Mode::Navigate;
                }
                KeyCode::Left => {
                    self.final_verdict = Verdict::Lgtm;
                }
                KeyCode::Right => {
                    self.final_verdict = Verdict::Ko;
                }
                KeyCode::Down => {
                    let n = self.review.comments().len();
                    if n > 0 {
                        self.selected_comment = (self.selected_comment + 1).min(n - 1);
                    }
                }
                KeyCode::Up => {
                    self.selected_comment = self.selected_comment.saturating_sub(1);
                }
                KeyCode::Enter => self.jump_to_comment(),
                KeyCode::Char(c) => {
                    self.general_buf.push(c);
                }
                _ => {}
            }
        }
        Outcome::Continue
    }

    // ---------------------------------------------------------------------------
    // Helpers de acción de foco / hilo
    // ---------------------------------------------------------------------------

    /// Cicla el foco entre Files y Diff (Thread no entra en el ciclo de Tab;
    /// con dos paneles forward/backward son simétricas).
    fn cycle_focus(&mut self, _forward: bool) {
        self.focus = match self.focus {
            Focus::Files => Focus::Diff,
            Focus::Diff | Focus::Thread => Focus::Files,
        };
    }

    /// Abre el panel Thread si la línea bajo el cursor tiene comentario anclado.
    fn open_thread(&mut self) {
        if self.focus != Focus::Diff {
            return;
        }
        if let Some(line) = self.flat_line(self.cursor_line) {
            let (side, lineno) = Self::anchor_of(line);
            let file_str = if self.diff.files.is_empty() {
                String::new()
            } else {
                self.diff.files[self.selected_file]
                    .path
                    .display()
                    .to_string()
            };
            let has = self.review.comments().iter().any(|c| {
                c.file == file_str
                    && c.side == side
                    && lineno >= c.start_line
                    && lineno <= c.end_line
            });
            if has {
                self.focus = Focus::Thread;
            }
        }
    }

    /// Desde la pantalla final, salta al hilo del comentario seleccionado.
    fn jump_to_comment(&mut self) {
        let comments = self.review.comments();
        if comments.is_empty() {
            return;
        }
        let idx = self.selected_comment.min(comments.len() - 1);
        let comment_file = comments[idx].file.clone();
        let comment_side = comments[idx].side.clone();
        let comment_start = comments[idx].start_line;

        // Buscar el índice de archivo que coincide.
        let file_idx = self
            .diff
            .files
            .iter()
            .position(|fd| fd.path.display().to_string() == comment_file)
            .unwrap_or(0);

        // Buscar el índice aplanado cuyo anchor sea (comment_side, comment_start).
        let flat_idx = self.find_flat_index(file_idx, &comment_side, comment_start);

        self.mode = Mode::Navigate;
        self.focus = Focus::Thread;
        self.selected_file = file_idx;
        self.cursor_line = flat_idx;
    }

    /// Recorre las líneas aplanadas del archivo dado buscando el índice cuyo
    /// anchor coincide con (side, lineno). Devuelve 0 si no se encuentra.
    fn find_flat_index(&self, file_idx: usize, side: &crate::review::Side, lineno: u32) -> usize {
        let Some(fd) = self.diff.files.get(file_idx) else {
            return 0;
        };
        let mut idx = 0usize;
        for hunk in &fd.hunks {
            for line in &hunk.lines {
                let (line_side, line_no) = Self::anchor_of(line);
                if line_side == *side && line_no == lineno {
                    return idx;
                }
                idx += 1;
            }
        }
        0
    }

    // ---------------------------------------------------------------------------
    // Navegación
    // ---------------------------------------------------------------------------

    fn nav_down(&mut self) {
        match self.focus {
            Focus::Files => {
                let max = self.diff.files.len().saturating_sub(1);
                if self.selected_file < max {
                    self.selected_file += 1;
                    self.cursor_line = 0;
                }
            }
            Focus::Diff => {
                let max = self.flat_lines_count().saturating_sub(1);
                if self.cursor_line < max {
                    self.cursor_line += 1;
                }
            }
            Focus::Thread => {}
        }
    }

    fn nav_up(&mut self) {
        match self.focus {
            Focus::Files => {
                if self.selected_file > 0 {
                    self.selected_file -= 1;
                    self.cursor_line = 0;
                }
            }
            Focus::Diff => {
                if self.cursor_line > 0 {
                    self.cursor_line -= 1;
                }
            }
            Focus::Thread => {}
        }
    }

    // ---------------------------------------------------------------------------
    // Helpers de líneas aplanadas
    // ---------------------------------------------------------------------------

    /// Número total de líneas aplanadas del archivo seleccionado.
    fn flat_lines_count(&self) -> usize {
        if self.diff.files.is_empty() {
            return 0;
        }
        let fd = &self.diff.files[self.selected_file];
        fd.hunks.iter().map(|h| h.lines.len()).sum()
    }

    /// Obtiene la línea aplanada en el índice dado del archivo seleccionado.
    fn flat_line(&self, idx: usize) -> Option<&crate::diff::Line> {
        if self.diff.files.is_empty() {
            return None;
        }
        let fd = &self.diff.files[self.selected_file];
        let mut remaining = idx;
        for hunk in &fd.hunks {
            if remaining < hunk.lines.len() {
                return Some(&hunk.lines[remaining]);
            }
            remaining -= hunk.lines.len();
        }
        None
    }

    /// Dada una línea aplanada, calcula (side, line_number) según la regla de anclaje.
    fn anchor_of(line: &crate::diff::Line) -> (Side, u32) {
        match line.kind {
            LineKind::Removed => (Side::Old, line.old_lineno.unwrap_or(0)),
            _ => (Side::New, line.new_lineno.unwrap_or(0)),
        }
    }

    // ---------------------------------------------------------------------------
    // Inicio de edición de comentarios
    // ---------------------------------------------------------------------------

    fn start_line_comment(&mut self) {
        if let Some(line) = self.flat_line(self.cursor_line) {
            let (side, lineno) = Self::anchor_of(line);
            let file = if self.diff.files.is_empty() {
                String::new()
            } else {
                self.diff.files[self.selected_file]
                    .path
                    .display()
                    .to_string()
            };
            self.comment_target = Some(CommentTarget::Line {
                file,
                side,
                line: lineno,
            });
            self.comment_buf.clear();
            self.mode = Mode::EditComment;
        }
    }

    fn start_range_comment(&mut self) {
        if self.diff.files.is_empty() {
            return;
        }
        // Los índices de cursor aplanados: range_start..=cursor_line (o viceversa).
        let (lo, hi) = if self.range_start <= self.cursor_line {
            (self.range_start, self.cursor_line)
        } else {
            (self.cursor_line, self.range_start)
        };

        let line_lo = self.flat_line(lo);
        let line_hi = self.flat_line(hi);

        if let (Some(ll), Some(lh)) = (line_lo, line_hi) {
            let (side_lo, num_lo) = Self::anchor_of(ll);
            let (_side_hi, num_hi) = Self::anchor_of(lh);
            let file = self.diff.files[self.selected_file]
                .path
                .display()
                .to_string();
            let (start, end) = if num_lo <= num_hi {
                (num_lo, num_hi)
            } else {
                (num_hi, num_lo)
            };
            self.comment_target = Some(CommentTarget::Range {
                file,
                side: side_lo,
                start,
                end,
            });
            self.comment_buf.clear();
            self.mode = Mode::EditComment;
        }
    }

    // ---------------------------------------------------------------------------
    // Guardado de comentarios
    // ---------------------------------------------------------------------------

    fn save_comment(&mut self) {
        if let Some(target) = self.comment_target.take() {
            let body = self.comment_buf.clone();
            match target {
                CommentTarget::Line { file, side, line } => {
                    self.review.add_line_comment(&file, side, line, &body);
                }
                CommentTarget::Range {
                    file,
                    side,
                    start,
                    end,
                } => {
                    self.review
                        .add_range_comment(&file, side, start, end, &body);
                }
            }
        }
        self.comment_buf.clear();
        self.mode = Mode::Navigate;
    }

    fn finalize_and_save(&mut self) -> Outcome {
        self.review.set_general(&self.general_buf);
        self.review.set_verdict(self.final_verdict.clone());
        let filename = crate::review::report_filename(&self.date);
        let path = self.out_dir.join(&filename);
        let md = crate::review::to_markdown(&self.review);
        if let Err(e) = std::fs::write(&path, &md) {
            // En caso de error de I/O: no podemos retornar error aquí sin cambiar
            // la firma pública. Registramos el fallo silenciosamente y devolvemos
            // Quit para no dejar la TUI colgada. En la práctica main maneja el
            // terminal.
            eprintln!("Error escribiendo el reporte: {e}");
            return Outcome::Quit;
        }
        Outcome::Saved(path)
    }

    // ---------------------------------------------------------------------------
    // Render
    // ---------------------------------------------------------------------------

    pub fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        if self.diff.files.is_empty() {
            render_empty(frame, area);
            return;
        }

        match self.mode {
            Mode::Final => self.render_final_screen(frame, area),
            _ => self.render_main(frame, area),
        }
    }

    // ---------------------------------------------------------------------------
    // Render — pantalla principal
    // ---------------------------------------------------------------------------

    fn render_main(&self, frame: &mut Frame, area: Rect) {
        // Panel inferior según el contexto: el hilo [3] si hay foco en él, o el
        // editor de comentario mientras se redacta. La barra de estado va abajo.
        let show_thread = self.focus == Focus::Thread;
        let show_editor = self.mode == Mode::EditComment;
        let panel_height: u16 = if show_thread {
            6
        } else if show_editor {
            5
        } else {
            0
        };

        let (main_area, panel_area, bar_area) = if panel_height > 0 {
            let vert = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(3),
                    Constraint::Length(panel_height),
                    Constraint::Length(1),
                ])
                .split(area);
            (vert[0], Some(vert[1]), vert[2])
        } else {
            let vert = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(3), Constraint::Length(1)])
                .split(area);
            (vert[0], None, vert[1])
        };

        // Layout horizontal: panel FILES ~20%, panel DIFF resto.
        let horiz = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
            .split(main_area);

        let files_area = horiz[0];
        let diff_area = horiz[1];

        self.render_files_panel(frame, files_area);
        self.render_diff_panel(frame, diff_area);

        if let Some(pa) = panel_area {
            if show_thread {
                self.render_thread_panel(frame, pa);
            } else {
                self.render_comment_editor(frame, pa);
            }
        }

        self.render_status_bar(frame, bar_area);
    }

    /// Rango de índices aplanados seleccionado mientras se está en modo rango.
    fn active_range(&self) -> Option<(usize, usize)> {
        if self.mode == Mode::RangeSelect {
            let lo = self.range_start.min(self.cursor_line);
            let hi = self.range_start.max(self.cursor_line);
            Some((lo, hi))
        } else {
            None
        }
    }

    /// Caja del editor de comentario: anclaje, texto en curso y ayuda de teclas.
    fn render_comment_editor(&self, frame: &mut Frame, area: Rect) {
        let anchor = match &self.comment_target {
            Some(CommentTarget::Line { file, line, .. }) => format!("{file}:{line}"),
            Some(CommentTarget::Range {
                file, start, end, ..
            }) => format!("{file}:{start}-{end}"),
            None => String::new(),
        };

        let block = Block::default()
            .title(format!("✎ Comentario · {anchor}"))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let lines = vec![
            TuiLine::raw(format!("{}▏", self.comment_buf)),
            TuiLine::styled(
                "Ctrl+S guardar · Esc cancelar",
                Style::default().fg(Color::DarkGray),
            ),
        ];
        let para = Paragraph::new(lines).wrap(Wrap { trim: false });
        frame.render_widget(para, inner);
    }

    /// Barra inferior de atajos según el modo; en rango muestra cuántas líneas
    /// llevás seleccionadas.
    fn render_status_bar(&self, frame: &mut Frame, area: Rect) {
        let text = match self.mode {
            Mode::Navigate => {
                "1/2 panel · j/k mover · c comentar · v rango · t split · g final · q salir"
                    .to_owned()
            }
            Mode::EditComment => "Ctrl+S guardar · Esc cancelar".to_owned(),
            Mode::RangeSelect => {
                let n = self.active_range().map_or(0, |(lo, hi)| hi - lo + 1);
                format!("RANGO: {n} líneas · j/k extender · c comentar · Esc cancelar")
            }
            Mode::Final => "← LGTM · → KO · Ctrl+S guardar · Esc volver".to_owned(),
        };
        let bar = Paragraph::new(text).style(Style::default().fg(Color::DarkGray));
        frame.render_widget(bar, area);
    }

    fn render_thread_panel(&self, frame: &mut Frame, area: Rect) {
        let file_str = if self.diff.files.is_empty() {
            String::new()
        } else {
            self.diff.files[self.selected_file]
                .path
                .display()
                .to_string()
        };

        // Recopilar los comentarios de la línea bajo el cursor.
        let comments_on_line: Vec<&crate::review::Comment> =
            if let Some(line) = self.flat_line(self.cursor_line) {
                let (side, lineno) = Self::anchor_of(line);
                self.review
                    .comments()
                    .iter()
                    .filter(|c| {
                        c.file == file_str
                            && c.side == side
                            && lineno >= c.start_line
                            && lineno <= c.end_line
                    })
                    .collect()
            } else {
                Vec::new()
            };

        let anchor_label = if let Some(line) = self.flat_line(self.cursor_line) {
            let (_side, lineno) = Self::anchor_of(line);
            format!("{file_str}:{lineno}")
        } else {
            file_str.clone()
        };

        let title = format!("[3] 💬 {anchor_label}");
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let lines: Vec<TuiLine> = comments_on_line
            .iter()
            .map(|c| TuiLine::raw(c.body.as_str()))
            .collect();
        let para = Paragraph::new(lines).wrap(Wrap { trim: false });
        frame.render_widget(para, inner);
    }

    fn render_files_panel(&self, frame: &mut Frame, area: Rect) {
        let focused = self.focus == Focus::Files;
        let border_style = if focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };
        let border_type = if focused {
            BorderType::Double
        } else {
            BorderType::Plain
        };

        let block = Block::default()
            .title("[1] FILES")
            .borders(Borders::ALL)
            .border_style(border_style)
            .border_type(border_type);

        let items: Vec<ListItem> = self
            .diff
            .files
            .iter()
            .enumerate()
            .map(|(i, fd)| {
                let status_char = match fd.status {
                    crate::diff::FileStatus::Added => 'A',
                    crate::diff::FileStatus::Modified => 'M',
                    crate::diff::FileStatus::Deleted => 'D',
                    crate::diff::FileStatus::Renamed => 'R',
                };
                let name = fd
                    .path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| fd.path.display().to_string());
                let marker = if i == self.selected_file { ">" } else { " " };
                let text = format!(
                    "{marker}{status_char} {name} +{} −{}",
                    fd.additions, fd.deletions
                );
                let style = if i == self.selected_file {
                    Style::default().add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(text).style(style)
            })
            .collect();

        let list = List::new(items).block(block);
        frame.render_widget(list, area);
    }

    fn render_diff_panel(&self, frame: &mut Frame, area: Rect) {
        match self.view_mode {
            ViewMode::Split => self.render_diff_split(frame, area),
            ViewMode::Unified => self.render_diff_unified(frame, area),
        }
    }

    fn render_diff_split(&self, frame: &mut Frame, area: Rect) {
        let focused = self.focus == Focus::Diff;
        let border_style = if focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };
        let border_type = if focused {
            BorderType::Double
        } else {
            BorderType::Plain
        };

        let block = Block::default()
            .title("[2] DIFF")
            .borders(Borders::ALL)
            .border_style(border_style)
            .border_type(border_type);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // El área interna se divide en dos mitades: OLD (izquierda) y NEW (derecha).
        let half = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(inner);

        let old_area = half[0];
        let new_area = half[1];

        // Encabezados OLD / NEW.
        let old_header = Block::default().title("OLD").borders(Borders::BOTTOM);
        let new_header = Block::default().title("NEW").borders(Borders::BOTTOM);

        // Calcula el área de contenido dentro de los encabezados.
        let old_content_area = Rect {
            y: old_area.y + 1,
            height: old_area.height.saturating_sub(1),
            ..old_area
        };
        let new_content_area = Rect {
            y: new_area.y + 1,
            height: new_area.height.saturating_sub(1),
            ..new_area
        };

        frame.render_widget(
            old_header,
            Rect {
                height: 1,
                ..old_area
            },
        );
        frame.render_widget(
            new_header,
            Rect {
                height: 1,
                ..new_area
            },
        );

        if let Some(fd) = self.diff.files.get(self.selected_file) {
            let mut old_lines: Vec<TuiLine> = Vec::new();
            let mut new_lines: Vec<TuiLine> = Vec::new();

            let active_range = self.active_range();
            let mut flat_idx: usize = 0;
            for hunk in &fd.hunks {
                for line in &hunk.lines {
                    let is_cursor = flat_idx == self.cursor_line;
                    let in_range =
                        active_range.is_some_and(|(lo, hi)| flat_idx >= lo && flat_idx <= hi);
                    let gutter = range_gutter(active_range.is_some(), in_range);
                    let has_comment = self.line_has_comment(fd, line);
                    let comment_mark = if has_comment { " 💬" } else { "" };
                    let mut cursor_style = if is_cursor {
                        Style::default().add_modifier(Modifier::REVERSED)
                    } else {
                        Style::default()
                    };
                    if in_range {
                        cursor_style = cursor_style.bg(Color::Blue);
                    }

                    match line.kind {
                        LineKind::Removed => {
                            let lineno = line.old_lineno.unwrap_or(0);
                            let text =
                                format!("{gutter}{lineno:>4} - {}{comment_mark}", line.content);
                            old_lines.push(TuiLine::styled(text, cursor_style.fg(Color::Red)));
                            new_lines.push(TuiLine::raw(""));
                        }
                        LineKind::Added => {
                            let lineno = line.new_lineno.unwrap_or(0);
                            let text =
                                format!("{gutter}{lineno:>4} + {}{comment_mark}", line.content);
                            old_lines.push(TuiLine::raw(""));
                            new_lines.push(TuiLine::styled(text, cursor_style.fg(Color::Green)));
                        }
                        LineKind::Context => {
                            let old_no = line.old_lineno.unwrap_or(0);
                            let new_no = line.new_lineno.unwrap_or(0);
                            let old_text = format!("{gutter}{old_no:>4}   {}", line.content);
                            let new_text =
                                format!("{gutter}{new_no:>4}   {}{comment_mark}", line.content);
                            old_lines.push(TuiLine::styled(old_text, cursor_style));
                            new_lines.push(TuiLine::styled(new_text, cursor_style));
                        }
                    }
                    flat_idx += 1;
                }
            }

            let old_para = Paragraph::new(old_lines).wrap(Wrap { trim: false });
            let new_para = Paragraph::new(new_lines).wrap(Wrap { trim: false });
            frame.render_widget(old_para, old_content_area);
            frame.render_widget(new_para, new_content_area);
        }
    }

    fn render_diff_unified(&self, frame: &mut Frame, area: Rect) {
        let focused = self.focus == Focus::Diff;
        let border_style = if focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };
        let border_type = if focused {
            BorderType::Double
        } else {
            BorderType::Plain
        };

        let block = Block::default()
            .title("[2] DIFF")
            .borders(Borders::ALL)
            .border_style(border_style)
            .border_type(border_type);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if let Some(fd) = self.diff.files.get(self.selected_file) {
            let active_range = self.active_range();
            let mut lines: Vec<TuiLine> = Vec::new();
            let mut flat_idx: usize = 0;

            for hunk in &fd.hunks {
                for line in &hunk.lines {
                    let is_cursor = flat_idx == self.cursor_line;
                    let in_range =
                        active_range.is_some_and(|(lo, hi)| flat_idx >= lo && flat_idx <= hi);
                    let gutter = range_gutter(active_range.is_some(), in_range);
                    let has_comment = self.line_has_comment(fd, line);
                    let comment_mark = if has_comment { " 💬" } else { "" };
                    let mut cursor_style = if is_cursor {
                        Style::default().add_modifier(Modifier::REVERSED)
                    } else {
                        Style::default()
                    };
                    if in_range {
                        cursor_style = cursor_style.bg(Color::Blue);
                    }

                    let tui_line = match line.kind {
                        LineKind::Removed => {
                            let old_no = line.old_lineno.unwrap_or(0);
                            let text = format!(
                                "{gutter}{old_no:>4}      - {}{comment_mark}",
                                line.content
                            );
                            TuiLine::styled(text, cursor_style.fg(Color::Red))
                        }
                        LineKind::Added => {
                            let new_no = line.new_lineno.unwrap_or(0);
                            let text = format!(
                                "{gutter}{new_no:>4}      + {}{comment_mark}",
                                line.content
                            );
                            TuiLine::styled(text, cursor_style.fg(Color::Green))
                        }
                        LineKind::Context => {
                            let old_no = line.old_lineno.unwrap_or(0);
                            let new_no = line.new_lineno.unwrap_or(0);
                            let text = format!(
                                "{gutter}{old_no:>4} {new_no:>4}   {}{comment_mark}",
                                line.content
                            );
                            TuiLine::styled(text, cursor_style)
                        }
                    };
                    lines.push(tui_line);
                    flat_idx += 1;
                }
            }

            let para = Paragraph::new(lines).wrap(Wrap { trim: false });
            frame.render_widget(para, inner);
        }
    }

    // ---------------------------------------------------------------------------
    // Render — pantalla final
    // ---------------------------------------------------------------------------

    fn render_final_screen(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("Finalizar review")
            .borders(Borders::ALL);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Layout: lista de comentarios arriba, general + veredicto abajo.
        let vert = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(4),
                Constraint::Length(3),
                Constraint::Length(2),
                Constraint::Length(1),
            ])
            .split(inner);

        let comments_area = vert[0];
        let general_area = vert[1];
        let verdict_area = vert[2];
        let hint_area = vert[3];

        // -- Lista de comentarios --
        let comment_lines: Vec<TuiLine> = self
            .review
            .comments()
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let anchor = build_anchor(c);
                let marker = if i == self.selected_comment { ">" } else { " " };
                let style = if i == self.selected_comment {
                    Style::default().add_modifier(Modifier::REVERSED)
                } else {
                    Style::default()
                };
                TuiLine::styled(format!("{marker} {anchor}  {}", c.body), style)
            })
            .collect();

        let comments_block = Block::default().title("Comentarios").borders(Borders::ALL);
        let comments_para = Paragraph::new(comment_lines)
            .block(comments_block)
            .wrap(Wrap { trim: false });
        frame.render_widget(comments_para, comments_area);

        // -- Comentario general --
        let general_block = Block::default()
            .title("Comentario general")
            .borders(Borders::ALL);
        let general_para = Paragraph::new(self.general_buf.as_str())
            .block(general_block)
            .wrap(Wrap { trim: false });
        frame.render_widget(general_para, general_area);

        // -- Veredicto --
        let lgtm_style = if self.final_verdict == Verdict::Lgtm {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED)
        } else {
            Style::default()
        };
        let ko_style = if self.final_verdict == Verdict::Ko {
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED)
        } else {
            Style::default()
        };

        let verdict_line = TuiLine::from(vec![
            Span::raw("Veredicto:  "),
            Span::styled("[ LGTM ]", lgtm_style),
            Span::raw("  "),
            Span::styled("[ KO ]", ko_style),
        ]);
        let verdict_para = Paragraph::new(verdict_line);
        frame.render_widget(verdict_para, verdict_area);

        // -- Hint --
        let hint = Paragraph::new("← LGTM · → KO · Ctrl+S guardar · Esc volver");
        frame.render_widget(hint, hint_area);
    }

    // ---------------------------------------------------------------------------
    // Helpers de render
    // ---------------------------------------------------------------------------

    /// Devuelve true si la línea dada tiene algún comentario anclado.
    fn line_has_comment(&self, fd: &crate::diff::FileDiff, line: &crate::diff::Line) -> bool {
        let (side, lineno) = Self::anchor_of(line);
        let file_str = fd.path.display().to_string();
        self.review.comments().iter().any(|c| {
            c.file == file_str && c.side == side && lineno >= c.start_line && lineno <= c.end_line
        })
    }
}

// ---------------------------------------------------------------------------
// Helpers de render libres (no necesitan &self)
// ---------------------------------------------------------------------------

fn render_empty(frame: &mut Frame, area: Rect) {
    let block = Block::default().title("code-review").borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let msg = Paragraph::new("No hay cambios sin commitear para revisar.")
        .style(Style::default())
        .wrap(Wrap { trim: false });
    frame.render_widget(msg, inner);
}

/// Formatea el anclaje de un comentario: `archivo:N` o `archivo:N-M`.
/// Marcador de canaleta para una línea del diff: vacío fuera del modo rango
/// (no desplaza el render normal), `▌` para una línea dentro del rango activo y
/// un espacio para las demás líneas mientras se selecciona.
fn range_gutter(range_active: bool, in_range: bool) -> &'static str {
    if !range_active {
        ""
    } else if in_range {
        "▌"
    } else {
        " "
    }
}

fn build_anchor(c: &crate::review::Comment) -> String {
    if c.start_line == c.end_line {
        format!("{}:{}", c.file, c.start_line)
    } else {
        format!("{}:{}-{}", c.file, c.start_line, c.end_line)
    }
}

// ---------------------------------------------------------------------------
// Tests unitarios
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::{Diff, FileDiff, FileStatus, Hunk, Line, LineKind};
    use crate::review::Side;
    use std::path::PathBuf;

    fn make_line(kind: LineKind, old: Option<u32>, new: Option<u32>, content: &str) -> Line {
        Line {
            kind,
            old_lineno: old,
            new_lineno: new,
            content: content.to_owned(),
        }
    }

    fn simple_diff() -> Diff {
        Diff {
            files: vec![FileDiff {
                path: PathBuf::from("src/diff.rs"),
                status: FileStatus::Modified,
                additions: 1,
                deletions: 1,
                hunks: vec![Hunk {
                    old_start: 12,
                    old_lines: 4,
                    new_start: 12,
                    new_lines: 4,
                    lines: vec![
                        make_line(
                            LineKind::Context,
                            Some(12),
                            Some(12),
                            "pub struct FileDiff {",
                        ),
                        make_line(
                            LineKind::Context,
                            Some(13),
                            Some(13),
                            "    pub path: PathBuf,",
                        ),
                        make_line(LineKind::Removed, Some(14), None, "    pub status: u8,"),
                        make_line(
                            LineKind::Added,
                            None,
                            Some(15),
                            "    pub status: FileStatus,",
                        ),
                        make_line(
                            LineKind::Context,
                            Some(15),
                            Some(16),
                            "    pub hunks: Vec<Hunk>,",
                        ),
                    ],
                }],
            }],
        }
    }

    fn two_file_diff() -> Diff {
        Diff {
            files: vec![
                FileDiff {
                    path: PathBuf::from("src/diff.rs"),
                    status: FileStatus::Modified,
                    additions: 1,
                    deletions: 1,
                    hunks: vec![Hunk {
                        old_start: 12,
                        old_lines: 4,
                        new_start: 12,
                        new_lines: 4,
                        lines: vec![
                            make_line(
                                LineKind::Context,
                                Some(12),
                                Some(12),
                                "pub struct FileDiff {",
                            ),
                            make_line(
                                LineKind::Context,
                                Some(13),
                                Some(13),
                                "    pub path: PathBuf,",
                            ),
                            make_line(LineKind::Removed, Some(14), None, "    pub status: u8,"),
                            make_line(
                                LineKind::Added,
                                None,
                                Some(15),
                                "    pub status: FileStatus,",
                            ),
                            make_line(
                                LineKind::Context,
                                Some(15),
                                Some(16),
                                "    pub hunks: Vec<Hunk>,",
                            ),
                        ],
                    }],
                },
                FileDiff {
                    path: PathBuf::from("src/main.rs"),
                    status: FileStatus::Modified,
                    additions: 3,
                    deletions: 0,
                    hunks: vec![Hunk {
                        old_start: 19,
                        old_lines: 1,
                        new_start: 20,
                        new_lines: 4,
                        lines: vec![
                            make_line(LineKind::Context, Some(19), Some(19), "fn run() {"),
                            make_line(
                                LineKind::Added,
                                None,
                                Some(20),
                                "    let mut files = Vec::new();",
                            ),
                            make_line(LineKind::Added, None, Some(21), "    for hunk in raw {"),
                            make_line(LineKind::Added, None, Some(22), "        files.push(hunk);"),
                        ],
                    }],
                },
            ],
        }
    }

    // --- Estado inicial ---

    #[test]
    fn initial_state_is_correct() {
        let app = App::new(simple_diff());
        assert_eq!(app.focus(), Focus::Files);
        assert_eq!(app.mode(), Mode::Navigate);
        assert_eq!(app.view_mode(), ViewMode::Split);
        assert_eq!(app.selected_file_index(), 0);
        assert_eq!(app.cursor_line(), 0);
    }

    // --- Transiciones de foco ---

    #[test]
    fn key_1_sets_focus_files() {
        let mut app = App::new(simple_diff());
        app.focus = Focus::Diff;
        let k = KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE);
        app.handle_key(k);
        assert_eq!(app.focus(), Focus::Files);
    }

    #[test]
    fn key_2_sets_focus_diff() {
        let mut app = App::new(simple_diff());
        let k = KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE);
        app.handle_key(k);
        assert_eq!(app.focus(), Focus::Diff);
    }

    // --- Navegación de archivos ---

    #[test]
    fn j_in_files_advances_selected_file() {
        let mut app = App::new(two_file_diff());
        let k = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        app.handle_key(k);
        assert_eq!(app.selected_file_index(), 1);
    }

    #[test]
    fn j_in_files_clamps_at_last() {
        let mut app = App::new(simple_diff()); // 1 archivo
        let k = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        app.handle_key(k);
        assert_eq!(app.selected_file_index(), 0);
    }

    #[test]
    fn k_in_files_decrements_selected_file() {
        let mut app = App::new(two_file_diff());
        app.selected_file = 1;
        let k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
        app.handle_key(k);
        assert_eq!(app.selected_file_index(), 0);
    }

    #[test]
    fn k_in_files_clamps_at_zero() {
        let mut app = App::new(two_file_diff());
        let k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
        app.handle_key(k);
        assert_eq!(app.selected_file_index(), 0);
    }

    #[test]
    fn changing_file_resets_cursor_line() {
        let mut app = App::new(two_file_diff());
        // Enfocamos DIFF y movemos el cursor.
        app.focus = Focus::Diff;
        app.cursor_line = 3;
        // Volvemos a FILES y bajamos de archivo.
        app.focus = Focus::Files;
        let k = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        app.handle_key(k);
        assert_eq!(app.cursor_line(), 0);
    }

    // --- Navegación de líneas (foco DIFF) ---

    #[test]
    fn j_in_diff_advances_cursor_line() {
        let mut app = App::new(simple_diff());
        app.focus = Focus::Diff;
        let k = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        app.handle_key(k);
        assert_eq!(app.cursor_line(), 1);
    }

    #[test]
    fn j_in_diff_clamps_at_last_line() {
        let mut app = App::new(simple_diff()); // 5 líneas: índices 0-4
        app.focus = Focus::Diff;
        app.cursor_line = 4;
        let k = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        app.handle_key(k);
        assert_eq!(app.cursor_line(), 4);
    }

    #[test]
    fn k_in_diff_decrements_cursor_line() {
        let mut app = App::new(simple_diff());
        app.focus = Focus::Diff;
        app.cursor_line = 2;
        let k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
        app.handle_key(k);
        assert_eq!(app.cursor_line(), 1);
    }

    #[test]
    fn k_in_diff_clamps_at_zero() {
        let mut app = App::new(simple_diff());
        app.focus = Focus::Diff;
        let k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
        app.handle_key(k);
        assert_eq!(app.cursor_line(), 0);
    }

    // --- Toggle de vista ---

    #[test]
    fn t_toggles_view_mode_split_to_unified() {
        let mut app = App::new(simple_diff());
        let k = KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE);
        app.handle_key(k);
        assert_eq!(app.view_mode(), ViewMode::Unified);
    }

    #[test]
    fn t_toggles_view_mode_unified_to_split() {
        let mut app = App::new(simple_diff());
        app.view_mode = ViewMode::Unified;
        let k = KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE);
        app.handle_key(k);
        assert_eq!(app.view_mode(), ViewMode::Split);
    }

    // --- q devuelve Quit ---

    #[test]
    fn q_returns_quit() {
        let mut app = App::new(simple_diff());
        let k = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert_eq!(app.handle_key(k), Outcome::Quit);
    }

    // --- Derivación de anclaje ---

    #[test]
    fn anchor_of_removed_line_is_old_side() {
        let line = make_line(LineKind::Removed, Some(14), None, "    pub status: u8,");
        let (side, lineno) = App::anchor_of(&line);
        assert_eq!(side, Side::Old);
        assert_eq!(lineno, 14);
    }

    #[test]
    fn anchor_of_added_line_is_new_side() {
        let line = make_line(
            LineKind::Added,
            None,
            Some(15),
            "    pub status: FileStatus,",
        );
        let (side, lineno) = App::anchor_of(&line);
        assert_eq!(side, Side::New);
        assert_eq!(lineno, 15);
    }

    #[test]
    fn anchor_of_context_line_is_new_side() {
        let line = make_line(
            LineKind::Context,
            Some(12),
            Some(12),
            "pub struct FileDiff {",
        );
        let (side, lineno) = App::anchor_of(&line);
        assert_eq!(side, Side::New);
        assert_eq!(lineno, 12);
    }

    // --- Comentario de línea: anclaje correcto ---

    #[test]
    fn comment_line_cursor_3_anchors_to_new_15() {
        // Cursor en índice 3 (línea Added new=15).
        let mut app = App::new(simple_diff());
        app.focus = Focus::Diff;
        app.cursor_line = 3;
        let c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE);
        app.handle_key(c);
        // Escribe el comentario
        for ch in "test body".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
        }
        let save = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL);
        app.handle_key(save);

        let comments = app.review().comments();
        assert_eq!(comments.len(), 1);
        let comment = &comments[0];
        assert_eq!(comment.file, "src/diff.rs");
        assert_eq!(comment.side, Side::New);
        assert_eq!(comment.start_line, 15);
        assert_eq!(comment.end_line, 15);
    }

    // --- Modo RangeSelect ---

    #[test]
    fn v_enters_range_select_mode() {
        let mut app = App::new(simple_diff());
        app.focus = Focus::Diff;
        let k = KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE);
        app.handle_key(k);
        assert_eq!(app.mode(), Mode::RangeSelect);
    }

    #[test]
    fn esc_in_range_select_returns_to_navigate() {
        let mut app = App::new(simple_diff());
        app.mode = Mode::RangeSelect;
        let k = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        app.handle_key(k);
        assert_eq!(app.mode(), Mode::Navigate);
    }

    // --- Modo Final ---

    #[test]
    fn g_enters_final_mode() {
        let mut app = App::new(simple_diff());
        let k = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE);
        app.handle_key(k);
        assert_eq!(app.mode(), Mode::Final);
    }

    #[test]
    fn left_arrow_in_final_sets_lgtm() {
        let mut app = App::new(simple_diff());
        app.mode = Mode::Final;
        let k = KeyEvent::new(KeyCode::Left, KeyModifiers::NONE);
        app.handle_key(k);
        assert_eq!(app.final_verdict, Verdict::Lgtm);
    }

    #[test]
    fn right_arrow_in_final_sets_ko() {
        let mut app = App::new(simple_diff());
        app.mode = Mode::Final;
        let k = KeyEvent::new(KeyCode::Right, KeyModifiers::NONE);
        app.handle_key(k);
        assert_eq!(app.final_verdict, Verdict::Ko);
    }

    #[test]
    fn esc_in_final_returns_to_navigate() {
        let mut app = App::new(simple_diff());
        app.mode = Mode::Final;
        let k = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        app.handle_key(k);
        assert_eq!(app.mode(), Mode::Navigate);
    }

    // --- build_anchor ---

    #[test]
    fn build_anchor_single_line() {
        let c = crate::review::Comment {
            file: "src/foo.rs".to_owned(),
            side: Side::New,
            start_line: 10,
            end_line: 10,
            body: "body".to_owned(),
        };
        assert_eq!(build_anchor(&c), "src/foo.rs:10");
    }

    #[test]
    fn build_anchor_range() {
        let c = crate::review::Comment {
            file: "src/main.rs".to_owned(),
            side: Side::New,
            start_line: 20,
            end_line: 22,
            body: "body".to_owned(),
        };
        assert_eq!(build_anchor(&c), "src/main.rs:20-22");
    }

    // --- Rango: comentario main.rs índices 1-3 -> new 20-22 ---

    #[test]
    fn range_comment_indices_1_to_3_anchors_to_new_20_22() {
        let mut app = App::new(two_file_diff());
        // Seleccionar main.rs (file index 1).
        app.selected_file = 1;
        app.focus = Focus::Diff;
        // Índice 1 en main.rs = Added new=20.
        app.cursor_line = 1;
        app.range_start = 1;
        // Extender hasta índice 3 (Added new=22).
        app.cursor_line = 3;
        // Confirmar rango.
        app.start_range_comment();
        // Escribir cuerpo.
        for ch in "rango body".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
        }
        let save = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL);
        app.handle_key(save);

        let comments = app.review().comments();
        assert_eq!(comments.len(), 1);
        let c = &comments[0];
        assert_eq!(c.file, "src/main.rs");
        assert_eq!(c.side, Side::New);
        assert_eq!(c.start_line, 20);
        assert_eq!(c.end_line, 22);
    }

    // --- cycle_focus: cicla entre Files y Diff (dos paneles) ---

    #[test]
    fn cycle_focus_from_files_goes_to_diff() {
        let mut app = App::new(simple_diff());
        app.focus = Focus::Files;
        app.cycle_focus(true);
        assert_eq!(app.focus(), Focus::Diff);
    }

    #[test]
    fn cycle_focus_from_diff_goes_to_files() {
        let mut app = App::new(simple_diff());
        app.focus = Focus::Diff;
        app.cycle_focus(true);
        assert_eq!(app.focus(), Focus::Files);
    }

    #[test]
    fn cycle_focus_backward_from_files_goes_to_diff() {
        let mut app = App::new(simple_diff());
        app.focus = Focus::Files;
        app.cycle_focus(false);
        assert_eq!(app.focus(), Focus::Diff);
    }

    #[test]
    fn cycle_focus_from_thread_goes_to_files() {
        let mut app = App::new(simple_diff());
        app.focus = Focus::Thread;
        app.cycle_focus(true);
        assert_eq!(app.focus(), Focus::Files);
    }

    // --- open_thread: abre el hilo si la línea tiene comentario ---

    #[test]
    fn open_thread_with_comment_sets_focus_thread() {
        let mut app = App::new(simple_diff());
        app.focus = Focus::Diff;
        app.cursor_line = 3; // Added new=15
        app.review
            .add_line_comment("src/diff.rs", Side::New, 15, "cuerpo");
        app.open_thread();
        assert_eq!(app.focus(), Focus::Thread);
    }

    #[test]
    fn open_thread_without_comment_keeps_focus_diff() {
        let mut app = App::new(simple_diff());
        app.focus = Focus::Diff;
        app.cursor_line = 0;
        app.open_thread();
        assert_eq!(app.focus(), Focus::Diff);
    }

    #[test]
    fn open_thread_ignores_enter_when_focus_is_files() {
        let mut app = App::new(simple_diff());
        app.focus = Focus::Files;
        app.review
            .add_line_comment("src/diff.rs", Side::New, 15, "cuerpo");
        app.open_thread();
        // open_thread solo actua con Focus::Diff
        assert_eq!(app.focus(), Focus::Files);
    }

    // --- find_flat_index: busca el índice aplanado por (side, lineno) ---

    #[test]
    fn find_flat_index_added_line_15_returns_3() {
        let app = App::new(simple_diff());
        let idx = app.find_flat_index(0, &Side::New, 15);
        assert_eq!(idx, 3, "la línea added new=15 está en el índice aplanado 3");
    }

    #[test]
    fn find_flat_index_context_line_12_returns_0() {
        let app = App::new(simple_diff());
        let idx = app.find_flat_index(0, &Side::New, 12);
        assert_eq!(idx, 0);
    }

    #[test]
    fn find_flat_index_not_found_returns_0() {
        let app = App::new(simple_diff());
        let idx = app.find_flat_index(0, &Side::New, 999);
        assert_eq!(idx, 0, "si no se encuentra, devuelve 0");
    }

    #[test]
    fn find_flat_index_added_20_in_second_file_returns_1() {
        let app = App::new(two_file_diff());
        let idx = app.find_flat_index(1, &Side::New, 20);
        assert_eq!(
            idx, 1,
            "la línea added new=20 en main.rs está en el índice 1"
        );
    }

    // --- selected_comment_index: arranca en 0 ---

    #[test]
    fn selected_comment_index_starts_at_zero() {
        let app = App::new(simple_diff());
        assert_eq!(app.selected_comment_index(), 0);
    }
}
