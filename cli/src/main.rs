use std::default::Default;
use std::sync::Arc;

use ratatui::{Frame, Terminal};
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use unicode_width::UnicodeWidthStr;
use clap::{ArgMatches};
use reqwest::Url;
use reqwest::cookie::Jar;

mod error;
mod input;

struct ServerInfo {
    url: Url,
}

impl Default for ServerInfo {
    fn default() -> Self {
        ServerInfo {
            url: Url::parse("http://localhost:80").unwrap()
        }
    }
}

struct UserInfo {
    username: String
}

impl Default for UserInfo {
    fn default() -> Self {
        UserInfo {
            username: String::from("unknown")
        }
    }
}

struct AppState {
    cookie_jar: Arc<Jar>,
    client: reqwest::blocking::Client,
    server: ServerInfo,
    user: UserInfo,
}

impl Default for AppState {
    fn default() -> Self {
        let cookie_jar = Arc::new(Jar::default());
        let client = reqwest::blocking::Client::builder()
            .cookie_provider(cookie_jar.clone())
            .user_agent("rfs-client-0.1.0")
            .build()
            .expect("failed to create client");

        AppState {
            cookie_jar,
            client,
            server: ServerInfo::default(),
            user: UserInfo::default(),
        }
    }
}

pub fn interactive_commands() -> clap::Command {
    use clap::{Command, Arg, ArgAction, value_parser};

    Command::new("")
        .subcommand_required(true)
        .no_binary_name(true)
        .disable_help_flag(true)
        .subcommand(
            Command::new("connect")
                .alias("c")
                .about("attempts to connect to the specified server")
                .disable_help_flag(true)
                .arg(
                    Arg::new("host")
                        .long("host")
                        .short('h')
                        .action(ArgAction::Set)
                        .default_value("localhost")
                        .help("the desired hostname to connect to")
                )
                .arg(
                    Arg::new("port")
                        .long("port")
                        .short('p')
                        .action(ArgAction::Set)
                        .default_value("80")
                        .value_parser(value_parser!(u16))
                        .help("the desired port to connect to")
                )
                .arg(
                    Arg::new("secure")
                        .long("secure")
                        .short('s')
                        .action(ArgAction::SetTrue)
                        .help("sets the connection to use https")
                )
        )
        .subcommand(
            Command::new("quit")
                .alias("q")
                .about("exits program")
        )
}

fn main() {
    let end_result = run();

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
}

fn run() -> error::Result<()> {
    let mut state = AppState::default();

    loop {
        let commands = interactive_commands();
        let given = input::read_stdin(">")?;
        let trimmed = given.trim();
        let Ok(args_list) = shell_words::split(&trimmed) else {
            println!("failed to parse command line args");
            break;
        };

        let matches = match commands.try_get_matches_from(args_list) {
            Ok(m) => m,
            Err(err) => {
                println!("{}", err);
                continue;
            }
        };

        match matches.subcommand() {
            Some(("connect", connect_matches)) => connect(&mut state, connect_matches)?,
            Some(("quit", quit_matches)) => {
                break;
            }
            _ => {
                println!("unknown command");
            }
        }
    }

    Ok(())
}

fn submit_user(
    state: &mut AppState
) -> error::Result<Option<lib::models::auth::AuthMethod>> {
    loop {
        let username = input::read_stdin_trimmed("username: ")?;

        let res = {
            let body = lib::actions::auth::RequestUser {
                username: username.clone()
            };

            state.client.post(state.server.url.join("/auth/request")?)
                .json(&body)
                .send()?
        };

        let status = res.status();

        if status != reqwest::StatusCode::OK {
            let json = res.json::<lib::json::Error>()?;

            if json.error() == "UserNotFound" {
                println!("requested username was not found");
                continue;
            }

            return Err(error::Error::new()
                .kind("FailedUserRequest")
                .message("failed to submit requested username")
                .source(format!("{:?}", json)));
        }

        println!("cookie_jar: {:?}", state.cookie_jar);

        state.user.username = username;

        let json = res.json::<lib::json::Wrapper<Option<lib::models::auth::AuthMethod>>>()?;

        return Ok(json.into_payload());
    }
}

