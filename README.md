# monet

One wallpaper per Hyprland workspace, with a directional wipe that follows the workspace slide animation.

monet listens to Hyprland's event socket and calls [awww](https://codeberg.org/LGFae/awww) to switch the wallpaper on the focused monitor. Going to a higher workspace wipes from the right, a lower one from the left.

## Requirements

- Hyprland
- `awww-daemon` running, `awww` in `PATH`

## Configuration

Edit the constants at the top of `src/main.rs`:

- `WALLPAPERS`: one image per workspace, relative to `$XDG_CONFIG_HOME/wallpapers`
- transition duration / bezier in `set_wallpaper`, kept in sync with Hyprland's `animation = workspaces, ...`

## Run

```sh
cargo run --release
```

Logs use `tracing` (`debug` in debug builds, `info` in release).

## References

- Hyprland IPC: https://wiki.hypr.land/IPC/
- awww: https://codeberg.org/LGFae/awww

## Note to self

This is the second time I write Hyprland event parsing by hand. At some point, write a proper lib that covers all events, ideally generated from the IPC docs.
