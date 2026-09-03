Name:           kally
Version:        0.1.0
Release:        1%{?dist}
Summary:        Git-first package manager for Kalcite
License:        MIT
BuildRequires:  cargo, rust

%description
Kally resolves and materializes reproducible Git dependencies for Kalcite.

%prep
%autosetup -n kally-%{version}

%build
cargo build --release

%install
install -Dm755 target/release/kally %{buildroot}%{_bindir}/kally

%files
%license LICENSE
%{_bindir}/kally
