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
BuildRequires:  qt6-qtbase-devel
BuildRequires:  qt6-qtwidgets-devel

%description
RakuOS Welcome is a first-login welcome application for RakuOS Linux.
It provides a guided setup experience across GNOME, KDE Plasma,
and COSMIC desktop environments.

%package gtk
Summary:        RakuOS Welcome App — GTK4/GNOME frontend
Requires:       rakuos-welcome = %{version}-%{release}
Requires:       gtk4
Requires:       libadwaita

%description gtk
GTK4/libadwaita frontend for the RakuOS Welcome app.
Install on GNOME-based RakuOS images.

%package qt
Summary:        RakuOS Welcome App — Qt6/KDE frontend
Requires:       rakuos-welcome = %{version}-%{release}
Requires:       qt6-qtbase

%description qt
Qt6 frontend for the RakuOS Welcome app.
Install on KDE Plasma-based RakuOS images.

%package cosmic
Summary:        RakuOS Welcome App — COSMIC frontend
Requires:       rakuos-welcome = %{version}-%{release}

%description cosmic
libcosmic/iced frontend for the RakuOS Welcome app.
Install on COSMIC-based RakuOS images.

%prep
%autosetup -n rakuos-welcome-main

%build
cargo build --release --bin rakuos-welcome-gtk
cargo build --release --bin rakuos-welcome-qt
cargo build --release --bin rakuos-welcome-cosmic

%install
install -Dm755 data/rakuos-welcome \
    %{buildroot}/usr/bin/rakuos-welcome
install -Dm644 data/org.rakuos.Welcome.desktop \
    %{buildroot}/usr/share/applications/org.rakuos.Welcome.desktop
install -Dm644 data/rakuos-welcome-autostart.desktop \
    %{buildroot}/etc/xdg/autostart/rakuos-welcome.desktop

install -Dm755 target/release/rakuos-welcome-gtk \
    %{buildroot}/usr/libexec/rakuos/rakuos-welcome-gtk
install -Dm755 target/release/rakuos-welcome-qt \
    %{buildroot}/usr/libexec/rakuos/rakuos-welcome-qt
install -Dm755 target/release/rakuos-welcome-cosmic \
    %{buildroot}/usr/libexec/rakuos/rakuos-welcome-cosmic

%files
/usr/bin/rakuos-welcome
/usr/share/applications/org.rakuos.Welcome.desktop
/etc/xdg/autostart/rakuos-welcome.desktop

%files gtk
/usr/libexec/rakuos/rakuos-welcome-gtk

%files qt
/usr/libexec/rakuos/rakuos-welcome-qt

%files cosmic
/usr/libexec/rakuos/rakuos-welcome-cosmic

%changelog
* %(date "+%%a %%b %%d %%Y") RakuOS Project <rakuos@rakuos.org> - 1.0.0-1
- Initial Rust rewrite with GTK4, Qt6, and COSMIC frontends
- Split into subpackages per DE: rakuos-welcome-gtk, -qt, -cosmic
