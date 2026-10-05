#[derive(serde::Deserialize)]
struct Monitor {
    name: String,
    focused: bool,
    #[serde(rename = "activeWorkspace")]
    active_workspace: Workspace,
}

#[derive(serde::Deserialize)]
struct Workspace {
    id: i64,
}

/// Keep track of which workspace is active on which monitor.
struct State {
    wallpapers: [std::path::PathBuf; WORKSPACE_COUNT],
    focused_monitor: String,
    monitor_workspace: std::collections::HashMap<String, usize>,
}

impl State {
    fn init(xdg_config_dir: &str, hypr_dir: &str) -> std::io::Result<Self> {
        /* Build the wallpaper full path */
        let wallpapers = std::array::from_fn(|i| {
            let path = format!("{}/{}/{}", xdg_config_dir, WALLPAPER_DIR, WALLPAPERS[i]);
            std::path::PathBuf::from(path)
        });

        /* Query the monitors from hyprland */
        use std::io::Read;
        use std::io::Write;

        let mut s = std::os::unix::net::UnixStream::connect(format!("{hypr_dir}/.socket.sock"))?;
        s.write_all(b"j/monitors")?;
        let mut out = String::new();
        s.read_to_string(&mut out)?;
        let monitors: Vec<Monitor> = serde_json::from_str(&out).map_err(std::io::Error::other)?;

        let mut focused = None;
        let mut monitor_workspace = std::collections::HashMap::new();

        for m in monitors {
            if m.focused {
                focused = Some(m.name.clone());
            }
            let Ok(workspace_id) = usize::try_from(m.active_workspace.id) else {
                tracing::warn!("Failed to get workspace id for monitor: {}", m.active_workspace.id);
                continue;
            };
            let Some(wallpaper) = workspace_id.checked_sub(1).and_then(|i| wallpapers.get(i)) else {
                tracing::warn!("Failed to get wallpaper for workspace id: {}", workspace_id);
                continue;
            };
            set_wallpaper(&m.name, wallpaper, "none", "0");
            monitor_workspace.insert(m.name, workspace_id);
        }

        let Some(focused_monitor) = focused else {
            return Err(std::io::Error::other(format!("Failed to find a focused workspace!")));
        };

        Ok(Self {
            wallpapers,
            focused_monitor,
            monitor_workspace,
        })
    }

    pub fn handle_event(&mut self, event: &str) -> std::io::Result<()> {
        let &[event_kind, data] = event.split(">>").collect::<Vec<_>>().as_slice() else {
            return Err(std::io::Error::other(format!("Invalid event: {event}")));
        };
        match event_kind {
            "focusedmonv2" => {
                let &[monitor, workspace] = data.split(',').collect::<Vec<_>>().as_slice() else {
                    return Err(std::io::Error::other(format!("Invalid focusedmonv2 event data: {data}")));
                };
                self.focused_monitor = monitor.to_string();
                let workspace_id = workspace
                    .parse::<usize>()
                    .map_err(|_| std::io::Error::other(format!("Invalid workspace id: {workspace}")))?;
                self.monitor_workspace.insert(monitor.to_string(), workspace_id);
            }
            "workspacev2" => {
                let &[workspace_id, _] = data.split(',').collect::<Vec<_>>().as_slice() else {
                    return Err(std::io::Error::other(format!("Invalid workspacev2 event data: {data}")));
                };
                let Ok(workspace) = workspace_id.parse::<usize>() else {
                    /* Special workspace is fine, don't do anything */
                    return Ok(());
                };
                let Some(wallpaper) = workspace.checked_sub(1).and_then(|i| self.wallpapers.get(i)) else {
                    tracing::warn!("No wallpaper for workspace {workspace}");
                    return Ok(());
                };

                let (transition, angle) = match self.monitor_workspace.insert(self.focused_monitor.clone(), workspace) {
                    Some(prev) => match prev.cmp(&workspace) {
                        std::cmp::Ordering::Equal => return Ok(()),
                        std::cmp::Ordering::Less => ("wipe", "0"),
                        std::cmp::Ordering::Greater => ("wipe", "180"),
                    },
                    None => ("fade", "0"),
                };
                tracing::debug!("wallpaper transition to {:?} on {}", wallpaper, self.focused_monitor);
                set_wallpaper(&self.focused_monitor, wallpaper, transition, angle);
            }
            _ => {}
        }

        Ok(())
    }
}

