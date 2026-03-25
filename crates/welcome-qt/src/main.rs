// rakuos-welcome-qt — Qt6/QML welcome app for KDE Plasma
// Runs once on first login; pass --force / -f to override.

mod backend;
use backend::WelcomeBackend;
use qmetaobject::QmlEngine;

fn main() {
    let force = std::env::args_os().any(|a| a == "--force" || a == "-f");
    if backend::done_file().exists() && !force {
        return;
    }

    // Qt6 blocks XMLHttpRequest from reading local files by default.
    // This flag re-enables it so the QML log area can poll the temp file.
    std::env::set_var("QML_XHR_ALLOW_FILE_READ", "1");

    // Register the backend type before creating the engine.
    qmetaobject::qml_register_type::<WelcomeBackend>(
        c"org.rakuos.welcome",
        1, 0,
        c"WelcomeBackend",
    );

    let mut engine = QmlEngine::new();

    // Installed path is /usr/share/rakuos-welcome-qt/main.qml.
    // During development set RAKUOS_QML_DIR to the qml/ source folder.
    let qml_dir = std::env::var("RAKUOS_QML_DIR")
        .unwrap_or_else(|_| "/usr/share/rakuos-welcome-qt".to_string());
    let qml_path = format!("file://{}/main.qml", qml_dir);

    engine.load_file(qml_path.into());
    engine.exec();
}
