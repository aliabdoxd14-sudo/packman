use std::path::Path;
use std::process::Command;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, BorderType, Cell, Paragraph, Row, Table, Tabs};
use ratatui::Frame;
use thiserror::Error;

mod pkg_manager;
use pkg_manager::PackageManager;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Package manager error: {0}")]
    PkgMgr(String),
}

impl From<String> for AppError {
    fn from(s: String) -> Self {
        AppError::PkgMgr(s)
    }
}

impl From<&str> for AppError {
    fn from(s: &str) -> Self {
        AppError::PkgMgr(s.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FilterMode {
    #[default]
    All,
    Installed,
    Available,
    Outdated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortField {
    #[default]
    Name,
    Version,
    Size,
    Status,
}

#[derive(Debug, Clone)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub status: PackageStatus,
    pub size: u64,
    pub description: String,
    pub repository: Option<String>,
    pub installed_by: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageStatus {
    Installed,
    Available,
    Outdated,
    Local,
}

impl Default for PackageStatus {
    fn default() -> Self {
        PackageStatus::Installed
    }
}

impl std::fmt::Display for PackageStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PackageStatus::Installed => write!(f, "installed"),
            PackageStatus::Available => write!(f, "available"),
            PackageStatus::Outdated => write!(f, "outdated"),
            PackageStatus::Local => write!(f, "local"),
        }
    }
}

pub struct App {
    pub package_managers: Vec<PackageManager>,
    pub active_pm_index: usize,
    pub packages: Vec<Package>,
    pub selected_package_index: usize,
    pub search_query: String,
    pub search_mode: bool,
    pub filter_mode: FilterMode,
    pub sort_field: SortField,
    pub sort_ascending: bool,
    pub loading: bool,
    pub message: Option<String>,
    pub show_outdated_only: bool,
    pub distro: String,
    pub terminal_size: (u16, u16),
}

impl App {
    pub fn new() -> AppResult<Self> {
        let package_managers = detect_package_managers();
        let distro = detect_distro();

        Ok(App {
            package_managers,
            active_pm_index: 0,
            packages: Vec::new(),
            selected_package_index: 0,
            search_query: String::new(),
            search_mode: false,
            filter_mode: FilterMode::All,
            sort_field: SortField::Name,
            sort_ascending: true,
            loading: false,
            message: None,
            show_outdated_only: false,
            distro,
            terminal_size: (80, 24),
        })
    }

    pub fn load_packages_sync(&mut self) {
        if self.active_pm_index >= self.package_managers.len() {
            return;
        }

        let pm = &self.package_managers[self.active_pm_index];

        if !pm.available {
            self.message = Some(format!("{} is not available", pm.name));
            return;
        }

        self.loading = true;
        self.packages.clear();

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        match rt.block_on(pm.list_packages()) {
            Ok(pkgs) => {
                self.packages = pkgs;
            }
            Err(e) => {
                self.message = Some(format!("Error loading packages: {}", e));
            }
        }

        self.loading = false;
    }

    pub async fn load_packages(&mut self) {
        let pm = self.package_managers[self.active_pm_index].clone();

        let pkgs = tokio::task::spawn_blocking(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(pm.list_packages())
        })
        .await
        .unwrap_or(Ok(Vec::new()));

        if let Ok(pkgs) = pkgs {
            self.packages = pkgs;
        }
    }

    pub fn active_pm(&self) -> Option<PackageManager> {
        self.package_managers.get(self.active_pm_index).cloned()
    }

