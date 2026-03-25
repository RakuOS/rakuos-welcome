// rakuos-welcome-cosmic — libcosmic welcome app for the COSMIC DE
// Runs once on first login; pass --force / -f to override.

use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, AtomicI32, Ordering},
    Arc,
};

use cosmic::app::{Core, Settings, Task};
use cosmic::iced::{self, Length, Subscription};
use cosmic::iced_core::layout::Limits;
use cosmic::{executor, widget, Application, ApplicationExt, Element};

// ── Constants ─────────────────────────────────────────────────────────────────

const APP_ID: &str = "org.rakuos.Welcome";
const WELCOME_DONE_FILE: &str = ".config/rakuos/welcome-done";
const TOTAL_PAGES: usize = 5;

const LOGO_DARK: &str = "/usr/share/pixmaps/fedora_whitelogo_med.png";
const LOGO_LIGHT: &str = "/usr/share/pixmaps/fedora_logo_med.png";

struct SetupPage {
    title: &'static str,
    icon: &'static str,
    description: &'static str,
    script: &'static str,
}

const SETUP_PAGES: &[SetupPage] = &[
    SetupPage {
        title: "Gaming Setup",
        icon: "🎮",
        description: "Install Steam and Lutris natively for the best gaming performance. \
                      Native packages give better Proton and Wine compatibility than Flatpak.",
        script: "setup-gaming",
    },
    SetupPage {
        title: "Virtualization",
        icon: "🖥",
        description: "Set up KVM/QEMU and virt-manager to run virtual machines. \
                      Ideal for running Windows or other Linux distros alongside RakuOS.",
        script: "setup-virtualization",
    },
    SetupPage {
        title: "Local AI with Ollama",
        icon: "🤖",
        description: "Install Ollama to run AI language models locally on your GPU. \
                      Keep your conversations private and use AI without the cloud.",
        script: "setup-ollama",
    },
];

// ── Helpers ───────────────────────────────────────────────────────────────────

fn done_file() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    PathBuf::from(home).join(WELCOME_DONE_FILE)
}

fn log_file_path() -> PathBuf {
    std::env::temp_dir().join("rakuos-welcome-cosmic.log")
}

// ── Shared state (updated by background thread) ───────────────────────────────

#[derive(Default)]
struct SharedState {
    running: AtomicBool,
    /// 0 = idle, 1 = success, 2 = failed
    result: AtomicI32,
}

// ── App ───────────────────────────────────────────────────────────────────────

pub struct WelcomeApp {
    core: Core,
    current_page: usize,
    log_text: String,
    last_log_len: usize,
    shared: Arc<SharedState>,
}

#[derive(Debug, Clone)]
pub enum Message {
    NextPage,
    BackPage,
    RunScript(usize),
    PollOutput,
    OpenUrl(String),
    Finish,
}

impl Application for WelcomeApp {
    type Executor = executor::Default;
    type Flags = ();
    type Message = Message;

    const APP_ID: &'static str = APP_ID;

