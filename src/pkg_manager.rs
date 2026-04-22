use std::process::Command;

use crate::{AppError, AppResult, Package, PackageStatus};

#[derive(Clone)]
pub struct PackageManager {
    pub name: String,
    pub command: String,
    pub list_command: String,
    pub available: bool,
    pub needs_root: bool,
}

impl PackageManager {
    pub fn new(name: &str, command: &str, needs_root: bool) -> Self {
        Self {
            name: name.to_string(),
            command: command.to_string(),
            list_command: command.to_string(),
            available: is_command_available(command),
            needs_root,
        }
    }

    pub async fn list_packages(&self) -> AppResult<Vec<Package>> {
        match self.name.as_str() {
            "pip" => self.list_pip().await,
            "npm" => self.list_npm().await,
            "cargo" => self.list_cargo().await,
            "brew" => self.list_brew().await,
            "apt" => self.list_apt().await,
            "pacman" => self.list_pacman().await,
            "aur" => self.list_aur().await,
            "rpm" => self.list_rpm().await,
            "flatpak" => self.list_flatpak().await,
            "snap" => self.list_snap().await,
            _ => Ok(Vec::new()),
        }
    }

    pub async fn install_package(&self, name: &str) -> AppResult<String> {
        let output = match self.name.as_str() {
            "pip" => Command::new(&self.command).args(["install", name]).output()?,
            "npm" => Command::new(&self.command).args(["install", "-g", name]).output()?,
            "cargo" => Command::new(&self.command).args(["install", name]).output()?,
            "brew" => Command::new(&self.command).args(["install", name]).output()?,
            "apt" => Command::new("sudo").args([&self.command, "install", name]).output()?,
            "pacman" => Command::new("sudo").args([&self.command, "-S", name]).output()?,
            "aur" => Command::new(&self.command).args(["-S", name]).output()?,
            "rpm" => Command::new("sudo").args([&self.command, "-ivh", name]).output()?,
            "flatpak" => Command::new(&self.command).args(["install", "flathub", name]).output()?,
            "snap" => Command::new("sudo").args([&self.command, "install", name]).output()?,
            _ => return Err(AppError::from("Unknown package manager")),
        };

        if output.status.success() {
            Ok(format!("Successfully installed {}", name))
        } else {
            Err(AppError::from(String::from_utf8_lossy(&output.stderr).to_string()))
        }
    }

    pub async fn remove_package(&self, name: &str) -> AppResult<String> {
        let output = match self.name.as_str() {
            "pip" => Command::new(&self.command).args(["uninstall", "-y", name]).output()?,
            "npm" => Command::new(&self.command).args(["uninstall", "-g", name]).output()?,
            "cargo" => Command::new(&self.command).args(["uninstall", name]).output()?,
            "brew" => Command::new(&self.command).args(["uninstall", name]).output()?,
            "apt" => Command::new("sudo").args([&self.command, "remove", name]).output()?,
            "pacman" => Command::new("sudo").args([&self.command, "-R", name]).output()?,
            "aur" => Command::new(&self.command).args(["-R", name]).output()?,
            "rpm" => Command::new("sudo").args([&self.command, "-e", name]).output()?,
            "flatpak" => Command::new(&self.command).args(["uninstall", name]).output()?,
            "snap" => Command::new("sudo").args([&self.command, "remove", name]).output()?,
            _ => return Err(AppError::from("Unknown package manager")),
        };

        if output.status.success() {
            Ok(format!("Successfully removed {}", name))
        } else {
            Err(AppError::from(String::from_utf8_lossy(&output.stderr).to_string()))
        }
    }