fn submit_auth(
    state: &mut AppState,
    auth_method: lib::models::auth::AuthMethod
) -> error::Result<Option<lib::models::auth::VerifyMethod>> {
    match auth_method {
        lib::models::auth::AuthMethod::None => {
            println!("AuthMethod::None");

            let auth_method = lib::actions::auth::SubmitAuth::None;

            let res = state.client.post(state.server.url.join("/auth/submit")?)
                .json(&auth_method)
                .send()?;

            let status = res.status();

            if status != reqwest::StatusCode::OK {
                let json = res.json::<lib::json::Error>()?;

                return Err(error::Error::new()
                    .kind("FailedAuthentication")
                    .message("failed to submit requested auth method")
                    .source(format!("{:?}", json)));
            }

            let json = res.json::<lib::json::Wrapper<Option<lib::models::auth::VerifyMethod>>>()?;

            return Ok(json.into_payload());
        }
        lib::models::auth::AuthMethod::Password => {
            println!("AuthMethod::Password");

            let prompt = format!("{} password: ", state.user.username);

            loop {
                let password = rpassword::prompt_password(&prompt)?;
                let auth_method = lib::actions::auth::SubmitAuth::Password(password);

                let res = state.client.post(state.server.url.join("/auth/submit")?)
                    .json(&auth_method)
                    .send()?;

                let status = res.status();

                if status != reqwest::StatusCode::OK {
                    let json = res.json::<lib::json::Error>()?;

                    if json.error() == "InvalidPassword" {
                        println!("invalid password provided");
                        continue;
                    }

                    return Err(error::Error::new()
                        .kind("FailedAuthentication")
                        .message("failed to submit requested auth method")
                        .source(format!("{:?}", json)));
                }

                let json = res.json::<lib::json::Wrapper<Option<lib::models::auth::VerifyMethod>>>()?;

                return Ok(json.into_payload());
            }
        }
    };
}

fn submit_verify(
    state: &mut AppState,
    verify_method: lib::models::auth::VerifyMethod
) -> error::Result<()> {
    match verify_method {
        lib::models::auth::VerifyMethod::None => {},
        lib::models::auth::VerifyMethod::Totp{ digits } => {
            let prompt = format!("totp({}) code: ", digits);

            'input_loop: loop {
                let otp = input::read_stdin_trimmed(&prompt)?;

                if otp.len() != digits as usize {
                    println!("invalid totp code length");
                    continue;
                }

                for ch in otp.chars() {
                    if !ch.is_ascii_digit() {
                        println!("invalid totp characters providied");
                        continue 'input_loop;
                    }
                }

                let res = state.client.post(state.server.url.join("/auth/verify")?)
                    .json(&verify_method)
                    .send()?;

                let status = res.status();

                if status != reqwest::StatusCode::OK {
                    let json = res.json::<lib::json::Error>()?;

                    return Err(error::Error::new()
                        .kind("FailedVerification")
                        .message("failed to submit requested verification method")
                        .source(format!("{:?}", json)));
                }

                break;
            }
        }
    }

    Ok(())
}

fn connect(state: &mut AppState, args: &ArgMatches) -> error::Result<()> {
    use lib::actions;
    use lib::models;

    let host = args.get_one::<String>("host").unwrap();
    let port = args.get_one("port")
        .map(|v: &u16| v.clone())
        .unwrap();

    if args.get_flag("secure") {
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

    let Some(auth_method) = submit_user(state)? else {
        println!("current session has already been authenticated");
        return Ok(());
    };

    let Some(verify_method) = submit_auth(state, auth_method)? else {
        println!("current session has already been authenticated");
        return Ok(());
    };

    submit_verify(state, verify_method)?;

    println!("session authenticated");

    Ok(())
}

/*

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
