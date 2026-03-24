// rakuos-welcome-gtk — GTK4/libadwaita welcome app for GNOME
// Runs once on first login; use --force to override.

use std::path::PathBuf;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, IconTheme, Label, Orientation,
    PolicyType, ScrolledWindow, Separator, Stack, StackTransitionType,
    TextView, WrapMode,
};

enum WorkerMsg {
    Line(String),
    Done(bool),
}
use libadwaita::prelude::*;
use libadwaita::{Application, ApplicationWindow, HeaderBar, StyleManager};

const APP_ID: &str = "org.rakuos.Welcome";
const WELCOME_DONE_FILE: &str = ".config/rakuos/welcome-done";
const LOGO_DARK: &str = "/usr/share/pixmaps/fedora_whitelogo_med.png";
const LOGO_LIGHT: &str = "/usr/share/pixmaps/fedora_logo_med.png";

struct SetupPage {
    name: &'static str,
    title: &'static str,
    icon: &'static str,
    description: &'static str,
    script: &'static str,
}

const SETUP_PAGES: &[SetupPage] = &[
    SetupPage {
        name: "gaming",
        title: "Gaming Setup",
        icon: "🎮",
        description: "Install Steam and Lutris natively for the best gaming performance. \
                      Native packages give better Proton and Wine compatibility than Flatpak.",
        script: "setup-gaming",
    },
    SetupPage {
        name: "virt",
        title: "Virtualization",
        icon: "🖥",
        description: "Set up KVM/QEMU and virt-manager to run virtual machines. \
                      Ideal for running Windows or other Linux distros alongside RakuOS.",
        script: "setup-virtualization",
    },
    SetupPage {
        name: "ollama",
        title: "Local AI with Ollama",
        icon: "🤖",
        description: "Install Ollama to run AI language models locally on your GPU. \
                      Keep your conversations private and use AI without the cloud.",
        script: "setup-ollama",
    },
];

fn done_file() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    PathBuf::from(home).join(WELCOME_DONE_FILE)
}

fn get_logo_path(style: &StyleManager) -> &'static str {
    if style.is_dark() { LOGO_DARK } else { LOGO_LIGHT }
}