    pub async fn upgrade_package(&self, name: &str) -> AppResult<String> {
        if self.name == "apt" {
            let update = Command::new("sudo").args([&self.command, "update"]).output()?;
            if !update.status.success() {
                return Err(AppError::from(String::from_utf8_lossy(&update.stderr).to_string()));
            }
            let output = Command::new("sudo").args([&self.command, "upgrade", name]).output()?;
            return if output.status.success() {
                Ok(format!("Successfully upgraded {}", name))
            } else {
                Err(AppError::from(String::from_utf8_lossy(&output.stderr).to_string()))
            };
        }

        let output = match self.name.as_str() {
            "pip" => Command::new(&self.command).args(["install", "--upgrade", name]).output()?,
            "npm" => Command::new(&self.command).args(["update", "-g", name]).output()?,
            "cargo" => Command::new(&self.command).args(["install", name]).output()?,
            "brew" => Command::new(&self.command).args(["upgrade", name]).output()?,
            "pacman" => Command::new("sudo").args([&self.command, "-S", name]).output()?,
            "aur" => Command::new(&self.command).args(["-S", name]).output()?,
            "rpm" => Command::new("sudo").args([&self.command, "-Uvh", name]).output()?,
            "flatpak" => Command::new(&self.command).args(["update", name]).output()?,
            "snap" => Command::new("sudo").args([&self.command, "refresh", name]).output()?,
            _ => return Err(AppError::from("Unknown package manager")),
        };

        if output.status.success() {
            Ok(format!("Successfully upgraded {}", name))
        } else {
            Err(AppError::from(String::from_utf8_lossy(&output.stderr).to_string()))
        }
    }

    async fn list_pip(&self) -> AppResult<Vec<Package>> {
        let output = Command::new("sh")
            .args(["-c", "pip list --format=json 2>/dev/null || pip3 list --format=json"])
            .output()?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let packages: Vec<serde_json::Value> = serde_json::from_str(&stdout).unwrap_or_default();

        let mut result = Vec::new();
        for pkg in packages {
            result.push(Package {
                name: pkg["name"].as_str().unwrap_or("").to_string(),
                version: pkg["version"].as_str().unwrap_or("").to_string(),
                status: PackageStatus::Installed,
                size: 0,
                description: String::new(),
                repository: None,
                installed_by: None,
            });
        }

        Ok(result)
    }

    async fn list_npm(&self) -> AppResult<Vec<Package>> {
        let output = Command::new("sh")
            .args(["-c", "npm list -g --json --depth=0 2>/dev/null || npm list -g --json"])
            .output()?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let data: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_default();

        let mut result = Vec::new();
        if let Some(dependencies) = data["dependencies"].as_object() {
            for (name, info) in dependencies {
                let version = info["version"].as_str().unwrap_or("").to_string();
                let description = info["description"].as_str().unwrap_or("").to_string();

                result.push(Package {
                    name: name.clone(),
                    version,
                    status: PackageStatus::Installed,
                    size: 0,
                    description,
                    repository: None,
                    installed_by: None,
                });
            }
        }

        Ok(result)
    }

    async fn list_cargo(&self) -> AppResult<Vec<Package>> {
        let output = Command::new("sh")
            .args(["-c", "cargo install --list 2>/dev/null"])
            .output()?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut result = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.splitn(2, " v").collect();
            if parts.len() == 2 {
                result.push(Package {
                    name: parts[0].trim().to_string(),
                    version: parts[1].trim().to_string(),
                    status: PackageStatus::Installed,
                    size: 0,
                    description: String::new(),
                    repository: None,
                    installed_by: None,
                });
            }
        }

