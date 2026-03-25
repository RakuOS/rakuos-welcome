fn main() {
    let qmake = std::env::var("QMAKE").unwrap_or_else(|_| "qmake6".to_string());

    let qt_headers = String::from_utf8(
        std::process::Command::new(&qmake)
            .args(["-query", "QT_INSTALL_HEADERS"])
            .output()
            .expect("Failed to run qmake")
            .stdout,
    )
    .unwrap()
    .trim()
    .to_string();

    let qt_libs = String::from_utf8(
        std::process::Command::new(&qmake)
            .args(["-query", "QT_INSTALL_LIBS"])
            .output()
            .expect("Failed to run qmake")
            .stdout,
    )
    .unwrap()
    .trim()
    .to_string();

    cc::Build::new()
        .cpp(true)
        .flag("-std=c++17")
        .include(&qt_headers)
        .include(format!("{}/QtCore", qt_headers))
        .include(format!("{}/QtGui", qt_headers))
        .file("src/qt_helpers.cpp")
        .compile("qt_helpers");

    println!("cargo:rustc-link-search={}", qt_libs);
    println!("cargo:rustc-link-lib=Qt6Gui");
    println!("cargo:rustc-link-lib=Qt6Core");
}
