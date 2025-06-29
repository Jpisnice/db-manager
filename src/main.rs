mod credentials;
mod database;
mod docker;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;
use tokio::runtime::Runtime;

use credentials::{AppConfig, DbCredentials, DecryptedDbInfo};
use database::DbType;

#[derive(Debug, Clone)]
enum AppState {
    Authentication,
    MainMenu,
    DatabaseList,
    CreateDatabase,
    DatabaseDetails(String),
    Error(String),
    ResetConfirmation,
}

#[derive(Debug, Clone)]
enum CreateDatabaseStep {
    Name,
    Type,
    Username,
    Password,
    Database,
    Port,
    RootPassword, // For MySQL
    Confirm,
}

struct App {
    state: AppState,
    should_quit: bool,
    passphrase: String,
    input_buffer: String,
    config: Option<AppConfig>,
    
    // Menu navigation
    menu_selection: usize,
    list_state: ListState,
    
    // Database creation
    create_step: CreateDatabaseStep,
    new_db_name: String,
    new_db_type: String,
    new_db_username: String,
    new_db_password: String,
    new_db_database: String,
    new_db_port: String,
    new_db_root_password: String,
    
    // Database list
    databases: Vec<DecryptedDbInfo>,
    
    // Error/status messages
    status_message: Option<String>,
    error_message: Option<String>,
    
    // Runtime for async operations
    rt: Runtime,
}