/// Number of workspaces used.
const WORKSPACE_COUNT: usize = 10;

/// Directory where wallpapers shall be looked for, after XDG_DATA_HOME.
const WALLPAPER_DIR: &'static str = "wallpapers";

/// Name of the wallpapers to look for, one per workspace.
const WALLPAPERS: [&'static str; WORKSPACE_COUNT] = [
    "mtg/Rakdos_Wallpaper_2560x1440.jpg",
    "mtg/Boros_Wallpaper_2560x1440.jpg",
    "mtg/Izzet_Wallpaper_2560x1440.jpg",
    "mtg/Orzhov_Wallpaper_2560x1440.jpg",
    "mtg/Azorius_Wallpaper_2560x1440.jpg",
    "mtg/Gruul_Wallpaper_2560x1440.jpg",
    "mtg/Dimir_Wallpaper_2560x1440.jpg",
    "mtg/Golgari_Wallpaper_2560x1440.jpg",
    "mtg/Simic_Wallpaper_2560x1440.jpg",
    "mtg/Selesnya_Wallpaper_2560x1440.jpg",
];

fn main() -> std::io::Result<()> {
    init_logging();

    let xdg_config_dir = match get_env_var("XDG_CONFIG_HOME") {
        Ok(var) => var,
        Err(_) => {
            let home = get_env_var("HOME")?;
            format!("{home}/.config")
        }
    };

    let xdg_runtime_dir = get_env_var("XDG_RUNTIME_DIR")?;
    let hyprland_instance_sig = get_env_var("HYPRLAND_INSTANCE_SIGNATURE")?;

    let hypr_dir = format!("{xdg_runtime_dir}/hypr/{hyprland_instance_sig}");
    let hypr_socket_path = format!("{hypr_dir}/.socket2.sock");
    let hyprland_event_stream = std::os::unix::net::UnixStream::connect(hypr_socket_path)?;
    let hyprland_event_stream = std::io::BufReader::new(hyprland_event_stream);

    let mut state = State::init(&xdg_config_dir, &hypr_dir)?;

    use std::io::BufRead;
    for line in hyprland_event_stream.lines() {
        if let Ok(line) = line {
            if let Err(e) = state.handle_event(&line) {
                tracing::error!("Unable to handle hyprland event: {e}")
            }
        }
    }

    Ok(())
}

fn init_logging() {
    #[cfg(debug_assertions)]
    let log_level = "debug";
    #[cfg(not(debug_assertions))]
    let log_level = "info";

    let filter = tracing_subscriber::EnvFilter::new(log_level);
    let builder = tracing_subscriber::fmt().with_env_filter(filter);

    builder.init();
}

fn get_env_var(key: &str) -> std::io::Result<String> {
    match std::env::var(key) {
        Ok(var) => Ok(var),
        Err(e) => Err(std::io::Error::other(format!("{key} env var unavailable: {e}"))),
    }
}

fn set_wallpaper(monitor: &str, path: &std::path::Path, transition: &str, angle: &str) {
    let mut cmd = std::process::Command::new("awww");
    cmd.args(["img", "-o", monitor])
        .arg(path)
        .args(["--transition-type", transition])
        .args(["--transition-angle", angle])
        .args(["--transition-duration", "0.5"])
        .args(["--transition-bezier", "0.22,1,0.36,1"])
        .args(["--transition-fps", "60"]);
    std::thread::spawn(move || {
        if let Err(e) = cmd.status() {
            tracing::error!("awww failed: {e}");
        }
    });
}
