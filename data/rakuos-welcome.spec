Name:           rakuos-welcome
Version:        1.0.0
Release:        1%{?dist}
Summary:        RakuOS Welcome App
License:        GPL-3.0-or-later
URL:            https://github.com/RakuOS/rakuos-welcome
Source0:        %{url}/archive/refs/heads/main.tar.gz

BuildRequires:  rust
BuildRequires:  cargo
BuildRequires:  gcc
BuildRequires:  gtk4-devel
BuildRequires:  libadwaita-devel

Requires:       gtk4
Requires:       libadwaita

%description
RakuOS Welcome is a first-login welcome application for RakuOS Linux.
It provides a guided setup experience across GNOME, KDE Plasma,
and COSMIC desktop environments.

%prep
%autosetup -n rakuos-welcome-main

%build
cargo build --release --bin rakuos-welcome-gtk
cargo build --release --bin rakuos-welcome-qt
cargo build --release --bin rakuos-welcome-cosmic

%install
install -Dm755 target/release/rakuos-welcome-gtk \
    %{buildroot}/usr/libexec/rakuos/rakuos-welcome-gtk
install -Dm755 target/release/rakuos-welcome-qt \
    %{buildroot}/usr/libexec/rakuos/rakuos-welcome-qt
install -Dm755 target/release/rakuos-welcome-cosmic \
    %{buildroot}/usr/libexec/rakuos/rakuos-welcome-cosmic
install -Dm755 data/rakuos-welcome \
    %{buildroot}/usr/bin/rakuos-welcome

%files
/usr/bin/rakuos-welcome
/usr/libexec/rakuos/rakuos-welcome-gtk
/usr/libexec/rakuos/rakuos-welcome-qt
/usr/libexec/rakuos/rakuos-welcome-cosmic

%changelog
* %(date "+%%a %%b %%d %%Y") RakuOS Project <rakuos@rakuos.org> - 1.0.0-1
- Initial Rust rewrite with GTK4, Qt6, and COSMIC frontends