fn build_ui(app: &Application) {
    let force = std::env::args_os().any(|a| a == "--force" || a == "-f");
    if done_file().exists() && !force {
        return;
    }

    let win = ApplicationWindow::builder()
        .application(app)
        .title("Welcome to RakuOS")
        .default_width(720)
        .default_height(560)
        .build();

    // Window icon
    if let Some(display) = gtk4::gdk::Display::default() {
        IconTheme::for_display(&display).add_search_path("/usr/share/pixmaps");
    }
    win.set_icon_name(Some("rakuos-logo"));

    // Root layout
    let root = GtkBox::new(Orientation::Vertical, 0);
    win.set_content(Some(&root));

    // Header bar
    let header = HeaderBar::new();
    root.append(&header);

    // Page stack
    let stack = Stack::new();
    stack.set_transition_type(StackTransitionType::SlideLeftRight);
    stack.set_transition_duration(200);
    stack.set_vexpand(true);
    root.append(&stack);

    // Log area (hidden until a setup runs)
    let log_view = TextView::new();
    log_view.set_editable(false);
    log_view.set_cursor_visible(false);
    log_view.set_monospace(true);
    log_view.set_wrap_mode(WrapMode::WordChar);
    let log_buffer = log_view.buffer();

    let log_scroll = ScrolledWindow::builder()
        .child(&log_view)
        .min_content_height(150)
        .max_content_height(150)
        .hscrollbar_policy(PolicyType::Never)
        .build();
    log_scroll.set_visible(false);
    root.append(&log_scroll);

    // Nav bar
    let sep = Separator::new(Orientation::Horizontal);
    root.append(&sep);

    let nav = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .margin_start(24)
        .margin_end(24)
        .margin_top(12)
        .margin_bottom(12)
        .build();

    let dots_box = GtkBox::new(Orientation::Horizontal, 8);
    nav.append(&dots_box);

    let spacer = GtkBox::new(Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    nav.append(&spacer);

    let back_btn = Button::with_label("← Back");
    back_btn.set_visible(false);
    nav.append(&back_btn);

    let next_btn = Button::with_label("Next →");
    next_btn.add_css_class("suggested-action");
    nav.append(&next_btn);

    root.append(&nav);

    // Build all pages
    let total_pages = SETUP_PAGES.len() + 2; // welcome + setups + done
    let mut page_names: Vec<String> = Vec::new();

    // Welcome page
    build_welcome_page(&stack);
    page_names.push("welcome".to_string());

    // Setup pages
    for sp in SETUP_PAGES {
        build_setup_page(&stack, sp, &log_scroll, &log_buffer);
        page_names.push(sp.name.to_string());
    }

    // Done page
    build_done_page(&stack);
    page_names.push("done".to_string());

    // Dots
    let dot_labels: Vec<Label> = (0..total_pages)
        .map(|_| {
            let dot = Label::new(Some("●"));
            dots_box.append(&dot);
            dot
        })
        .collect();

    let current = Arc::new(Mutex::new(0usize));

    let update_nav = {
        let dot_labels = dot_labels.clone();
        let back_btn = back_btn.clone();
        let next_btn = next_btn.clone();
        let stack = stack.clone();
        let page_names = page_names.clone();
        let total = total_pages;
        move |idx: usize| {
            for (i, dot) in dot_labels.iter().enumerate() {
                if i == idx {
                    dot.add_css_class("accent");
                    dot.remove_css_class("dim-label");
                } else {
                    dot.add_css_class("dim-label");
                    dot.remove_css_class("accent");
                }
            }
            back_btn.set_visible(idx > 0);
            next_btn.set_label(if idx == total - 1 { "Finish" } else { "Next →" });
            stack.set_visible_child_name(&page_names[idx]);
        }
    };

    update_nav(0);

    // Next button
    next_btn.connect_clicked({
        let current = Arc::clone(&current);
        let log_scroll = log_scroll.clone();
        let win = win.clone();
        let update_nav = update_nav.clone();
        move |_| {
            let mut idx = current.lock().unwrap();
            if *idx == total_pages - 1 {
                let df = done_file();
                let _ = std::fs::create_dir_all(df.parent().unwrap());
                let _ = std::fs::File::create(&df);
                win.close();
            } else {
                *idx += 1;
                log_scroll.set_visible(false);
                update_nav(*idx);
            }
        }
    });

    // Back button
    back_btn.connect_clicked({
        let current = Arc::clone(&current);
        let log_scroll = log_scroll.clone();
        let update_nav = update_nav.clone();
        move |_| {
            let mut idx = current.lock().unwrap();
            if *idx > 0 {
                *idx -= 1;
                log_scroll.set_visible(false);
                update_nav(*idx);
            }
        }
    });

    win.present();
}

fn build_welcome_page(stack: &Stack) {
    let style = StyleManager::default();
    let logo_path = get_logo_path(&style);

    let page = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(16)
        .valign(Align::Center)
        .halign(Align::Center)
        .margin_start(60)
        .margin_end(60)
        .margin_top(40)
        .margin_bottom(40)
        .build();

    // Logo
    let img = gtk4::Image::from_file(logo_path);
    img.set_pixel_size(300);
    img.set_halign(Align::Center);
    page.append(&img);

    let heading = Label::new(Some("Welcome to RakuOS Linux"));
    heading.add_css_class("title-1");
    heading.set_halign(Align::Center);
    page.append(&heading);

    let tagline = Label::new(Some("The Hybrid Atomic Linux Desktop"));
    tagline.add_css_class("title-4");
    tagline.set_halign(Align::Center);
    page.append(&tagline);

    let desc = Label::new(Some(
        "RakuOS combines the stability and security of an atomic immutable base \
         with the flexibility of a traditional Linux distribution. \
         Your system can never be broken by a bad update.",
    ));
    desc.set_wrap(true);
    desc.set_max_width_chars(70);
    desc.set_halign(Align::Center);
    desc.set_justify(gtk4::Justification::Center);
    page.append(&desc);

    page.append(&Separator::new(Orientation::Horizontal));

    let links_label = Label::new(Some("Find us online:"));
    links_label.set_halign(Align::Center);
    page.append(&links_label);

    let links_box = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(24)
        .halign(Align::Center)
        .build();

    for (text, url) in [
        ("🌐 Website", "https://rakuos.org"),
        ("💻 GitHub", "https://github.com/RakuOS"),
        ("📦 SourceForge", "https://sourceforge.net/projects/rakuos/"),
    ] {
        let btn = gtk4::LinkButton::with_label(url, text);
        links_box.append(&btn);
    }
    page.append(&links_box);

    stack.add_named(&page, Some("welcome"));
}

fn build_setup_page(
    stack: &Stack,
    sp: &SetupPage,
    log_scroll: &ScrolledWindow,
    log_buffer: &gtk4::TextBuffer,
) {
    let page = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(16)
        .valign(Align::Center)
        .halign(Align::Center)
        .margin_start(60)
        .margin_end(60)
        .margin_top(40)
        .margin_bottom(40)
        .build();

    let icon_label = Label::new(None);
    icon_label.set_markup(&format!("<span size=\"3000%\">{}</span>", sp.icon));
    icon_label.set_halign(Align::Center);
    page.append(&icon_label);

    let title_label = Label::new(Some(sp.title));
    title_label.add_css_class("title-2");
    title_label.set_halign(Align::Center);
    page.append(&title_label);

    let desc_label = Label::new(Some(sp.description));
    desc_label.set_wrap(true);
    desc_label.set_max_width_chars(70);
    desc_label.set_halign(Align::Center);
    desc_label.set_justify(gtk4::Justification::Center);
    page.append(&desc_label);

    let install_btn = Button::with_label(&format!("Set Up {}", sp.title));
    install_btn.add_css_class("suggested-action");
    install_btn.add_css_class("pill");
    install_btn.set_halign(Align::Center);
    install_btn.set_size_request(200, 42);
    page.append(&install_btn);

    let script = sp.script.to_string();
    let log_scroll = log_scroll.clone();
    let log_buffer = log_buffer.clone();

    install_btn.connect_clicked(move |btn| {
        btn.set_sensitive(false);
        btn.set_label("Installing...");
        log_scroll.set_visible(true);
        log_buffer.insert(&mut log_buffer.end_iter(), "Starting setup, please wait...\n");

        let (sender, receiver) = async_channel::unbounded::<WorkerMsg>();
        let script = script.clone();

        thread::spawn(move || {
            match Command::new("pkexec")
                .arg(format!("/usr/libexec/rakuos/{}", script))
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            {
                Ok(mut child) => {
                    // Stream stdout line by line as it arrives
                    if let Some(stdout) = child.stdout.take() {
                        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                            let _ = sender.send_blocking(WorkerMsg::Line(line));
                        }
                    }
                    // Capture any stderr after stdout closes
                    if let Some(stderr) = child.stderr.take() {
                        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                            let _ = sender.send_blocking(WorkerMsg::Line(line));
                        }
                    }
                    let success = child.wait().map(|s| s.success()).unwrap_or(false);
                    let _ = sender.send_blocking(WorkerMsg::Done(success));
                }
                Err(e) => {
                    let _ = sender.send_blocking(WorkerMsg::Line(e.to_string()));
                    let _ = sender.send_blocking(WorkerMsg::Done(false));
                }
            }
        });

        let log_buffer = log_buffer.clone();
        let log_scroll = log_scroll.clone();
        let btn = btn.clone();

        glib::MainContext::default().spawn_local(async move {
            while let Ok(msg) = receiver.recv().await {
                match msg {
                    WorkerMsg::Line(line) => {
                        if !line.is_empty() {
                            log_buffer.insert(&mut log_buffer.end_iter(), &format!("{line}\n"));
                            let adj = log_scroll.vadjustment();
                            adj.set_value(adj.upper());
                        }
                    }
                    WorkerMsg::Done(success) => {
                        if success {
                            btn.set_label("✓ Done");
                            log_buffer.insert(&mut log_buffer.end_iter(), "\nSetup completed successfully!\n");
                        } else {
                            btn.set_sensitive(true);
                            btn.set_label("Retry");
                            log_buffer.insert(&mut log_buffer.end_iter(), "\nSetup failed. You can retry or skip.\n");
                        }
                        let adj = log_scroll.vadjustment();
                        adj.set_value(adj.upper());
                        break;
                    }
                }
            }
        });
    });

    stack.add_named(&page, Some(sp.name));
}

