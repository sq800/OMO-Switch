use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use crate::services::get_home_dir;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

const OMO_PLUGIN_NAMES: [&str; 2] = ["oh-my-openagent", "oh-my-opencode"];

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VersionInfo {
    pub name: String,
    pub current_version: Option<String>,
    pub latest_version: Option<String>,
    pub has_update: bool,
    pub update_command: String,
    pub update_hint: String,
    pub installed: bool,
    pub install_source: Option<String>,
    pub install_path: Option<String>,
    pub detected_from: Option<String>,
}

/// Get opencode current version by executing `opencode --version`
///
/// 通过 PATH 查找（which opencode）定位实际安装路径，兼容 npm 全局安装、
/// scoop、手动安装等场景。3 秒超时防止命令卡住阻塞 UI。
pub fn get_opencode_version() -> Option<(String, std::path::PathBuf)> {
    let found = which::which("opencode").ok()?;
    let version = try_execute_version(&found)?;
    Some((version, found))
}

fn try_execute_version(bin_path: &std::path::Path) -> Option<String> {
    let mut cmd = Command::new(bin_path);
    cmd.arg("--version")
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let mut child = cmd.spawn().ok()?;

    let timeout = Duration::from_secs(3);
    let start = Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => {
                let output = child.wait_with_output().ok()?;
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                return if !version.is_empty() {
                    Some(version)
                } else {
                    None
                };
            }
            Ok(Some(_)) => return None, // 命令执行失败
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(_) => return None,
        }
    }
}

fn get_opencode_config_candidates(home: &Path) -> Vec<PathBuf> {
    let base = home.join(".config").join("opencode");
    vec![base.join("opencode.json"), base.join("opencode.jsonc")]
}

fn parse_json_or_json5(content: &str) -> Option<Value> {
    serde_json::from_str::<Value>(content)
        .or_else(|_| json5::from_str::<Value>(content))
        .ok()
}

fn read_plugin_version_from_config(path: &str, plugin_names: &[&str]) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    let config = parse_json_or_json5(&content)?;
    let plugins = config.get("plugin")?.as_array()?;

    for plugin in plugins {
        if let Some(raw) = plugin.as_str() {
            for plugin_name in plugin_names {
                if let Some(version) = raw.strip_prefix(&format!("{}@", plugin_name)) {
                    return Some(version.to_string());
                }
            }
        }
    }
    None
}

fn is_plugin_declared_in_config(path: &str, plugin_names: &[&str]) -> bool {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => return false,
    };
    let config = match parse_json_or_json5(&content) {
        Some(config) => config,
        None => return false,
    };
    let plugins = match config.get("plugin").and_then(|v| v.as_array()) {
        Some(plugins) => plugins,
        None => return false,
    };

    plugins.iter().any(|plugin| {
        let Some(raw) = plugin.as_str() else {
            return false;
        };
        plugin_names
            .iter()
            .any(|name| raw == *name || raw.starts_with(&format!("{}@", name)))
    })
}

fn check_omo_in_config(home: &Path) -> Option<(Option<String>, PathBuf)> {
    for config_path in get_opencode_config_candidates(home) {
        let cp_str = config_path.to_string_lossy();
        if let Some(version) = read_plugin_version_from_config(&cp_str, &OMO_PLUGIN_NAMES) {
            return Some((Some(version), config_path));
        }
        if is_plugin_declared_in_config(&cp_str, &OMO_PLUGIN_NAMES) {
            return Some((None, config_path));
        }
    }
    None
}

fn is_omo_installed() -> bool {
    let home = match get_home_dir() {
        Ok(h) => h,
        Err(_) => return false,
    };
    check_omo_in_config(&home).is_some()
}

