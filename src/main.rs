/// Keep track of which workspace is active on which monitor.
struct State {
    wallpapers: [std::path::PathBuf; WORKSPACE_COUNT],
    focused_monitor: Option<hipc::types::MonitorName>,
    monitor_workspace: std::collections::HashMap<hipc::types::MonitorName, hipc::types::WorkspaceId>,
}

impl State {
    fn init(xdg_config_dir: &str) -> std::io::Result<Self> {
        /* Build the wallpaper full path */
        let wallpapers = std::array::from_fn(|i| {
            let path = format!("{}/{}/{}", xdg_config_dir, WALLPAPER_DIR, WALLPAPERS[i]);
            std::path::PathBuf::from(path)
        });

        let mut result = Self {
            wallpapers,
            focused_monitor: None,
            monitor_workspace: std::collections::HashMap::new(),
        };
        result.load_from_monitors()?;

        Ok(result)
    }

    fn load_from_monitors(&mut self) -> std::io::Result<()> {
        let monitors = hipc::commands::monitors()?;

        let mut focused_monitor = None;
        self.monitor_workspace.clear();

        for monitor in monitors {
            if monitor.focused {
                focused_monitor = Some(monitor.name.clone());
            }
            let wallpaper_index = monitor.active_workspace.id.raw() - 1;
            let wallpaper_index = match usize::try_from(wallpaper_index) {
                Ok(index) => index,
                Err(_) => {
                    tracing::warn!("Unable to convert workspace id to wallpaper index: {}", wallpaper_index);
                    continue;
                }
            };
            let Some(wallpaper) = self.wallpapers.get(wallpaper_index) else {
                tracing::warn!("Failed to get wallpaper for workspace id: {}", wallpaper_index);
                continue;
            };
            set_wallpaper(&monitor.name, wallpaper);
            self.monitor_workspace.insert(monitor.name, monitor.active_workspace.id);
        }

        self.focused_monitor = focused_monitor;

        Ok(())
    }

    pub fn handle_event(&mut self, event: hipc::HyprlandEvent) -> std::io::Result<()> {
        match event {
            hipc::HyprlandEvent::FocusedMonitorV2 { monitor, workspace } => {
                self.focused_monitor = Some(monitor.clone());
                self.monitor_workspace.insert(monitor, workspace);
            }
            hipc::HyprlandEvent::WorkspaceV2 { id, .. } => {
                let wallpaper_index = match usize::try_from(id.raw() - 1) {
                    Ok(id) => id,
                    Err(_) => {
                        tracing::warn!("Unable to convert workspace id to wallpaper index: {}", id.raw() - 1);
                        return Ok(());
                    }
                };
                let Some(wallpaper) = self.wallpapers.get(wallpaper_index) else {
                    tracing::warn!("No wallpaper for workspace {}", id.raw());
                    return Ok(());
                };
                let Some(focused_monitor) = &self.focused_monitor else {
                    tracing::warn!("No focued monitor, unable to make transition");
                    return Ok(());
                };
                self.monitor_workspace.insert(focused_monitor.clone(), id);
                tracing::debug!("wallpaper transition to {:?} on {}", wallpaper, &**focused_monitor);
                set_wallpaper(focused_monitor, wallpaper);
            }
            hipc::HyprlandEvent::MonitorAddedV2 { .. } | hipc::HyprlandEvent::MonitorRemovedV2 { .. } => {
                self.load_from_monitors()?;
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

    let mut hyprland_socket = hipc::HyprlandEventSocket::connect()?;

    let mut state = State::init(&xdg_config_dir)?;

    loop {
        match hyprland_socket.read() {
            Ok(event) => {
                if let Err(e) = state.handle_event(event) {
                    tracing::error!("Unable to handle hyprland event: {e}");
                }
            }
            Err(e) => tracing::error!("Unable to parse hyprland event: {e}"),
        }
    }
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

fn set_wallpaper(monitor: &str, path: &std::path::Path) {
    let mut cmd = std::process::Command::new("awww");
    cmd.args(["img", "-o", monitor]).arg(path);
    std::thread::spawn(move || {
        if let Err(e) = cmd.status() {
            tracing::error!("awww failed: {e}");
        }
    });
}
