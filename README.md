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

## Install

On my nixos config, I installed it with:

```nix
{ config, pkgs, ... }:
let
  awww = "${pkgs.awww}/bin/awww";

  monetSrc = pkgs.fetchFromGitHub {
    owner = "VirgileHenry";
    repo = "monet";
    rev = "d58d571ad188af0b6232cd8b36da62dcde58e321";
    sha256 = "sha256-KYtwkKLWLsAKKuA/aMSnUcCt8bwGj1GKbIwhy/04weY=";
  };
  monet = pkgs.rustPlatform.buildRustPackage {
    pname = "monet";
    version = "0.1.0";
    src = monetSrc;
    cargoHash = "sha256-HfxNxB2rnQDb3hKLkmrsI06sfxstSPyXsIo8WwybUKg=";
    meta = with pkgs.lib; {
      description = "Per-workspace wallpaper switcher";
    };
  };
in
{
  systemd.user.services = {
    awww = {
      Unit = {
        Description = "awww wallpaper daemon";
        PartOf = [ "graphical-session.target" ];
        After = [ "graphical-session.target" ];
      };
      Service = {
        ExecStart = "${pkgs.awww}/bin/awww-daemon";
        # unit only counts as "started" once the daemon answers
        ExecStartPost = "${pkgs.bash}/bin/sh -c 'until ${awww} query >/dev/null 2>&1; do sleep 0.05; done'";
        Restart = "on-failure";
      };
      Install.WantedBy = [ "graphical-session.target" ];
    };

    monet = {
      Unit = {
        Description = "Per-workspace wallpaper switcher";
        Requires = [ "awww.service" ];
        After = [ "awww.service" ];
        PartOf = [ "awww.service" ];
      };
      Service = {
        ExecStart = "${monet}/bin/monet";
        Environment = "PATH=${pkgs.awww}/bin";
        Restart = "on-failure";
        RestartSec = 1;
      };
      Install.WantedBy = [ "awww.service" ];
    };
  };
}
```
