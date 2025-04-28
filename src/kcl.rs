use std::fs;
use zed_extension_api::{self as zed, LanguageServerId, Result};

struct KclExtension {
    cached_binary_path: Option<String>,
}

impl KclExtension {
    fn language_server_binary_path(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<String> {
        if let Some(path) = worktree.which("kcl-language-server") {
            return Ok(path);
        }

        if let Some(path) = &self.cached_binary_path {
            if fs::metadata(path).map_or(false, |stat| stat.is_file()) {
                return Ok(path.clone());
            }
        }

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );
        let release = zed::latest_github_release(
            "kcl-lang/kcl",
            zed::GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;

        let (platform, arch) = zed::current_platform();

        // Get OS name based on platform
        let os_name = match platform {
            zed::Os::Mac => "darwin",
            zed::Os::Linux => "linux",
            zed::Os::Windows => "windows",
        };

        // Get architecture name
        let arch_name = match arch {
            zed::Architecture::Aarch64 => "arm64",
            zed::Architecture::X86 => "amd64",
            zed::Architecture::X8664 =>
                return Err(format!("unsupported architecture: {arch:?}")),
        };

        // Format asset name differently for Windows vs Linux/macOS
        let asset_name = if platform == zed::Os::Windows {
            format!(
                "kclvm-{}-{}.zip",
                release.version.clone(),
                os_name
            )
        } else {
            format!(
                "kclvm-{}-{}-{}.tar.gz",
                release.version.clone(),
                os_name,
                arch_name
            )
        };

        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == asset_name)
            .ok_or_else(|| format!("no asset found matching {:?}", asset_name))?;

        let version_dir = format!("kcl-language-server-{}", release.version.clone());
        let binary_path = format!("{version_dir}/kclvm/bin/kcl-language-server");

        if !fs::metadata(&binary_path).map_or(false, |stat| stat.is_file()) {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );

            let downloaded_file_type = if platform == zed::Os::Windows {
                zed::DownloadedFileType::Zip
                } else {
                    zed::DownloadedFileType::GzipTar
                };
            zed::download_file(
                &asset.download_url,
                &version_dir,
                downloaded_file_type
            )
            .map_err(|e| format!("failed to download file: {e}"))?;

            zed::make_file_executable(&binary_path)?;

            let entries =
                fs::read_dir(".").map_err(|e| format!("failed to list working directory {e}"))?;
            for entry in entries {
                let entry = entry.map_err(|e| format!("failed to load directory entry {e}"))?;
                if entry.file_name().to_str() != Some(&version_dir) {
                    fs::remove_dir_all(entry.path()).ok();
                }
            }
        }

        self.cached_binary_path = Some(binary_path.clone());
        Ok(binary_path)
    }
}

impl zed::Extension for KclExtension {
    fn new() -> Self {
        Self {
            cached_binary_path: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        Ok(zed::Command {
            command: self.language_server_binary_path(language_server_id, worktree)?,
            args: Vec::new(),
            env: Default::default(),
        })
    }
}

zed::register_extension!(KclExtension);