        Ok(result)
    }

    async fn list_brew(&self) -> AppResult<Vec<Package>> {
        let output = Command::new("sh")
            .args(["-c", "brew list --versions 2>/dev/null"])
            .output()?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut result = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if !parts.is_empty() {
                let name = parts[0].to_string();
                let version = if parts.len() > 1 {
                    parts[1..].join(" ")
                } else {
                    "".to_string()
                };
                result.push(Package {
                    name,
                    version,
                    status: PackageStatus::Installed,
                    size: 0,
                    description: String::new(),
                    repository: Some("homebrew".to_string()),
                    installed_by: None,
                });
            }
        }

        Ok(result)
    }

    async fn list_apt(&self) -> AppResult<Vec<Package>> {
        let output = Command::new("sh")
            .args(["-c", "dpkg-query -W -f='${package} ${version} ${status}\\n' 2>/dev/null"])
            .output()?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut result = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split(' ').collect();
            if parts.len() >= 3 {
                let status = if parts[2].contains("installed") {
                    PackageStatus::Installed
                } else {
                    PackageStatus::Available
                };

                result.push(Package {
                    name: parts[0].to_string(),
                    version: parts[1].to_string(),
                    status,
                    size: 0,
                    description: String::new(),
                    repository: None,
                    installed_by: None,
                });
            }
        }

        Ok(result)
    }

    async fn list_pacman(&self) -> AppResult<Vec<Package>> {
        let output = Command::new("sh")
            .args(["-c", "pacman -Q 2>/dev/null"])
            .output()?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut result = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split(' ').collect();
            if parts.len() >= 2 {
                result.push(Package {
                    name: parts[0].to_string(),
                    version: parts[1].to_string(),
                    status: PackageStatus::Installed,
                    size: 0,
                    description: String::new(),
                    repository: Some("core".to_string()),
                    installed_by: None,
                });
            }
        }

        Ok(result)
    }

    async fn list_aur(&self) -> AppResult<Vec<Package>> {
        let output = Command::new("sh")
            .args(["-c", "yay -Qem 2>/dev/null || paru -Qem 2>/dev/null"])
            .output()?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut result = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split(' ').collect();
            if parts.len() >= 2 {
                result.push(Package {
                    name: parts[0].to_string(),
                    version: parts[1].to_string(),
                    status: PackageStatus::Installed,
                    size: 0,
                    description: String::new(),
                    repository: Some("aur".to_string()),
                    installed_by: None,
                });
            }
        }

        Ok(result)
    }

    async fn list_rpm(&self) -> AppResult<Vec<Package>> {
        let output = Command::new("sh")
            .args(["-c", "rpm -qa --queryformat '%{NAME}\\n%{VERSION}\\n' 2>/dev/null"])
            .output()?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = stdout.lines().collect();
        let mut result = Vec::new();

        let mut i = 0;
        while i < lines.len() - 1 {
            let name = lines[i].to_string();
            let version = lines.get(i + 1).unwrap_or(&"").to_string();
            result.push(Package {
                name,
                version,
                status: PackageStatus::Installed,
                size: 0,
                description: String::new(),
                repository: None,
                installed_by: None,
            });
            i += 2;
        }

        Ok(result)
    }

    async fn list_flatpak(&self) -> AppResult<Vec<Package>> {
        let output = Command::new("sh")
            .args(["-c", "flatpak list --app --columns=application,version 2>/dev/null"])
            .output()?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut result = Vec::new();

        for line in stdout.lines().skip(1) {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 2 {
                result.push(Package {
                    name: parts[0].to_string(),
                    version: parts[1].to_string(),
                    status: PackageStatus::Installed,
                    size: 0,
                    description: String::new(),
                    repository: Some("flathub".to_string()),
                    installed_by: None,
                });
            }
        }

        Ok(result)
    }

    async fn list_snap(&self) -> AppResult<Vec<Package>> {
        let output = Command::new("sh")
            .args(["-c", "snap list 2>/dev/null"])
            .output()?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut result = Vec::new();

        for line in stdout.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                result.push(Package {
                    name: parts[0].to_string(),
                    version: parts[1].to_string(),
                    status: PackageStatus::Installed,
                    size: 0,
                    description: String::new(),
                    repository: None,
                    installed_by: None,
                });
            }
        }

        Ok(result)
    }
}

fn is_command_available(cmd: &str) -> bool {
    Command::new("sh")
        .args(["-c", &format!("command -v {}", cmd)])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}