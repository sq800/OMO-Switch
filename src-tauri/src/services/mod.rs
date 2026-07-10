pub mod config_cache_service;
pub mod config_service;
pub mod import_export_service;
pub mod model_service;
pub mod preset_service;
pub mod provider_service;
pub mod provider_store;
pub mod version_service;

use std::path::PathBuf;

pub(crate) fn get_home_dir() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .or_else(|| std::env::var_os("USERPROFILE").filter(|value| !value.is_empty()))
        .map(PathBuf::from)
        .ok_or_else(|| "无法获取 HOME 或 USERPROFILE 环境变量".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    struct HomeEnvGuard {
        home: Option<std::ffi::OsString>,
        userprofile: Option<std::ffi::OsString>,
    }

    impl HomeEnvGuard {
        fn capture() -> Self {
            Self {
                home: std::env::var_os("HOME"),
                userprofile: std::env::var_os("USERPROFILE"),
            }
        }
    }

    impl Drop for HomeEnvGuard {
        fn drop(&mut self) {
            unsafe {
                match &self.home {
                    Some(value) => std::env::set_var("HOME", value),
                    None => std::env::remove_var("HOME"),
                }
                match &self.userprofile {
                    Some(value) => std::env::set_var("USERPROFILE", value),
                    None => std::env::remove_var("USERPROFILE"),
                }
            }
        }
    }

    #[test]
    #[serial]
    fn test_get_home_dir_uses_userprofile_when_home_is_missing() {
        let _guard = HomeEnvGuard::capture();
        unsafe {
            std::env::remove_var("HOME");
            std::env::set_var("USERPROFILE", r"C:\Users\omo-test");
        }

        let home = get_home_dir().expect("应回退到 USERPROFILE");

        assert_eq!(home, PathBuf::from(r"C:\Users\omo-test"));
    }

    #[test]
    #[serial]
    fn test_get_home_dir_prefers_home() {
        let _guard = HomeEnvGuard::capture();
        unsafe {
            std::env::set_var("HOME", "/tmp/omo-home");
            std::env::set_var("USERPROFILE", r"C:\Users\omo-test");
        }

        let home = get_home_dir().expect("应优先使用 HOME");

        assert_eq!(home, PathBuf::from("/tmp/omo-home"));
    }
}
