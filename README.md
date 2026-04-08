# kwtypr

`kwtypr` is [KWtype](https://github.com/Sporif/KWtype), but blazingly fast™.

It types text through KWin's privileged [Fake Input](https://wayland.app/protocols/kde-fake-input) protocol, so it only works on KDE Plasma Wayland and requires an installed desktop entry with `org_kde_kwin_fake_input`. Character mapping is derived from the compositor-provided XKB keymap.

## Requirements

- KDE Plasma on Wayland
- An installed desktop entry declaring `org_kde_kwin_fake_input`
- `just` for the recommended install flow

## Install

If you do not have `just` yet, install it first using the [official installation instructions](https://github.com/casey/just?tab=readme-ov-file#installation).

After installing `just`, you can _just_ run:

```sh
just install
```

This builds `kwtypr`, installs the binary to `~/.local/bin/kwtypr`, installs the desktop entry to `~/.local/share/applications/kwtypr.desktop`, and refreshes KDE's desktop cache so KWin picks up the required fake-input permission.

If `~/.local/bin` is not on your `PATH`, add it before trying to run `kwtypr`.

For a different prefix, for example a system-wide install under `/usr/local`, run:

```sh
sudo just install prefix=/usr/local
```

To remove an installation:

```sh
just uninstall
```

## Usage

```sh
kwtypr [OPTIONS] <TEXT>...
```

Positional arguments are joined with spaces before typing.

Useful options:

- `--initial-delay <MS>`: wait before typing starts
- `--character-delay <MS>`: wait between characters
- `--key-hold <MS>`: hold each key before release
- `--unicode-fallback`: enable `Ctrl+Shift+U` Unicode input fallback
- `--ready-timeout <MS>`: fail if Wayland initialization takes too long

## Differences From KWtype

- `kwtypr` joins positional arguments with spaces, so `kwtypr hello world` types `hello world`.
- Unicode fallback is not enabled by default. Pass `--unicode-fallback` if you want `Ctrl+Shift+U` fallback for characters that cannot be typed directly with the current layout.
- If typing completes but some characters could not be mapped, `kwtypr` keeps going, reports the failures, and exits with code `2`.

## Exit Codes

- `0`: all requested characters were typed
- `1`: initialization or Wayland I/O failed
- `2`: typing completed, but one or more characters could not be typed with the current layout

## Troubleshooting

If some applications miss the first character, or the typed text does not appear reliably, add a small initial delay:

```sh
kwtypr --initial-delay 1 "example text"
```

In practice, an initial delay of `1` millisecond appears to fix input in Google Chrome in setups where Chromium works without it.

If a character cannot be typed directly with the current layout, enable Unicode fallback explicitly:

```sh
kwtypr --unicode-fallback "..."
```

If `kwtypr` still reports that a character could not be typed, check whether the current keyboard layout provides the keys needed for that fallback sequence.