impl App {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let rt = Runtime::new()?;
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        
        Ok(App {
            state: AppState::Authentication,
            should_quit: false,
            passphrase: String::new(),
            input_buffer: String::new(),
            config: None,
            menu_selection: 0,
            list_state,
            create_step: CreateDatabaseStep::Name,
            new_db_name: String::new(),
            new_db_type: "postgres".to_string(),
            new_db_username: String::new(),
            new_db_password: String::new(),
            new_db_database: String::new(),
            new_db_port: "5432".to_string(),
            new_db_root_password: String::new(),
            databases: Vec::new(),
            status_message: None,
            error_message: None,
            rt,
        })
    }

    fn handle_key_event(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        // Clear status messages on any key press
        self.status_message = None;
        self.error_message = None;

        match &self.state {
            AppState::Authentication => self.handle_auth_input(key),
            AppState::MainMenu => self.handle_main_menu_input(key),
            AppState::DatabaseList => self.handle_database_list_input(key),
            AppState::CreateDatabase => self.handle_create_database_input(key),
            AppState::DatabaseDetails(_) => self.handle_database_details_input(key),
            AppState::Error(_) => self.handle_error_input(key),
            AppState::ResetConfirmation => self.handle_reset_confirmation_input(key),
        }
    }

    fn handle_auth_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                if !self.input_buffer.is_empty() {
                    self.passphrase = self.input_buffer.clone();
                    self.input_buffer.clear();
                    self.authenticate();
                }
            }
            KeyCode::Esc => {
                self.should_quit = true;
            }
            KeyCode::Char(c) => {
                self.input_buffer.push(c);
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
            }
            KeyCode::F(1) => {
                // F1 key to reset configuration
                if credentials::AppConfig::config_exists() {
                    self.state = AppState::ResetConfirmation;
                } else {
                    self.error_message = Some("No configuration file found to reset.".to_string());
                }
            }
            _ => {}
        }
    }

    fn handle_main_menu_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up => {
                if self.menu_selection > 0 {
                    self.menu_selection -= 1;
                }
            }
            KeyCode::Down => {
                if self.menu_selection < 3 {
                    self.menu_selection += 1;
                }
            }
            KeyCode::Enter => {
                match self.menu_selection {
                    0 => {
                        self.load_databases();
                        self.state = AppState::DatabaseList;
                    }
                    1 => {
                        self.reset_create_database_form();
                        self.state = AppState::CreateDatabase;
                    }
                    2 => {
                        // Refresh databases
                        self.load_databases();
                        self.status_message = Some("Database list refreshed".to_string());
                    }
                    3 => {
                        self.should_quit = true;
                    }
                    _ => {}
                }
            }
            KeyCode::Esc => {
                self.should_quit = true;
            }
            _ => {}
        }
    }

    fn handle_database_list_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up => {
                if let Some(selected) = self.list_state.selected() {
                    if selected > 0 {
                        self.list_state.select(Some(selected - 1));
                    }
                }
            }
            KeyCode::Down => {
                if let Some(selected) = self.list_state.selected() {
                    if selected < self.databases.len().saturating_sub(1) {
                        self.list_state.select(Some(selected + 1));
                    }
                }
            }
            KeyCode::Enter => {
                if let Some(selected) = self.list_state.selected() {
                    if selected < self.databases.len() {
                        let db_name = self.databases[selected].name.clone();
                        self.state = AppState::DatabaseDetails(db_name);
                    }
                }
            }
            KeyCode::Esc => {
                self.state = AppState::MainMenu;
            }
            KeyCode::Char('c') => {
                self.reset_create_database_form();
                self.state = AppState::CreateDatabase;
            }
            KeyCode::Char('r') => {
                self.load_databases();
                self.status_message = Some("Database list refreshed".to_string());
            }
            _ => {}
        }
    }

    fn handle_create_database_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                self.advance_create_step();
            }
            KeyCode::Esc => {
                self.state = AppState::MainMenu;
            }
            KeyCode::Char(c) => {
                match &self.create_step {
                    CreateDatabaseStep::Name => self.new_db_name.push(c),
                    CreateDatabaseStep::Username => self.new_db_username.push(c),
                    CreateDatabaseStep::Password => self.new_db_password.push(c),
                    CreateDatabaseStep::Database => self.new_db_database.push(c),
                    CreateDatabaseStep::Port => {
                        if c.is_ascii_digit() {
                            self.new_db_port.push(c);
                        }
                    }
                    CreateDatabaseStep::RootPassword => self.new_db_root_password.push(c),
                    CreateDatabaseStep::Type => {
                        // Handle type selection with numbers
                        match c {
                            '1' => {
                                self.new_db_type = "postgres".to_string();
                                self.new_db_port = "5432".to_string();
                            }
                            '2' => {
                                self.new_db_type = "mysql".to_string();
                                self.new_db_port = "3306".to_string();
                            }
                            '3' => {
                                self.new_db_type = "redis".to_string();
                                self.new_db_port = "6379".to_string();
                            }
                            _ => {}
                        }
                    }
                    CreateDatabaseStep::Confirm => {}
                }
            }
            KeyCode::Backspace => {
                match &self.create_step {
                    CreateDatabaseStep::Name => { self.new_db_name.pop(); }
                    CreateDatabaseStep::Username => { self.new_db_username.pop(); }
                    CreateDatabaseStep::Password => { self.new_db_password.pop(); }
                    CreateDatabaseStep::Database => { self.new_db_database.pop(); }
                    CreateDatabaseStep::Port => { self.new_db_port.pop(); }
                    CreateDatabaseStep::RootPassword => { self.new_db_root_password.pop(); }
                    _ => {}
                }
            }
            KeyCode::Tab => {
                // Navigate between database types
                if matches!(self.create_step, CreateDatabaseStep::Type) {
                    match self.new_db_type.as_str() {
                        "postgres" => {
                            self.new_db_type = "mysql".to_string();
                            self.new_db_port = "3306".to_string();
                        }
                        "mysql" => {
                            self.new_db_type = "redis".to_string();
                            self.new_db_port = "6379".to_string();
                        }
                        "redis" => {
                            self.new_db_type = "postgres".to_string();
                            self.new_db_port = "5432".to_string();
                        }
                        _ => {
                            self.new_db_type = "postgres".to_string();
                            self.new_db_port = "5432".to_string();
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_database_details_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.state = AppState::DatabaseList;
            }
            KeyCode::Char('d') => {
                // Delete database
                if let AppState::DatabaseDetails(ref name) = self.state.clone() {
                    self.delete_database(name.clone());
                }
            }
            _ => {}
        }
    }

    fn handle_error_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter | KeyCode::Esc => {
                self.state = AppState::MainMenu;
            }
            _ => {}
        }
    }

    fn handle_reset_confirmation_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                // User confirmed reset
                match credentials::AppConfig::reset_config() {
                    Ok(()) => {
                        self.status_message = Some("Configuration reset successfully! You can now set a new passphrase.".to_string());
                        self.state = AppState::Authentication;
                        self.passphrase.clear();
                        self.input_buffer.clear();
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Failed to reset configuration: {}", e));
                        self.state = AppState::Authentication;
                    }
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                // User cancelled reset
                self.state = AppState::Authentication;
            }
            _ => {}
        }
    }

    fn authenticate(&mut self) {
        match AppConfig::load_or_create(&self.passphrase) {
            Ok(config) => {
                self.config = Some(config);
                self.state = AppState::MainMenu;
                self.status_message = Some("Authentication successful!".to_string());
            }
            Err(e) => {
                self.error_message = Some(format!("Authentication failed: {}", e));
                self.passphrase.clear();
            }
        }
    }

    fn load_databases(&mut self) {
        if let Some(ref config) = self.config {
            match config.get_all_databases(&self.passphrase) {
                Ok(databases) => {
                    self.databases = databases;
                    self.list_state.select(Some(0));
                }
                Err(e) => {
                    self.error_message = Some(format!("Failed to load databases: {}", e));
                }
            }
        }
    }

    fn reset_create_database_form(&mut self) {
        self.create_step = CreateDatabaseStep::Name;
        self.new_db_name.clear();
        self.new_db_type = "postgres".to_string();
        self.new_db_username.clear();
        self.new_db_password.clear();
        self.new_db_database.clear();
        self.new_db_port = "5432".to_string();
        self.new_db_root_password.clear();
    }

    fn advance_create_step(&mut self) {
        match &self.create_step {
            CreateDatabaseStep::Name => {
                if !self.new_db_name.is_empty() {
                    self.create_step = CreateDatabaseStep::Type;
                }
            }
            CreateDatabaseStep::Type => {
                // Set default port based on database type
                match self.new_db_type.as_str() {
                    "postgres" => self.new_db_port = "5432".to_string(),
                    "mysql" => self.new_db_port = "3306".to_string(),
                    "redis" => self.new_db_port = "6379".to_string(),
                    _ => {}
                }
                self.create_step = CreateDatabaseStep::Username;
            }
            CreateDatabaseStep::Username => {
                if !self.new_db_username.is_empty() {
                    self.create_step = CreateDatabaseStep::Password;
                }
            }
            CreateDatabaseStep::Password => {
                if !self.new_db_password.is_empty() {
                    if self.new_db_type == "redis" {
                        self.create_step = CreateDatabaseStep::Port;
                    } else {
                        self.create_step = CreateDatabaseStep::Database;
                    }
                }
            }
            CreateDatabaseStep::Database => {
                if !self.new_db_database.is_empty() {
                    self.create_step = CreateDatabaseStep::Port;
                }
            }
            CreateDatabaseStep::Port => {
                if !self.new_db_port.is_empty() {
                    if self.new_db_type == "mysql" {
                        self.create_step = CreateDatabaseStep::RootPassword;
                    } else {
                        self.create_step = CreateDatabaseStep::Confirm;
                    }
                }
            }
            CreateDatabaseStep::RootPassword => {
                self.create_step = CreateDatabaseStep::Confirm;
            }
            CreateDatabaseStep::Confirm => {
                self.create_database();
            }
        }
    }

    fn create_database(&mut self) {
        if let Some(ref mut config) = self.config {
            let credentials = DbCredentials {
                username: self.new_db_username.clone(),
                password: self.new_db_password.clone(),
                database: if self.new_db_type == "redis" {
                    "0".to_string() // Redis database number
                } else {
                    self.new_db_database.clone()
                },
                port: self.new_db_port.parse().unwrap_or(5432),
                root_password: if self.new_db_type == "mysql" && !self.new_db_root_password.is_empty() {
                    Some(self.new_db_root_password.clone())
                } else {
                    None
                },
            };

            let result = self.rt.block_on(async {
                config.create_database(
                    self.new_db_name.clone(),
                    self.new_db_type.clone(),
                    credentials,
                    &self.passphrase,
                ).await
            });

            match result {
                Ok(()) => {
                    self.status_message = Some(format!("Database '{}' created successfully!", self.new_db_name));
                    self.state = AppState::MainMenu;
                    self.load_databases();
                }
                Err(e) => {
                    self.error_message = Some(format!("Failed to create database: {}", e));
                }
            }
        }
    }

    fn delete_database(&mut self, name: String) {
        if let Some(ref mut config) = self.config {
            match config.remove_database(&name) {
                Ok(()) => {
                    self.status_message = Some(format!("Database '{}' deleted successfully!", name));
                    self.state = AppState::DatabaseList;
                    self.load_databases();
                }
                Err(e) => {
                    self.error_message = Some(format!("Failed to delete database: {}", e));
                }
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Check for command line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "--reset" | "-r" => {
                println!("🗄️  Database Manager - Configuration Reset");
                println!();
                
                if !credentials::AppConfig::config_exists() {
                    println!("No configuration file found. Nothing to reset.");
                    return Ok(());
                }
                
                print!("⚠️  WARNING: This will delete all stored database configurations! Are you sure? (y/N): ");
                std::io::Write::flush(&mut std::io::stdout())?;
                
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                
                if input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes" {
                    credentials::AppConfig::reset_config()?;
                    println!("✅ Configuration reset successfully!");
                    println!("You can now run the application with a new passphrase.");
                } else {
                    println!("Reset cancelled.");
                }
                return Ok(());
            }
            "--help" | "-h" => {
                println!("🗄️  Database Manager");
                println!();
                println!("Usage:");
                println!("  db-tool                 Launch the interactive interface");
                println!("  db-tool --reset         Reset configuration (delete all stored databases)");
                println!("  db-tool --help          Show this help message");
                println!();
                println!("Interactive Controls:");
                println!("  F1                      Reset configuration (when on login screen)");
                println!("  Esc                     Quit application");
                return Ok(());
            }
            _ => {
                println!("Unknown argument: {}", args[1]);
                println!("Use --help for usage information.");
                return Ok(());
            }
        }
    }

    // Initialize terminal
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(
        stdout,
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run
    let mut app = App::new()?;
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        println!("Error: {}", err);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if crossterm::event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                app.handle_key_event(key);
            }
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}

