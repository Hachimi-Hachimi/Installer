use std::{fs::File, io::Write, path::PathBuf};

use pelite::resources::version_info::Language;

use crate::utils;

pub struct Installer {
    pub install_dir: Option<PathBuf>,
    pub target: Target,
    pub custom_target: Option<String>
}

impl Installer {
    pub fn custom(install_dir: Option<PathBuf>, target: Option<String>) -> Installer {
        Installer {
            install_dir: install_dir.or_else(Self::detect_install_dir),
            target: Target::Winhttp,
            custom_target: target
        }
    }

    fn detect_install_dir() -> Option<PathBuf> {
        let steam_dir = steamlocate::SteamDir::locate().ok()?;
        let (umamusume, library) = steam_dir.find_app(3224770).ok().flatten()?;
        Some(library.resolve_app_dir(&umamusume))
    }

    pub fn get_target_path(&self, target: Target) -> Option<PathBuf> {
        Some(self.install_dir.as_ref()?.join(target.dll_name()))
    }

    pub fn get_current_target_path(&self) -> Option<PathBuf> {
        let install_dir = self.install_dir.as_ref()?;
        
        Some(
            if let Some(custom_target) = &self.custom_target {
                install_dir.join(custom_target)
            }
            else {
                install_dir.join(self.target.dll_name())
            }
        )
    }

    const LANG_NEUTRAL_UNICODE: Language = Language { lang_id: 0x0000, charset_id: 0x04b0 };
    pub fn get_target_version_info(&self, target: Target) -> Option<TargetVersionInfo> {
        let path = self.get_target_path(target)?;
        let map = pelite::FileMap::open(&path).ok()?;

        // File exists, so return empty version info if we can't read it
        let Some(version_info) = utils::read_pe_version_info(map.as_ref()) else {
            return Some(TargetVersionInfo::default());
        };

        Some(TargetVersionInfo {
            name: version_info.value(Self::LANG_NEUTRAL_UNICODE, "ProductName"),
            version: version_info.value(Self::LANG_NEUTRAL_UNICODE, "ProductVersion")
        })
    }

    pub fn get_target_display_label(&self, target: Target) -> String {
        if let Some(version_info) = self.get_target_version_info(target) {
            version_info.get_display_label(target)
        }
        else {
            target.dll_name().to_owned()
        }
    }

    pub fn is_current_target_installed(&self) -> bool {
        let Some(path) = self.get_current_target_path() else {
            return false;
        };

        let Ok(metadata) = std::fs::metadata(&path) else {
            return false;
        };

        metadata.is_file()
    }

    pub fn get_hachimi_installed_target(&self) -> Option<Target> {
        for target in Target::VALUES {
            if let Some(version_info) = self.get_target_version_info(*target) {
                if version_info.is_hachimi() {
                    return Some(*target);
                }
            }
        }
        None
    }

    pub fn install(&self) -> Result<(), Error> {
        let path = self.get_current_target_path().ok_or(Error::NoInstallDir)?;
        let mut file = File::create(&path)?;

        #[cfg(feature = "compress_dll")]
        file.write(&include_bytes_zstd!("hachimi.dll", 19))?;

        #[cfg(not(feature = "compress_dll"))]
        file.write(include_bytes!("../hachimi.dll"))?;

        Ok(())
    }

    pub fn uninstall(&self) -> Result<(), Error> {
        let path = self.get_current_target_path().ok_or(Error::NoInstallDir)?;
        std::fs::remove_file(&path)?;
        Ok(())
    }
}

impl Default for Installer {
    fn default() -> Installer {
        Installer {
            install_dir: Self::detect_install_dir(),
            target: Target::Winhttp,
            custom_target: None
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Target {
    Winhttp,
    Version,
    Opengl32
}

impl Target {
    pub const VALUES: &[Self] = &[Self::Winhttp, Self::Version, Self::Opengl32];

    pub fn dll_name(&self) -> &'static str {
        match self {
            Self::Winhttp => "winhttp.dll",
            Self::Version => "version.dll",
            Self::Opengl32 => "opengl32.dll"
        }
    }
}

#[derive(Debug, Default)]
pub struct TargetVersionInfo {
    pub name: Option<String>,
    pub version: Option<String>
}

impl TargetVersionInfo {
    pub fn get_display_label(&self, target: Target) -> String {
        let name = self.name.clone().unwrap_or_else(|| "Unknown".to_string());
        format!("* {} ({})", target.dll_name(), name)
    }

    pub fn is_hachimi(&self) -> bool {
        if let Some(name) = &self.name {
            return name == "Hachimi";
        }
        false
    }
}

#[derive(Debug)]
pub enum Error {
    NoInstallDir,
    IoError(std::io::Error)
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NoInstallDir => write!(f, "No install location specified"),
            Error::IoError(error) => write!(f, "I/O error: {}", error)
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::IoError(e)
    }
}