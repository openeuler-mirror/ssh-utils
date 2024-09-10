%define debug_package %{nil}

Name:           ssh-utils
Version:        0.1.0
Release:        1%{?dist}
Summary:        ssh-utils is a tool for fast ssh connections.

License:        MulanPSL-2.0
URL:            https://gitee.com/openeuler/ssh-utils
Source0:        https://gitee.com/openeuler/ssh-utils/repository/archive/v0.1.0.zip

BuildRequires:  rust cargo openssl-devel

%description
ssh-utils is a tool for fast ssh connections.

%prep
%setup -q -n %{name}-v%{version}

%build
cargo build --release

%install
install -D -m 0755 target/release/ssh-utils %{buildroot}/usr/bin/ssh-utils

%files
%license LICENSE
%doc README.md
/usr/bin/ssh-utils

%changelog
* Tue Sep 10 2024 Kurisu <i@kuri.su> - 0.1.0
  - Initial release.