fn ui(f: &mut Frame, app: &App) {
    match &app.state {
        AppState::Authentication => draw_auth_screen(f, app),
        AppState::MainMenu => draw_main_menu(f, app),
        AppState::DatabaseList => draw_database_list(f, app),
        AppState::CreateDatabase => draw_create_database(f, app),
        AppState::DatabaseDetails(name) => draw_database_details(f, app, name),
        AppState::Error(msg) => draw_error_screen(f, msg),
        AppState::ResetConfirmation => draw_reset_confirmation(f, app),
    }

    // Draw status/error messages as overlays
    if let Some(ref msg) = app.status_message {
        draw_status_popup(f, msg, Color::Green);
    }
    if let Some(ref msg) = app.error_message {
        draw_status_popup(f, msg, Color::Red);
    }
}

fn draw_auth_screen(f: &mut Frame, app: &App) {
    let area = f.area();
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Percentage(30),
        ])
        .split(area);

    let title = Paragraph::new("🗄️  Database Manager")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(title, chunks[0]);

    let prompt = Paragraph::new("Enter your passphrase:")
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center);
    f.render_widget(prompt, chunks[1]);

    let password_display = "*".repeat(app.input_buffer.len());
    let input = Paragraph::new(password_display)
        .style(Style::default().fg(Color::Yellow))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Passphrase"));
    f.render_widget(input, chunks[2]);

    let help = Paragraph::new("Enter: Authenticate | F1: Reset Config (if forgot passphrase) | Esc: Quit")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);
    f.render_widget(help, chunks[3]);
}