fn build_done_page(stack: &Stack) {
    let page = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(16)
        .valign(Align::Center)
        .halign(Align::Center)
        .margin_start(60)
        .margin_end(60)
        .margin_top(40)
        .margin_bottom(40)
        .build();

    let icon = Label::new(None);
    icon.set_markup("<span size=\"3000%\">🎉</span>");
    icon.set_halign(Align::Center);
    page.append(&icon);

    let heading = Label::new(Some("You're All Set!"));
    heading.add_css_class("title-1");
    heading.set_halign(Align::Center);
    page.append(&heading);

    let desc = Label::new(Some(
        "RakuOS is ready to use. You can always run additional setup \
         at any time from the terminal using the rakuos command.",
    ));
    desc.set_wrap(true);
    desc.set_max_width_chars(70);
    desc.set_halign(Align::Center);
    desc.set_justify(gtk4::Justification::Center);
    page.append(&desc);

    let tip = Label::new(Some(
        "💡 Tip: Run  rakuos  in a terminal to see all available commands.",
    ));
    tip.set_halign(Align::Center);
    tip.add_css_class("monospace");
    page.append(&tip);

    stack.add_named(&page, Some("done"));
}

fn main() {
    // Strip --force/-f before GLib sees argv — GLib rejects unknown flags
    let args: Vec<String> = std::env::args()
        .filter(|a| a != "--force" && a != "-f")
        .collect();

    let app = Application::builder()
        .application_id(APP_ID)
        .build();

    app.connect_activate(build_ui);
    app.run_with_args(&args);
}