    pub fn filtered_packages(&self) -> Vec<(usize, &Package)> {
        let mut filtered: Vec<_> = self
            .packages
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                let matches_search = if self.search_query.is_empty() {
                    true
                } else {
                    let query = self.search_query.to_lowercase();
                    p.name.to_lowercase().contains(&query)
                        || p.description.to_lowercase().contains(&query)
                };

                let matches_filter = match self.filter_mode {
                    FilterMode::All => true,
                    FilterMode::Installed => p.status == PackageStatus::Installed,
                    FilterMode::Available => p.status == PackageStatus::Available,
                    FilterMode::Outdated => p.status == PackageStatus::Outdated,
                };

                let matches_outdated = if self.show_outdated_only {
                    p.status == PackageStatus::Outdated
                } else {
                    true
                };

                matches_search && matches_filter && matches_outdated
            })
            .collect();

        filtered.sort_by(|a, b| {
            let cmp = match self.sort_field {
                SortField::Name => a.1.name.cmp(&b.1.name),
                SortField::Version => a.1.version.cmp(&b.1.version),
                SortField::Size => a.1.size.cmp(&b.1.size),
                SortField::Status => {
                    let a_status = match a.1.status {
                        PackageStatus::Installed => 0,
                        PackageStatus::Available => 1,
                        PackageStatus::Outdated => 2,
                        PackageStatus::Local => 3,
                    };
                    let b_status = match b.1.status {
                        PackageStatus::Installed => 0,
                        PackageStatus::Available => 1,
                        PackageStatus::Outdated => 2,
                        PackageStatus::Local => 3,
                    };
                    a_status.cmp(&b_status)
                }
            };
            if self.sort_ascending {
                cmp
            } else {
                cmp.reverse()
            }
        });

        filtered
    }

    pub fn select_next(&mut self) {
        let count = self.filtered_packages().len();
        if count > 0 {
            self.selected_package_index = (self.selected_package_index + 1) % count;
        }
    }

    pub fn select_previous(&mut self) {
        let count = self.filtered_packages().len();
        if count > 0 {
            self.selected_package_index = if self.selected_package_index == 0 {
                count - 1
            } else {
                self.selected_package_index - 1
            };
        }
    }

    pub fn select_first(&mut self) {
        self.selected_package_index = 0;
    }

    pub fn up(&mut self, amt: usize) {
        let count = self.filtered_packages().len();
        if count > 0 {
            self.selected_package_index = self.selected_package_index.saturating_sub(amt);
        }
    }

    pub fn down(&mut self, amt: usize) {
        let count = self.filtered_packages().len();
        if count > 0 {
            self.selected_package_index = (self.selected_package_index + amt).min(count - 1);
        }
    }
}

pub fn detect_distro() -> String {
    if Path::new("/etc/os-release").exists() {
        if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
            for line in content.lines() {
                if line.starts_with("PRETTY_NAME=") {
                    return line
                        .trim_start_matches("PRETTY_NAME=")
                        .trim_matches('"')
                        .to_string();
                }
            }
        }
    }
    if Path::new("/etc/arch-release").exists() {
        return "Arch Linux".to_string();
    }
    if Path::new("/etc/debian_version").exists() {
        return "Debian".to_string();
    }
    if Path::new("/etc/fedora-release").exists() {
        return "Fedora".to_string();
    }
    "Unknown Linux".to_string()
}

fn detect_package_managers() -> Vec<PackageManager> {
    let mut managers = Vec::new();

    let pm_configs = vec![
        ("pip", "pip3", "pip", false),
        ("npm", "npm", "npm", false),
        ("pnpm", "pnpm", "pnpm", false),
        ("cargo", "cargo", "cargo", false),
        ("brew", "brew", "brew", false),
        ("apt", "apt", "dpkg", true),
        ("pacman", "pacman", "pacman", true),
        ("aur", "yay", "yay", false),
        ("rpm", "rpm", "rpm", true),
        ("flatpak", "flatpak", "flatpak", true),
        ("snap", "snap", "snap", false),
    ];

    for (name, cmd, list_cmd, needs_root) in pm_configs {
        if is_command_available(cmd) {
            managers.push(PackageManager {
                name: name.to_string(),
                command: cmd.to_string(),
                list_command: list_cmd.to_string(),
                available: true,
                needs_root: needs_root || is_command_available("sudo"),
            });
        }
    }

    managers
}

fn is_command_available(cmd: &str) -> bool {
    Command::new("sh")
        .args(["-c", &format!("command -v {}", cmd)])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

struct AppColors {
    bg: Color,
    fg: Color,
    primary: Color,
    secondary: Color,
    accent: Color,
    warning: Color,
    error: Color,
    surface: Color,
    border: Color,
}

impl AppColors {
    const fn new() -> Self {
        Self {
            bg: Color::Rgb(26, 27, 38),
            fg: Color::Rgb(169, 177, 214),
            primary: Color::Rgb(122, 162, 247),
            secondary: Color::Rgb(187, 154, 247),
            accent: Color::Rgb(158, 206, 106),
            warning: Color::Rgb(224, 175, 104),
            error: Color::Rgb(247, 118, 142),
            surface: Color::Rgb(36, 40, 59),
            border: Color::Rgb(65, 72, 104),
        }
    }
}

const COLORS: AppColors = AppColors::new();

fn render_app(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(frame.area());

    render_header(frame, app, chunks[0]);
    render_body(frame, app, chunks[1]);
    render_footer(frame, app, chunks[2]);
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(20),
            Constraint::Min(0),
            Constraint::Length(20),
        ])
        .split(area);

    let title = Paragraph::new(" PackMan ")
        .style(Style::default().fg(COLORS.primary))
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(COLORS.border)),
        )
        .alignment(Alignment::Center);

    let distro = Paragraph::new(app.distro.as_str())
        .style(Style::default().fg(COLORS.secondary))
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(COLORS.border)),
        )
        .alignment(Alignment::Center);

    let pm_tabs = if app.package_managers.is_empty() {
        Tabs::new(vec!["No PMs"])
            .style(Style::default().fg(COLORS.fg))
            .select(0)
    } else {
        let names: Vec<_> = app
            .package_managers
            .iter()
            .map(|pm| pm.name.as_str())
            .collect();
        Tabs::new(names)
            .style(Style::default().fg(COLORS.fg))
            .select(app.active_pm_index)
            .highlight_style(Style::default().fg(COLORS.primary).bg(COLORS.surface))
    };

    let pm_block = pm_tabs.block(
        Block::bordered()
            .title(" Package Managers ")
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(COLORS.border)),
    );

    f.render_widget(title, chunks[0]);
    f.render_widget(pm_block, chunks[1]);
    f.render_widget(distro, chunks[2]);
}

