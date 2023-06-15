use std::default::Default;
use std::sync::Arc;
use std::path::PathBuf;

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

impl ServerInfo {
    pub fn new() -> Self {
        ServerInfo {
            url: Url::parse("http://localhost:80").unwrap()
        }
    }
}

type CookieStore = reqwest_cookie_store::CookieStore;
type CookieStoreSync = reqwest_cookie_store::CookieStoreRwLock;

struct AppState {
    cookie_file: PathBuf,
    store: Arc<CookieStoreSync>,
    client: reqwest::blocking::Client,
    server: ServerInfo,
}

impl AppState {
    pub fn new<P>(given_file: P) -> error::Result<Self>
    where
        P: AsRef<std::path::Path>
    {
        let given_file_ref = given_file.as_ref();

        let store = if given_file_ref.try_exists()? {
            let file = std::fs::OpenOptions::new()
                .read(true)
                .open(given_file_ref)?;
            let reader = std::io::BufReader::new(file);

            CookieStore::load_json(reader)
                .map_err(|e| error::Error::new()
                    .kind("FailedLoadingRFSCookies")
                    .message("failed to load the requested cookies file")
                    .source(e))?
        } else {
            CookieStore::default()
        };

        let store = Arc::new(CookieStoreSync::new(store));
        let client = reqwest::blocking::Client::builder()
            .cookie_provider(store.clone())
            .user_agent("rfs-client-0.1.0")
            .build()
            .expect("failed to create client");

        Ok(AppState {
            cookie_file: given_file_ref.to_owned(),
            store,
            client,
            server: ServerInfo::new(),
        })
    }

    pub fn save_store(&self) -> error::Result<()> {
        let store = self.store.read()
            .map_err(|_e| error::Error::new()
                .kind("RwLockPoisoned")
                .message("something has caused the RwLock to be poisoned"))?;
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(&self.cookie_file)
            .map_err(|e| error::Error::new()
                .kind("FailedSavingRFSCookies")
                .message("failed to open the rfs cookies file")
                .source(e))?;
        let mut writer = std::io::BufWriter::new(file);

        store.save_json(&mut writer)
            .map_err(|e| error::Error::new()
                .kind("FailedSavingRFSCookies")
                .message("failed to save data to the rfs cookies file")
                .source(e))?;

        Ok(())
    }
}

fn main_subcommands(mut command: clap::Command) -> clap::Command {
    use clap::{Command, Arg, ArgAction, value_parser};

    command
        .subcommand(
            Command::new("connect")
                .alias("c")
                .about("attempts a login to the specified server")
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
}

pub fn app_commands() -> clap::Command {
    use clap::{Command, Arg, ArgAction, value_parser};

    let command = Command::new("rfs-cli")
        .arg(
            Arg::new("cookies")
                .long("cookies")
                .action(ArgAction::Set)
                .value_parser(value_parser!(PathBuf))
                .help("specifies a specific file to save any cookie data to")
        );

    main_subcommands(command)
}

pub fn interactive_commands() -> clap::Command {
    use clap::{Command, Arg, ArgAction, value_parser};

    let command = Command::new("")
        .subcommand_required(true)
        .no_binary_name(true)
        .disable_help_flag(true);

    main_subcommands(command)
        .subcommand(
            Command::new("quit")
                .alias("q")
                .about("exits program")
        )
}

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
    let end_result = run();

    if let Err(err) = end_result {
        print_error(err);
    }
}

fn run() -> error::Result<()> {
    let app_matches = app_commands().get_matches();

    let session_file = if let Some(arg) = app_matches.get_one::<PathBuf>("cookies") {
        arg.clone()
    } else {
        let mut current_dir = std::env::current_dir()?;
        current_dir.push("rfs_cookies.json");
        current_dir
    };

    let mut state = AppState::new(session_file)?;

    match app_matches.subcommand() {
        None => {
            loop {
                let given = input::read_stdin(">")?;
                let trimmed = given.trim();

                let Ok(args_list) = shell_words::split(&trimmed) else {
                    println!("failed to parse command line args");
                    continue;
                };

                let matches = match interactive_commands().try_get_matches_from(args_list) {
                    Ok(m) => m,
                    Err(err) => {
                        println!("{}", err);
                        continue;
                    }
                };

                let result = match matches.subcommand() {
                    Some(("quit", quit_matches)) => {
                        return Ok(());
                    },
                    Some((cmd, cmd_matches)) => run_subcommand(&mut state, cmd, cmd_matches),
                    _ => {
                        println!("unknown command");

                        Ok(())
                    }
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

fn run_subcommand(state: &mut AppState, command: &str, matches: &ArgMatches) -> error::Result<()> {
    match command {
        "connect" => connect(state, matches),
        _ => {
            println!("uknown command");

            Ok(())
        }
    }
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

        state.save_store()?;

        let json = res.json::<lib::json::Wrapper<Option<lib::models::auth::AuthMethod>>>()?;

        return Ok(json.into_payload());
    }
}

fn submit_auth(
    state: &mut AppState,
    auth_method: lib::models::auth::AuthMethod
) -> error::Result<Option<lib::models::auth::VerifyMethod>> {
    match auth_method {
        lib::models::auth::AuthMethod::None => {}
        lib::models::auth::AuthMethod::Password => {
            println!("AuthMethod::Password");

            let prompt = "password: ";

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

    Ok(None)
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
