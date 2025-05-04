# Photoflow

It is a simple gamepad-controlled media gallery viewer
that aggregates media form different sources.

![Screenshot](screenshot.png)

The main purpose of the application is to show photos and videos
from different phones and cameras on my HTPC.

It supports only most popular audio and video formats: `jpeg`, `heic`, `mp4`, `mov`.

## Controls

### Gallery Screen

| Gamepad              | Keyboard   | Action                   |
|----------------------|------------|--------------------------|
| DPad Buttons         | Arrow Keys | Move the focus indicator |
| Bottom Action Button | Enter      | View selected media file |
| Right Action Button  | Esc        | Exit application         |

### Media Viewer Screen

| Gamepad                 | Keyboard              | Action                        |
|-------------------------|-----------------------|-------------------------------|
| DPad Left/Right Buttons | Arrow Left/Right Keys | Load previous/next media file |
| Bottom Action Button    | Enter                 | Play/pause video              |
| Triggers L2/R2          | Home/End              | Rewind/fast forward video     |
| Right Action Button     | Esc                   | Go back to the Gallery        |

## Configuration

The config file should be located at `$XDG_CONFIG_HOME/phtoflow.toml`.
You just need to set a list of directories for indexing.

Example config:

```toml
sources = [
    "/home/user/photos/android/",
    "/home/user/photos/camera/",
    "/home/user/photos/iphone/"
]
```

## Build

The application is written in Rust, so it is built with `cargo`. It uses `Slint` as a graphical
toolkit
and uses `winit` as a backend
(required to work with windowing API not provided by `Slint`).

You can build it for Wayland, X11 or for both platforms using `wayland` (default) and `x11`
features:

- `cargo build --release` produces the Wayland-only build,
- `cargo build --release --features x11` produces the universal build that support both Wayland and
  X11
- `cargo build --release --no-default-features --features x11` produces the X11-only build