fn render_body(f: &mut Frame, app: &App, area: Rect) {
    if app.loading {
        let msg = Paragraph::new("Loading packages...")
            .style(Style::default().fg(COLORS.fg))
            .alignment(Alignment::Center);
        f.render_widget(msg, area);
        return;
    }

    let filtered = app.filtered_packages();

    if filtered.is_empty() {
        let msg = if app.package_managers.is_empty() {
            "No package managers detected"
        } else if app.search_query.is_empty() {
            "No packages found"
        } else {
            "No packages match your search"
        };

        let msg = Paragraph::new(msg)
            .style(Style::default().fg(COLORS.warning))
            .alignment(Alignment::Center);
        f.render_widget(msg, area);
        return;
    }

    let selected_idx = filtered
        .iter()
        .map(|(i, _)| *i)
        .nth(app.selected_package_index);

    let visible_rows = (area.height as usize).saturating_sub(2);
    let max_scroll = filtered.len().saturating_sub(visible_rows);
    let half_visible = visible_rows / 2;
    let scroll_offset = app
        .selected_package_index
        .saturating_sub(half_visible)
        .min(max_scroll);

    let rows: Vec<_> = filtered
        .iter()
        .skip(scroll_offset)
        .take(visible_rows)
        .map(|(idx, pkg)| {
            let is_selected = Some(*idx) == selected_idx;
            let status_color = match pkg.status {
                PackageStatus::Installed => COLORS.accent,
                PackageStatus::Available => COLORS.fg,
                PackageStatus::Outdated => COLORS.warning,
                PackageStatus::Local => COLORS.secondary,
            };
            let style = if is_selected {
                Style::default().fg(COLORS.bg).bg(COLORS.primary)
            } else {
                Style::default().fg(COLORS.fg)
            };

            Row::new(vec![
                Cell::from(pkg.name.as_str()).style(style),
                Cell::from(pkg.version.as_str()).style(style),
                Cell::from(pkg.status.to_string()).style(style.fg(status_color)),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        vec![
            Constraint::Percentage(35),
            Constraint::Percentage(25),
            Constraint::Percentage(20),
            Constraint::Min(0),
        ],
    )
    .block(
        Block::bordered()
            .title(" Packages ")
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(COLORS.border)),
    )
    .header(Row::new(vec!["Name", "Version", "Status"]).style(Style::default().fg(COLORS.primary)))
    .column_spacing(1);

    f.render_widget(table, area);
}

fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(30)])
        .split(area);

    let search_indicator = if app.search_mode {
        format!(" [SEARCH: {}] ", app.search_query)
    } else {
        " ↑↓ Navigate  / Search  u Upgrade  r Remove  i Install  q Quit ".to_string()
    };
    let action_hints = Paragraph::new(search_indicator)
        .style(Style::default().fg(if app.search_mode {
            COLORS.warning
        } else {
            COLORS.fg
        }))
        .alignment(Alignment::Center);

    let status = if let Some((_, pkg)) = app
        .filtered_packages()
        .iter()
        .nth(app.selected_package_index)
    {
        format!(" {} {} ({}) ", pkg.name, pkg.version, pkg.status)
    } else {
        " No package selected ".to_string()
    };

    let status_text = Paragraph::new(status)
        .style(Style::default().fg(COLORS.accent))
        .alignment(Alignment::Right);

    f.render_widget(action_hints, chunks[0]);
    f.render_widget(status_text, chunks[1]);
}

fn handle_pm_switch(app: &mut App, _runtime: &tokio::runtime::Runtime) {
    app.load_packages_sync();
}

