use std::path::PathBuf;

use clap::{ArgMatches};

mod error;
mod input;
mod state;
mod auth;
mod util;
mod commands;

fn print_error(err: error::Error) {
    match err.into_parts() {
        (kind, Some(msg), Some(err)) => {
            println!("{}: {}\n{}", kind, msg, err);
        },
        (kind, Some(msg), None) => {
            println!("{}: {}", kind, msg);
        },
        (kind, None, Some(err)) => {
            println!("{}: {}", kind, err);
        },
        (kind, None, None) => {
            println!("{}", kind);
        }
    }
}

fn main() {
    use tracing_subscriber::{FmtSubscriber, EnvFilter};

    FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init()
        .expect("failed to initialize global tracing subscriber");

    let end_result = run();

    if let Err(err) = end_result {
        print_error(err);
    }
}

fn run() -> error::Result<()> {
    let app_matches = commands::cli().get_matches();

    let session_file = if let Some(arg) = app_matches.get_one::<PathBuf>("cookies") {
        arg.clone()
    } else {
        let mut current_dir = std::env::current_dir()?;
        current_dir.push("rfs_cookies.json");
        current_dir
    };

    let mut state = state::AppState::load(session_file)?;

    let host = app_matches.get_one::<String>("host").unwrap();
    let port = app_matches.get_one("port")
        .map(|v: &u16| v.clone())
        .unwrap();

    if app_matches.get_flag("secure") {
        state.server.url.set_scheme("https").unwrap();
    } else {
        state.server.url.set_scheme("http").unwrap();
    }

    if state.server.url.set_host(Some(host)).is_err() {
        println!("cannot set host to the value provided. {}", host);
        return Ok(());
    }

    if state.server.url.set_port(Some(port)).is_err() {
        println!("cannot set port to the value provided. {}", port);
        return Ok(());
    }

    match app_matches.subcommand() {
        None => {
            loop {
                let given = input::read_stdin(">")?;
                let trimmed = given.trim();

                let Ok(args_list) = shell_words::split(&trimmed) else {
                    println!("failed to parse command line args");
                    continue;
                };

                let matches = match commands::interactive().try_get_matches_from(args_list) {
                    Ok(m) => m,
                    Err(err) => {
                        println!("{}", err);
                        continue;
                    }
                };

                let result = match matches.subcommand() {
                    Some(("quit", _quit_matches)) => {
                        return Ok(());
                    },
                    Some((cmd, cmd_matches)) => run_subcommand(&mut state, cmd, cmd_matches),
                    _ => unreachable!()
                };

                if let Err(err) = result {
                    print_error(err);
                }
            }
        },
        Some((cmd, cmd_matches)) => run_subcommand(&mut state, cmd, cmd_matches)?
    }

    Ok(())
}

fn run_subcommand(state: &mut state::AppState, command: &str, matches: &ArgMatches) -> error::Result<()> {
    match command {
        "connect" => commands::connect(state, matches),
        "disconnect" => commands::disconnect(state, matches),
        "hash" => commands::hash(state, matches),
        "storage" => commands::storage(state, matches),
        "fs" => commands::fs(state, matches),
        "user" => commands::user(state, matches),
        _ => {
            println!("uknown command");

            Ok(())
        }
    }
}

/*
use ratatui::{Frame, Terminal};
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use unicode_width::UnicodeWidthStr;

type AppBackend = CrosstermBackend<std::io::Stdout>;

enum InputMode {
    Normal,
    Editing,
}

struct AppState {
    input: String,
    input_mode: InputMode,
    messages: Vec<String>,
}

impl AppState {
    fn new() -> Self {
        Self {
            input: String::new(),
            input_mode: InputMode::Normal,
            messages: Vec::new()
        }
    }
}

fn main() {
    let mut stdout = std::io::stdout();

    crossterm::terminal::enable_raw_mode()
        .expect("failed to enable terminal raw mode");

    crossterm::execute!(
        stdout,
        crossterm::terminal::EnterAlternateScreen,
    ).expect("failed to execute commands on stdout");

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)
        .expect("failed to create terminal");

    let end_result = run(&mut terminal);

    crossterm::terminal::disable_raw_mode()
        .expect("failed to disable raw mode");

    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen,
    ).expect("failed to execute command on stdout");

    if let Err(err) = end_result {
        match err.into_parts() {
            (kind, Some(msg), Some(err)) => {
                println!("{}: {}\n{}", kind, msg, err);
            },
            (kind, Some(msg), None) => {
                println!("{}: {}", kind, msg);
            },
            (kind, None, Some(err)) => {
                println!("{}: {}", kind, err);
            },
            (kind, None, None) => {
                println!("{}", kind);
            }
        }
    }

    terminal.show_cursor()
        .expect("failed to show terminal cursor");
}

fn run(terminal: &mut Terminal<AppBackend>) -> error::Result<()> {
    use crossterm::event::{self, Event, KeyCode, KeyEventKind};

    let mut app_state = AppState::new();

    loop {
        terminal.draw(|f| ui(f, &app_state))?;

        if let Event::Key(key) = event::read()? {
            match app_state.input_mode {
                InputMode::Normal => match key.code {
                    KeyCode::Char('e') => {
                        app_state.input_mode = InputMode::Editing;
                    }
                    KeyCode::Char('q') => {
                        return Ok(());
                    }
                    _ => {}
                },
                InputMode::Editing if key.kind == KeyEventKind::Press => match key.code {
                    KeyCode::Enter => {
                        app_state.messages.push(app_state.input.drain(..).collect());
                    }
                    KeyCode::Char(c) => {
                        app_state.input.push(c);
                    }
                    KeyCode::Backspace => {
                        app_state.input.pop();
                    }
                    KeyCode::Esc => {
                        app_state.input_mode = InputMode::Normal;
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }
}

fn ui(frame: &mut Frame<AppBackend>, app: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints(vec![
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(3),
        ])
        .split(frame.size());

    let (msg, style) = match app.input_mode {
        InputMode::Normal => (
            vec![
                Span::raw("Press "),
                Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to exit, "),
                Span::styled("e", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to start editing."),
            ],
            Style::default().add_modifier(Modifier::RAPID_BLINK),
        ),
        InputMode::Editing => (
            vec![
                Span::raw("Press "),
                Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to stop editing, "),
                Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to record the message"),
            ],
            Style::default(),
        ),
    };

    let mut text = Text::from(Line::from(msg));
    text.patch_style(style);

    let help_message = Paragraph::new(text);

    frame.render_widget(help_message, chunks[1]);

    let input = Paragraph::new(app.input.as_str())
        .style(match app.input_mode {
            InputMode::Normal => Style::default(),
            InputMode::Editing => Style::default().fg(Color::Yellow),
        })
        .block(Block::default().title("Input"));

    frame.render_widget(input, chunks[2]);

    match app.input_mode {
        InputMode::Normal =>
            // Hide the cursor. `Frame` does this by default, so we don't need to do anything here
            {}

        InputMode::Editing => {
            // Make the cursor visible and ask ratatui to put it at the specified coordinates after
            // rendering
            frame.set_cursor(
                // Put cursor past the end of the input text
                chunks[2].x + app.input.width() as u16,
                // Move one line down, from the border to the input line
                chunks[2].y + 1,
            )
        }
    }

    let messages: Vec<ListItem> = app
        .messages
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let content = Line::from(Span::raw(format!("{i}: {m}")));
            ListItem::new(content)
        })
        .collect();
    let messages = List::new(messages).block(Block::default().title("Messages"));

    frame.render_widget(messages, chunks[0]);
}
*/