fn get_npm_latest_version(package_name: &str) -> Option<String> {
    let url = format!("https://registry.npmjs.org/{}/latest", package_name);
    let resp = ureq::get(&url)
        .timeout(std::time::Duration::from_secs(4))
        .call()
        .ok()?;
    let json: serde_json::Value = resp.into_json().ok()?;
    json.get("version")?.as_str().map(|s| s.to_string())
}

/// Get Oh My OpenAgent latest version from npm registry (兼容旧包名)
pub fn get_omo_latest_version() -> Option<String> {
    get_npm_latest_version("oh-my-openagent").or_else(|| get_npm_latest_version("oh-my-opencode"))
}

/// Get OpenCode latest version from GitHub Releases
pub fn get_opencode_latest_version() -> Option<String> {
    let resp = ureq::get("https://api.github.com/repos/anomalyco/opencode/releases/latest")
        .set("User-Agent", "OMO-Switch")
        .timeout(std::time::Duration::from_secs(3))
        .call()
        .ok()?;
    let json: serde_json::Value = resp.into_json().ok()?;
    json.get("tag_name")?
        .as_str()
        .map(|s| s.trim_start_matches('v').to_string())
}

/// Simple semver comparison: returns true if latest > current
pub fn has_newer_version(current: &str, latest: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> { v.split('.').filter_map(|s| s.parse().ok()).collect() };
    let c = parse(current);
    let l = parse(latest);
    l > c
}

/// Check all versions
pub fn check_all_versions() -> Vec<VersionInfo> {
    let mut results = Vec::new();

    // OpenCode
    let oc_result = get_opencode_version();
    let oc_current = oc_result.as_ref().map(|(v, _)| v.clone());
    let oc_path = oc_result.as_ref().map(|(_, p)| p.to_string_lossy().to_string());
    let oc_latest = get_opencode_latest_version();
    results.push(VersionInfo {
        name: "OpenCode".to_string(),
        installed: oc_result.is_some(),
        current_version: oc_current.clone(),
        latest_version: oc_latest.clone(),
        has_update: match (&oc_current, &oc_latest) {
            (Some(c), Some(l)) => has_newer_version(c, l),
            _ => false,
        },
        update_command: "opencode upgrade".to_string(),
        update_hint: "Run 'opencode upgrade' in terminal".to_string(),
        install_source: None,
        install_path: oc_path.clone(),
        detected_from: oc_path,
    });

    // Oh My OpenAgent
    let mut omo_current;
    let omo_config_path;
    let omo_installed;
    if let Ok(home) = get_home_dir() {
        if let Some((version, config_path)) = check_omo_in_config(&home) {
            omo_current = version;
            omo_config_path = Some(config_path.to_string_lossy().to_string());
            omo_installed = true;
        } else {
            omo_current = None;
            omo_config_path = None;
            omo_installed = is_omo_installed();
        }
    } else {
        omo_current = None;
        omo_config_path = None;
        omo_installed = false;
    }

    let omo_is_latest = omo_current.as_deref() == Some("latest");
    let omo_latest = get_omo_latest_version();
    if omo_is_latest {
        omo_current = omo_latest.clone();
    }
    let has_update = match (&omo_current, &omo_latest) {
        (Some(c), Some(l)) if !omo_is_latest => has_newer_version(c, l),
        _ => false,
    };
    results.push(VersionInfo {
        name: "Oh My OpenAgent".to_string(),
        installed: omo_installed,
        current_version: omo_current,
        latest_version: omo_latest,
        has_update,
        update_command: "bunx oh-my-openagent install".to_string(),
        update_hint: "Run 'bunx oh-my-openagent install' in terminal".to_string(),
        install_source: None,
        install_path: omo_config_path.clone(),
        detected_from: omo_config_path,
    });

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_newer_version() {
        assert!(has_newer_version("3.5.2", "3.5.3"));
        assert!(!has_newer_version("3.5.3", "3.5.3"));
        assert!(!has_newer_version("3.5.3", "3.5.2"));
        assert!(has_newer_version("3.4.0", "3.5.0"));
    }
}