pub fn run() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("📦 PackMan - Package Manager TUI");
        println!();
        println!("Usage: packman [OPTIONS]");
        println!();
        println!("Options:");
        println!("  -h, --help     Show this help message");
        println!();
        println!("Keyboard Shortcuts:");
        println!("  ↑/↓ or j/k    Navigate package list");
        println!("  /             Toggle search mode");
        println!("  u             Upgrade selected package");
        println!("  r             Remove selected package");
        println!("  i             Install package (type name in search)");
        println!("  Tab           Switch package manager");
        println!("  Tab           Switch package manager");
        println!("  Ctrl+R        Refresh package list");
        println!("  Ctrl+O        Toggle outdated packages only");
        println!("  q or Esc      Quit");
        println!();
        println!("Supported Package Managers:");
        println!("  pip, npm, cargo, apt, pacman, aur, rpm, flatpak, snap, brew");
        return;
    }

    let mut app = App::new().expect("Failed to create app");
    let mut terminal = ratatui::init();

    {
        let backend = terminal.backend_mut();
        let _ = execute!(backend, EnterAlternateScreen);
    }

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    app.load_packages_sync();

    let mut should_quit = false;

    while !should_quit {
        let _ = terminal.draw(|f| render_app(f, &app));

        if crossterm::event::poll(std::time::Duration::from_millis(100)).unwrap_or(false) {
            if let Ok(Event::Key(KeyEvent {
                code, modifiers, ..
            })) = crossterm::event::read()
            {
                match code {
                    KeyCode::Esc => {
                        if app.search_mode {
                            app.search_mode = false;
                            app.search_query.clear();
                        } else {
                            should_quit = true;
                        }
                    }
                    KeyCode::Char('q') => {
                        if !app.search_mode {
                            should_quit = true;
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        app.select_previous();
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        app.select_next();
                    }
                    KeyCode::Char('/') => {
                        app.search_mode = !app.search_mode;
                        if !app.search_mode {
                            app.search_query.clear();
                        }
                    }
                    KeyCode::Char('u') => {
                        let pkg_name = app
                            .filtered_packages()
                            .get(app.selected_package_index)
                            .map(|(_, p)| p.name.clone());
                        if let Some(name) = pkg_name {
                            if let Some(pm) = app.active_pm() {
                                let result = runtime.block_on(pm.upgrade_package(&name));
                                match result {
                                    Ok(_) => {
                                        app.message = Some(format!("Upgraded {}", name));
                                        app.load_packages_sync();
                                    }
                                    Err(e) => {
                                        app.message = Some(format!("Error: {}", e));
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::Char('r') => {
                        let pkg_name = app
                            .filtered_packages()
                            .get(app.selected_package_index)
                            .map(|(_, p)| p.name.clone());
                        if let Some(name) = pkg_name {
                            if let Some(pm) = app.active_pm() {
                                let result = runtime.block_on(pm.remove_package(&name));
                                match result {
                                    Ok(_) => {
                                        app.message = Some(format!("Removed {}", name));
                                        app.load_packages_sync();
                                    }
                                    Err(e) => {
                                        app.message = Some(format!("Error: {}", e));
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::Char('i') => {
                        if !app.search_query.is_empty() {
                            let name = app.search_query.clone();
                            if let Some(pm) = app.active_pm() {
                                let result = runtime.block_on(pm.install_package(&name));
                                match result {
                                    Ok(_) => {
                                        app.message = Some(format!("Installed {}", name));
                                        app.load_packages_sync();
                                    }
                                    Err(e) => {
                                        app.message = Some(format!("Error: {}", e));
                                    }
                                }
                            }
                            app.search_query.clear();
                        }
                    }
                    KeyCode::Char('o') if modifiers.contains(KeyModifiers::CONTROL) => {
                        app.show_outdated_only = !app.show_outdated_only;
                    }
                    KeyCode::Char('R') if modifiers.contains(KeyModifiers::CONTROL) => {
                        app.load_packages_sync();
                    }
                    KeyCode::Tab => {
                        let pm_count = app.package_managers.len();
                        if pm_count > 0 {
                            app.active_pm_index = (app.active_pm_index + 1) % pm_count;
                            handle_pm_switch(&mut app, &runtime);
                        }
                    }
                    KeyCode::Backspace => {
                        app.search_query.pop();
                    }
                    KeyCode::Char(c) => {
                        if app.search_mode
                            && (c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '*')
                        {
                            app.search_query.push(c);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    {
        let backend = terminal.backend_mut();
        let _ = execute!(backend, LeaveAlternateScreen);
    }
    ratatui::restore();
}
