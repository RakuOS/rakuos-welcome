// WelcomeBackend — qmetaobject QObject exposed to QML
//
// Cross-thread log streaming: a background thread writes script output to a
// temp file and updates shared AtomicXxx state.  A QML Timer calls
// pollScript() every 300 ms on the main thread, which reads the atomics
// and updates the Qt properties so QML can react.

#![allow(non_snake_case)]

use qmetaobject::prelude::*;
use std::sync::{
    atomic::{AtomicBool, AtomicI32, Ordering},
    Arc,
};
use std::path::PathBuf;

pub const TOTAL_PAGES: i32 = 5; // welcome + gaming + virt + ollama + done
const WELCOME_DONE_FILE: &str = ".config/rakuos/welcome-done";

pub fn done_file() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    PathBuf::from(home).join(WELCOME_DONE_FILE)
}

fn script_for_page(page_index: i32) -> Option<&'static str> {
    match page_index {
        1 => Some("setup-gaming"),
        2 => Some("setup-virtualization"),
        3 => Some("setup-ollama"),
        _ => None,
    }
}

fn log_file_path() -> PathBuf {
    std::env::temp_dir().join("rakuos-welcome-qt.log")
}

/// Shared state updated by the background thread, read by pollScript().
struct SharedState {
    running:      AtomicBool,
    result:       AtomicI32,   // 0 = idle, 1 = success, 2 = failed
    log_revision: AtomicI32,
}

impl Default for SharedState {
    fn default() -> Self {
        Self {
            running:      AtomicBool::new(false),
            result:       AtomicI32::new(0),
            log_revision: AtomicI32::new(0),
        }
    }
}

#[derive(QObject)]
pub struct WelcomeBackend {
    base: qt_base_class!(trait QObject),

    // ── Qt properties (camelCase so QML handlers work naturally) ─────────
    pub currentPage:   qt_property!(i32;  NOTIFY currentPageChanged),
    pub scriptRunning: qt_property!(bool; NOTIFY scriptRunningChanged),
    pub scriptResult:  qt_property!(i32;  NOTIFY scriptResultChanged),
    /// Incremented when log file has new content; QML watches via onLogRevisionChanged.
    pub logRevision:   qt_property!(i32;  NOTIFY logRevisionChanged),

    // ── Signals ──────────────────────────────────────────────────────────
    pub currentPageChanged:   qt_signal!(),
    pub scriptRunningChanged: qt_signal!(),
    pub scriptResultChanged:  qt_signal!(),
    pub logRevisionChanged:   qt_signal!(),
    /// Emitted by finishSetup(); QML connects: onCloseRequested: Qt.quit()
    pub closeRequested:       qt_signal!(),

    // ── Invokable methods ─────────────────────────────────────────────────
    pub nextPage: qt_method!(fn nextPage(&mut self) {
        let idx = self.currentPage;
        if idx < TOTAL_PAGES - 1 {
            // Reset shared atomics so pollScript() doesn't restore the old state.
            self.shared.result.store(0, Ordering::Relaxed);
            self.shared.running.store(false, Ordering::Relaxed);
            self.scriptResult = 0;
            self.scriptResultChanged();
            self.scriptRunning = false;
            self.scriptRunningChanged();
            self.currentPage = idx + 1;
            self.currentPageChanged();
        }
    }),

    pub backPage: qt_method!(fn backPage(&mut self) {
        let idx = self.currentPage;
        if idx > 0 {
            // Reset shared atomics so pollScript() doesn't restore the old state.
            self.shared.result.store(0, Ordering::Relaxed);
            self.shared.running.store(false, Ordering::Relaxed);
            self.scriptResult = 0;
            self.scriptResultChanged();
            self.scriptRunning = false;
            self.scriptRunningChanged();
            self.currentPage = idx - 1;
            self.currentPageChanged();
        }
    }),

    pub totalPages: qt_method!(fn totalPages(&self) -> i32 {
        TOTAL_PAGES
    }),

