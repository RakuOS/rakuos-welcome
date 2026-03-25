fn main() {
    // Ask qmake where Qt headers live so cpp_build can compile our C++ snippets.
    let qmake = std::env::var("QMAKE").unwrap_or_else(|_| "qmake6".to_string());
    let query = std::process::Command::new(&qmake)
        .args(["-query", "QT_INSTALL_HEADERS"])
        .output()
        .expect("Failed to run qmake — set QMAKE env var if needed");
    let qt_headers = String::from_utf8(query.stdout).unwrap().trim().to_string();

    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    cpp_build::Config::new()
        .flag("-std=c++17")
        .include(&qt_headers)
        .include(format!("{}/QtCore", qt_headers))
        .include(format!("{}/QtGui", qt_headers))
        .build(format!("{}/src/main.rs", manifest));
}
