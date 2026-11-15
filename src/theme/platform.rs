/// Whose conventions the components follow: window buttons, the title bar's layout and key caps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    Mac,
    Windows,
    Linux,
}

impl Platform {
    /// The GPUI operating system that names this platform's keys.
    pub fn os(self) -> gpui::OperatingSystem {
        match self {
            Platform::Mac => gpui::OperatingSystem::Mac,
            Platform::Windows => gpui::OperatingSystem::Windows,
            Platform::Linux => gpui::OperatingSystem::Linux,
        }
    }

    /// The system this build targets, where the theme starts.
    pub fn current() -> Self {
        if cfg!(target_os = "macos") {
            Platform::Mac
        } else if cfg!(target_os = "windows") {
            Platform::Windows
        } else {
            Platform::Linux
        }
    }
}