    /// Called by QML Timer every ~300 ms to sync background thread state.
    pub pollScript: qt_method!(fn pollScript(&mut self) {
        let shared = &self.shared;
        let new_running = shared.running.load(Ordering::Relaxed);
        let new_result  = shared.result.load(Ordering::Relaxed);
        let new_rev     = shared.log_revision.load(Ordering::Relaxed);

        if self.scriptRunning != new_running {
            self.scriptRunning = new_running;
            self.scriptRunningChanged();
        }
        if self.scriptResult != new_result {
            self.scriptResult = new_result;
            self.scriptResultChanged();
        }
        if self.logRevision != new_rev {
            self.logRevision = new_rev;
            self.logRevisionChanged();
        }
    }),

    pub runScriptForPage: qt_method!(fn runScriptForPage(&mut self, pageIndex: i32) {
        if self.scriptRunning {
            return;
        }
        let script_name = match script_for_page(pageIndex) {
            Some(s) => s.to_string(),
            None    => return,
        };

        let log_path = log_file_path();
        let _ = std::fs::write(&log_path, "Starting setup, please wait…\n");

        self.shared.running.store(true, Ordering::Relaxed);
        self.shared.result.store(0, Ordering::Relaxed);
        self.shared.log_revision.store(0, Ordering::Relaxed);

        self.scriptRunning = true;
        self.scriptRunningChanged();
        self.scriptResult = 0;
        self.scriptResultChanged();
        self.logRevision = 0;
        self.logRevisionChanged();

        let shared = self.shared.clone();

        std::thread::spawn(move || {
            use std::io::{BufRead, BufReader, Write};
            use std::process::{Command, Stdio};

            let append_log = |text: &str| {
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .append(true)
                    .open(&log_path)
                {
                    let _ = f.write_all(text.as_bytes());
                    let _ = f.write_all(b"\n");
                }
            };

            match Command::new("pkexec")
                .arg(format!("/usr/libexec/rakuos/{}", script_name))
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            {
                Ok(mut child) => {
                    let mut rev = 0i32;

                    if let Some(stdout) = child.stdout.take() {
                        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                            if !line.is_empty() {
                                append_log(&line);
                                rev += 1;
                                shared.log_revision.store(rev, Ordering::Relaxed);
                            }
                        }
                    }
                    if let Some(stderr) = child.stderr.take() {
                        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                            if !line.is_empty() {
                                append_log(&line);
                                rev += 1;
                                shared.log_revision.store(rev, Ordering::Relaxed);
                            }
                        }
                    }

                    let success = child.wait().map(|s| s.success()).unwrap_or(false);
                    append_log(if success {
                        "\nSetup completed successfully!"
                    } else {
                        "\nSetup failed. You can retry or skip."
                    });

                    shared.result.store(if success { 1 } else { 2 }, Ordering::Relaxed);
                    shared.log_revision.fetch_add(1, Ordering::Relaxed);
                    shared.running.store(false, Ordering::Relaxed);
                }
                Err(e) => {
                    append_log(&format!("{}\nSetup failed. You can retry or skip.", e));
                    shared.result.store(2, Ordering::Relaxed);
                    shared.log_revision.fetch_add(1, Ordering::Relaxed);
                    shared.running.store(false, Ordering::Relaxed);
                }
            }
        });
    }),

    pub finishSetup: qt_method!(fn finishSetup(&mut self) {
        let df = done_file();
        let _ = std::fs::create_dir_all(df.parent().unwrap());
        let _ = std::fs::File::create(&df);
        self.closeRequested();
    }),

    // ── Internal (non-Qt) field ───────────────────────────────────────────
    shared: Arc<SharedState>,
}

impl Default for WelcomeBackend {
    fn default() -> Self {
        WelcomeBackend {
            base:                   Default::default(),
            currentPage:            Default::default(),
            scriptRunning:          Default::default(),
            scriptResult:           Default::default(),
            logRevision:            Default::default(),
            currentPageChanged:     Default::default(),
            scriptRunningChanged:   Default::default(),
            scriptResultChanged:    Default::default(),
            logRevisionChanged:     Default::default(),
            closeRequested:         Default::default(),
            nextPage:               Default::default(),
            backPage:               Default::default(),
            totalPages:             Default::default(),
            pollScript:             Default::default(),
            runScriptForPage:       Default::default(),
            finishSetup:            Default::default(),
            shared:                 Arc::new(SharedState::default()),
        }
    }
}
