# qobuz_identifier

A small command-line tool that takes a qobuz ID and matches it to MusicBrainz releases by barcode.

## Setup

Before use, a Qobuz App ID must be obtained (see https://github.com/ImAiiR/QobuzDownloaderX/issues/2#issuecomment-575852400)

This must then either be passed directly to the application via the command-line or the environment variable `QOBUZ_IDENTNFIER_APPID`, or placed in a file whose path is passed to the application by the same method.

## Usage

A Nix flake is provided for convenience:

```sh
nix run github:Sciencentistguy/qobuz_identifier -- '<URL>'
```

It can also be run or installed via cargo

```sh
git clone https://github.com/Sciencentistguy/qobuz_identifier.git \
    && cd qobuz_identifier \
    && cargo run
# or
cargo install --git https://github.com/Sciencentistguy/qobuz_identifier.git
```

## Disclaimer

This software is published for research purposes. Please familiarise yourself with the Qobuz terms of service in your region before use.

---

Made available under the terms of version 2.0 of the Mozilla Public Licence