fn draw_main_menu(f: &mut Frame, app: &App) {
    let area = f.area();
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .split(area);

    let title = Paragraph::new("🗄️  Database Manager - Main Menu")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    let menu_items = vec![
        "📋 List Databases",
        "➕ Create Database", 
        "🔄 Refresh",
        "❌ Exit",
    ];

    let items: Vec<ListItem> = menu_items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let style = if i == app.menu_selection {
                Style::default().fg(Color::Black).bg(Color::White)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(*item).style(style)
        })
        .collect();

    let menu = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Menu"))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));
    f.render_widget(menu, chunks[1]);

    let help = Paragraph::new("Use ↑↓ arrows to navigate, Enter to select, Esc to quit")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Help"));
    f.render_widget(help, chunks[2]);
}

fn draw_database_list(f: &mut Frame, app: &App) {
    let area = f.area();
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .split(area);

    let title = Paragraph::new("📋 Database List")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    if app.databases.is_empty() {
        let empty_msg = Paragraph::new("No databases found. Press 'c' to create one.")
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Databases"));
        f.render_widget(empty_msg, chunks[1]);
    } else {
        let items: Vec<ListItem> = app.databases
            .iter()
            .map(|db| {
                let type_icon = match db.db_type {
                    DbType::Postgres => "🐘",
                    DbType::MySQL => "🐬", 
                    DbType::Redis => "🔴",
                };
                let line = format!("{} {} ({}:{})", type_icon, db.name, db.credentials.username, db.credentials.port);
                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Databases"))
            .highlight_style(Style::default().fg(Color::Black).bg(Color::White))
            .highlight_symbol("▶ ");
        f.render_stateful_widget(list, chunks[1], &mut app.list_state.clone());
    }

    let help = Paragraph::new("↑↓: Navigate | Enter: Details | c: Create | r: Refresh | Esc: Back")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Help"));
    f.render_widget(help, chunks[2]);
}

fn draw_create_database(f: &mut Frame, app: &App) {
    let area = f.area();
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(area);

    let title = Paragraph::new("➕ Create New Database")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    let form_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(2),
        ])
        .split(chunks[1]);

    // Name field
    let (name_style, name_border_style) = if matches!(app.create_step, CreateDatabaseStep::Name) {
        (Style::default().fg(Color::Black).bg(Color::Yellow), Style::default().fg(Color::Yellow))
    } else {
        (Style::default().fg(Color::White), Style::default().fg(Color::Gray))
    };
    let name_display = if app.new_db_name.is_empty() && matches!(app.create_step, CreateDatabaseStep::Name) {
        "█".to_string() // Show cursor when field is active and empty
    } else if matches!(app.create_step, CreateDatabaseStep::Name) {
        format!("{}█", app.new_db_name) // Show cursor at end when active
    } else {
        app.new_db_name.clone()
    };
    let name_widget = Paragraph::new(name_display)
        .style(name_style)
        .block(Block::default().borders(Borders::ALL).title("Database Name").border_style(name_border_style));
    f.render_widget(name_widget, form_chunks[0]);

    // Type selection
    let (type_style, type_border_style) = if matches!(app.create_step, CreateDatabaseStep::Type) {
        (Style::default().fg(Color::Black).bg(Color::Yellow), Style::default().fg(Color::Yellow))
    } else {
        (Style::default().fg(Color::White), Style::default().fg(Color::Gray))
    };
    let type_text = format!("1) PostgreSQL  2) MySQL  3) Redis    Selected: {} (Port: {})", app.new_db_type, app.new_db_port);
    let type_widget = Paragraph::new(type_text)
        .style(type_style)
        .block(Block::default().borders(Borders::ALL).title("Database Type").border_style(type_border_style));
    f.render_widget(type_widget, form_chunks[1]);

    // Username field
    let (username_style, username_border_style) = if matches!(app.create_step, CreateDatabaseStep::Username) {
        (Style::default().fg(Color::Black).bg(Color::Yellow), Style::default().fg(Color::Yellow))
    } else {
        (Style::default().fg(Color::White), Style::default().fg(Color::Gray))
    };
    let username_display = if app.new_db_username.is_empty() && matches!(app.create_step, CreateDatabaseStep::Username) {
        "█".to_string()
    } else if matches!(app.create_step, CreateDatabaseStep::Username) {
        format!("{}█", app.new_db_username)
    } else {
        app.new_db_username.clone()
    };
    let username_widget = Paragraph::new(username_display)
        .style(username_style)
        .block(Block::default().borders(Borders::ALL).title("Username").border_style(username_border_style));
    f.render_widget(username_widget, form_chunks[2]);

    // Password field
    let (password_style, password_border_style) = if matches!(app.create_step, CreateDatabaseStep::Password) {
        (Style::default().fg(Color::Black).bg(Color::Yellow), Style::default().fg(Color::Yellow))
    } else {
        (Style::default().fg(Color::White), Style::default().fg(Color::Gray))
    };
    let password_display = if app.new_db_password.is_empty() && matches!(app.create_step, CreateDatabaseStep::Password) {
        "█".to_string()
    } else if matches!(app.create_step, CreateDatabaseStep::Password) {
        format!("{}█", "*".repeat(app.new_db_password.len()))
    } else {
        "*".repeat(app.new_db_password.len())
    };
    let password_widget = Paragraph::new(password_display)
        .style(password_style)
        .block(Block::default().borders(Borders::ALL).title("Password").border_style(password_border_style));
    f.render_widget(password_widget, form_chunks[3]);

    // Database name field (skip for Redis)
    if app.new_db_type != "redis" {
        let (db_style, db_border_style) = if matches!(app.create_step, CreateDatabaseStep::Database) {
            (Style::default().fg(Color::Black).bg(Color::Yellow), Style::default().fg(Color::Yellow))
        } else {
            (Style::default().fg(Color::White), Style::default().fg(Color::Gray))
        };
        let db_display = if app.new_db_database.is_empty() && matches!(app.create_step, CreateDatabaseStep::Database) {
            "█".to_string()
        } else if matches!(app.create_step, CreateDatabaseStep::Database) {
            format!("{}█", app.new_db_database)
        } else {
            app.new_db_database.clone()
        };
        let db_widget = Paragraph::new(db_display)
            .style(db_style)
            .block(Block::default().borders(Borders::ALL).title("Database Name").border_style(db_border_style));
        f.render_widget(db_widget, form_chunks[4]);
    }

    // Port field
    let (port_style, port_border_style) = if matches!(app.create_step, CreateDatabaseStep::Port) {
        (Style::default().fg(Color::Black).bg(Color::Yellow), Style::default().fg(Color::Yellow))
    } else {
        (Style::default().fg(Color::White), Style::default().fg(Color::Gray))
    };
    let port_display = if app.new_db_port.is_empty() && matches!(app.create_step, CreateDatabaseStep::Port) {
        "█".to_string()
    } else if matches!(app.create_step, CreateDatabaseStep::Port) {
        format!("{}█", app.new_db_port)
    } else {
        app.new_db_port.clone()
    };
    let port_widget = Paragraph::new(port_display)
        .style(port_style)
        .block(Block::default().borders(Borders::ALL).title("Port").border_style(port_border_style));
    f.render_widget(port_widget, form_chunks[5]);

    // Root password field (MySQL only)
    if app.new_db_type == "mysql" {
        let (root_style, root_border_style) = if matches!(app.create_step, CreateDatabaseStep::RootPassword) {
            (Style::default().fg(Color::Black).bg(Color::Yellow), Style::default().fg(Color::Yellow))
        } else {
            (Style::default().fg(Color::White), Style::default().fg(Color::Gray))
        };
        let root_display = if app.new_db_root_password.is_empty() && matches!(app.create_step, CreateDatabaseStep::RootPassword) {
            "█".to_string()
        } else if matches!(app.create_step, CreateDatabaseStep::RootPassword) {
            format!("{}█", "*".repeat(app.new_db_root_password.len()))
        } else {
            "*".repeat(app.new_db_root_password.len())
        };
        let root_widget = Paragraph::new(root_display)
            .style(root_style)
            .block(Block::default().borders(Borders::ALL).title("Root Password").border_style(root_border_style));
        f.render_widget(root_widget, form_chunks[6]);
    }

    // Confirmation
    if matches!(app.create_step, CreateDatabaseStep::Confirm) {
        let confirm_text = "Press Enter to create database";
        let confirm_widget = Paragraph::new(confirm_text)
            .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Confirm"));
        f.render_widget(confirm_widget, form_chunks[7]);
    }

    let help = Paragraph::new("Enter: Next field | Tab/Numbers: Change type (auto-sets port) | Backspace: Delete | Esc: Cancel")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Help"));
    f.render_widget(help, chunks[2]);
}