    fn core(&self) -> &Core {
        &self.core
    }
    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, (): ()) -> (Self, Task<Message>) {
        let mut app = Self {
            core,
            current_page: 0,
            log_text: String::new(),
            last_log_len: 0,
            shared: Arc::new(SharedState::default()),
        };
        app.set_header_title("Welcome to RakuOS".into());
        (app, Task::none())
    }

    fn subscription(&self) -> Subscription<Message> {
        let running = self.shared.running.load(Ordering::Relaxed);
        let result  = self.shared.result.load(Ordering::Relaxed);
        let on_setup_page = self.current_page > 0 && self.current_page < TOTAL_PAGES - 1;
        if running || (result > 0 && on_setup_page) {
            iced::time::every(std::time::Duration::from_millis(200))
                .map(|_| Message::PollOutput)
        } else {
            Subscription::none()
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::NextPage => {
                if self.current_page < TOTAL_PAGES - 1 {
                    self.current_page += 1;
                    self.shared.result.store(0, Ordering::Relaxed);
                    self.log_text.clear();
                    self.last_log_len = 0;
                }
            }

            Message::BackPage => {
                if self.current_page > 0 {
                    self.current_page -= 1;
                    self.shared.result.store(0, Ordering::Relaxed);
                    self.log_text.clear();
                    self.last_log_len = 0;
                }
            }

            Message::RunScript(page) => {
                if self.shared.running.load(Ordering::Relaxed) {
                    return Task::none();
                }
                let Some(page_def) = SETUP_PAGES.get(page.saturating_sub(1)) else {
                    return Task::none();
                };
                let script = page_def.script.to_string();

                let log_path = log_file_path();
                let init_msg = "Starting setup, please wait…\n";
                let _ = std::fs::write(&log_path, init_msg);
                self.log_text = init_msg.to_string();
                self.last_log_len = self.log_text.len();

                self.shared.running.store(true,  Ordering::Relaxed);
                self.shared.result.store(0,      Ordering::Relaxed);

                let shared = self.shared.clone();
                std::thread::spawn(move || {
                    use std::io::{BufRead, BufReader, Write};
                    use std::process::{Command, Stdio};

                    let append = |text: &str| {
                        if let Ok(mut f) = std::fs::OpenOptions::new()
                            .append(true).open(&log_path)
                        {
                            let _ = f.write_all(text.as_bytes());
                            let _ = f.write_all(b"\n");
                        }
                    };

                    match Command::new("pkexec")
                        .arg(format!("/usr/libexec/rakuos/{}", script))
                        .stdout(Stdio::piped())
                        .stderr(Stdio::piped())
                        .spawn()
                    {
                        Ok(mut child) => {
                            if let Some(out) = child.stdout.take() {
                                for line in BufReader::new(out).lines().map_while(Result::ok) {
                                    if !line.is_empty() { append(&line); }
                                }
                            }
                            if let Some(err) = child.stderr.take() {
                                for line in BufReader::new(err).lines().map_while(Result::ok) {
                                    if !line.is_empty() { append(&line); }
                                }
                            }
                            let ok = child.wait().map(|s| s.success()).unwrap_or(false);
                            append(if ok {
                                "\nSetup completed successfully!"
                            } else {
                                "\nSetup failed. You can retry or skip."
                            });
                            shared.result.store(if ok { 1 } else { 2 }, Ordering::Relaxed);
                            shared.running.store(false, Ordering::Relaxed);
                        }
                        Err(e) => {
                            append(&format!("{}\nSetup failed.", e));
                            shared.result.store(2,      Ordering::Relaxed);
                            shared.running.store(false, Ordering::Relaxed);
                        }
                    }
                });
            }

            Message::PollOutput => {
                if let Ok(content) = std::fs::read_to_string(log_file_path()) {
                    if content.len() != self.last_log_len {
                        self.log_text = content;
                        self.last_log_len = self.log_text.len();
                    }
                }
            }

            Message::OpenUrl(url) => {
                let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
            }

            Message::Finish => {
                let df = done_file();
                let _ = std::fs::create_dir_all(df.parent().unwrap());
                let _ = std::fs::File::create(&df);
                std::process::exit(0);
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<Message> {
        let running = self.shared.running.load(Ordering::Relaxed);
        let result  = self.shared.result.load(Ordering::Relaxed);
        let on_setup_page = self.current_page > 0 && self.current_page < TOTAL_PAGES - 1;

        // ── Page content ──────────────────────────────────────────────────
        let page: Element<Message> = match self.current_page {
            0         => self.view_welcome(),
            p @ 1..=3 => self.view_setup(p),
            _         => self.view_done(),
        };

        // ── Log area ──────────────────────────────────────────────────────
        let log_area: Option<Element<Message>> = if on_setup_page && (running || result > 0) {
            Some(
                widget::container(
                    widget::scrollable(
                        widget::text(&self.log_text)
                            .font(iced::Font::MONOSPACE)
                            .size(12),
                    )
                    .height(Length::Fixed(150.0)),
                )
                .width(Length::Fill)
                .padding(8)
                .into(),
            )
        } else {
            None
        };

        // ── Navigation dots ───────────────────────────────────────────────
        let dots = (0..TOTAL_PAGES).fold(
            widget::row::with_capacity(TOTAL_PAGES),
            |row, i| {
                row.push(
                    widget::text(if i == self.current_page { "●" } else { "○" }).size(10),
                )
                .spacing(4)
            },
        );

        // ── Back / Next buttons ───────────────────────────────────────────
        let back_btn: Option<Element<Message>> = if self.current_page > 0 {
            Some(
                widget::button::standard("← Back")
                    .on_press_maybe((!running).then_some(Message::BackPage))
                    .into(),
            )
        } else {
            None
        };

        let next_label = if self.current_page == TOTAL_PAGES - 1 { "Finish" } else { "Next →" };
        let next_msg   = if self.current_page == TOTAL_PAGES - 1 { Message::Finish } else { Message::NextPage };
        let next_btn: Element<Message> = widget::button::suggested(next_label)
            .on_press_maybe((!running).then_some(next_msg))
            .into();

        let mut nav_row = widget::row::with_capacity(4)
            .push(widget::container(dots).width(Length::Fill))
            .spacing(8)
            .padding(iced::Padding::from([12, 24]));
        if let Some(b) = back_btn {
            nav_row = nav_row.push(b);
        }
        nav_row = nav_row.push(next_btn);

        // ── Assemble ──────────────────────────────────────────────────────
        let mut col = widget::column::with_capacity(5)
            .push(widget::container(page).width(Length::Fill).height(Length::Fill));

        if let Some(log) = log_area {
            col = col.push(widget::divider::horizontal::default());
            col = col.push(log);
        }

        col.push(widget::divider::horizontal::default())
           .push(nav_row)
           .into()
    }
}

// ── Page views ────────────────────────────────────────────────────────────────

impl WelcomeApp {
    fn logo_path(&self) -> &'static str {
        if cosmic::theme::is_dark() {
            LOGO_DARK
        } else {
            LOGO_LIGHT
        }
    }

    fn view_welcome(&self) -> Element<Message> {
        let logo = widget::image(widget::image::Handle::from_path(self.logo_path()))
            .width(Length::Fixed(240.0))
            .height(Length::Fixed(80.0));

        let content = widget::column()
            .push(widget::container(logo).width(Length::Fill).center_x(Length::Fill))
            .push(widget::container(widget::text::title1("Welcome to RakuOS Linux")).width(Length::Fill).center_x(Length::Fill))
            .push(widget::container(widget::text("The Hybrid Atomic Linux Desktop").size(14)).width(Length::Fill).center_x(Length::Fill))
            .push(widget::Space::new().height(Length::Fixed(8.0)))
            .push(
                widget::container(
                    widget::text(
                        "RakuOS combines the stability and security of an atomic immutable base \
                         with the flexibility of a traditional Linux distribution. \
                         Your system can never be broken by a bad update.",
                    )
                    .width(Length::Fixed(520.0)),
                )
                .width(Length::Fill)
                .center_x(Length::Fill),
            )
            .push(
                widget::container(widget::divider::horizontal::default())
                    .width(Length::Fixed(480.0))
                    .center_x(Length::Fill),
            )
            .push(widget::container(widget::text("Find us online:")).width(Length::Fill).center_x(Length::Fill))
            .push(
                widget::container(
                    widget::row()
                        .push(widget::button::link("🌐 Website").on_press(Message::OpenUrl("https://rakuos.org".into())))
                        .push(widget::button::link("💻 GitHub").on_press(Message::OpenUrl("https://github.com/RakuOS".into())))
                        .push(widget::button::link("📦 SourceForge").on_press(Message::OpenUrl("https://sourceforge.net/projects/rakuos/".into())))
                        .spacing(8),
                )
                .width(Length::Fill)
                .center_x(Length::Fill),
            )
            .spacing(12)
            .padding(40);

        widget::container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }

    fn view_setup(&self, page: usize) -> Element<Message> {
        let def     = &SETUP_PAGES[page - 1];
        let running = self.shared.running.load(Ordering::Relaxed);
        let result  = self.shared.result.load(Ordering::Relaxed);

        let btn_label = if result == 1 {
            "✓ Done".to_string()
        } else if result == 2 {
            "Retry".to_string()
        } else if running {
            "Installing…".to_string()
        } else {
            format!("Set Up {}", def.title)
        };

        let setup_btn: Element<Message> = widget::button::suggested(btn_label)
            .on_press_maybe((!running && result != 1).then_some(Message::RunScript(page)))
            .into();

        widget::container(
            widget::column()
                .push(widget::container(widget::text(def.icon).size(48)).center_x(Length::Fill).width(Length::Fill))
                .push(widget::container(widget::text::title2(def.title)).center_x(Length::Fill).width(Length::Fill))
                .push(
                    widget::container(widget::text(def.description).width(Length::Fixed(480.0)))
                        .center_x(Length::Fill)
                        .width(Length::Fill),
                )
                .push(widget::container(setup_btn).width(Length::Fill).center_x(Length::Fill))
                .spacing(16)
                .padding(40),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_y(Length::Fill)
        .into()
    }

    fn view_done(&self) -> Element<Message> {
        widget::container(
            widget::column()
                .push(widget::container(widget::text("🎉").size(56)).width(Length::Fill).center_x(Length::Fill))
                .push(widget::container(widget::text::title1("You're All Set!")).width(Length::Fill).center_x(Length::Fill))
                .push(
                    widget::container(
                        widget::text(
                            "RakuOS is ready to use. You can always run additional setup\n\
                             at any time from the terminal using the rakuos command.",
                        )
                        .width(Length::Fixed(480.0)),
                    )
                    .width(Length::Fill)
                    .center_x(Length::Fill),
                )
                .push(
                    widget::container(
                        widget::text("💡 Tip: Run  rakuos  in a terminal to see all available commands.")
                            .font(iced::Font::MONOSPACE)
                            .size(13),
                    )
                    .width(Length::Fill)
                    .center_x(Length::Fill),
                )
                .spacing(16)
                .padding(40),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_y(Length::Fill)
        .into()
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let force = std::env::args_os().any(|a| a == "--force" || a == "-f");
    if done_file().exists() && !force {
        return Ok(());
    }

    let settings = Settings::default()
        .size(iced::Size::new(720.0, 580.0))
        .size_limits(Limits::NONE.min_width(600.0).min_height(500.0))
        .is_daemon(false);

    cosmic::app::run::<WelcomeApp>(settings, ())?;
    Ok(())
}