fn draw_database_details(f: &mut Frame, app: &App, name: &str) {
    let area = f.area();
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .split(area);

    let title = Paragraph::new(format!("🔍 Database Details: {}", name))
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    if let Some(db) = app.databases.iter().find(|d| d.name == *name) {
        let type_icon = match db.db_type {
            DbType::Postgres => "🐘",
            DbType::MySQL => "🐬",
            DbType::Redis => "🔴",
        };

        let details = format!(
            "{} Type: {:?}\n\n📦 Container: {}\n\n👤 Username: {}\n\n🏠 Host: localhost:{}\n\n🗄️ Database: {}\n\n🔗 Connection: {}\n\n📅 Created: {}",
            type_icon,
            db.db_type,
            db.container_id,
            db.credentials.username,
            db.credentials.port,
            db.credentials.database,
            db.connection_string,
            db.created_at.format("%Y-%m-%d %H:%M:%S")
        );

        let details_widget = Paragraph::new(details)
            .style(Style::default().fg(Color::White))
            .block(Block::default().borders(Borders::ALL).title("Information"))
            .wrap(Wrap { trim: true });
        f.render_widget(details_widget, chunks[1]);
    }

    let help = Paragraph::new("d: Delete database | Esc: Back to list")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Help"));
    f.render_widget(help, chunks[2]);
}

fn draw_error_screen(f: &mut Frame, msg: &str) {
    let area = f.area();
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Length(5),
            Constraint::Percentage(30),
        ])
        .split(area);

    let error_widget = Paragraph::new(msg)
        .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Error"))
        .wrap(Wrap { trim: true });
    f.render_widget(error_widget, chunks[1]);
}

fn draw_reset_confirmation(f: &mut Frame, _app: &App) {
    let area = f.area();
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Length(8),
            Constraint::Length(3),
            Constraint::Percentage(20),
        ])
        .split(area);

    let title = Paragraph::new("⚠️  Reset Configuration")
        .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(title, chunks[0]);

    let warning_text = "WARNING: This will permanently delete all stored database configurations!\n\nAll saved database connections will be lost and you'll need to recreate them.\n\nThis action cannot be undone.\n\nAre you sure you want to reset the configuration?";
    let warning = Paragraph::new(warning_text)
        .style(Style::default().fg(Color::Yellow))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("⚠️  Warning"))
        .wrap(Wrap { trim: true });
    f.render_widget(warning, chunks[1]);

    let help = Paragraph::new("Y: Yes, reset configuration | N: No, go back | Esc: Cancel")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Options"));
    f.render_widget(help, chunks[2]);
}

fn draw_status_popup(f: &mut Frame, msg: &str, color: Color) {
    let area = f.area();
    let popup_area = Rect {
        x: area.width / 4,
        y: area.height / 2,
        width: area.width / 2,
        height: 3,
    };

    f.render_widget(Clear, popup_area);
    let popup = Paragraph::new(msg)
        .style(Style::default().fg(color).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Status"));
    f.render_widget(popup, popup_area);
}
