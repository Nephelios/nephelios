use dirs::home_dir;
use std::process::Command;
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Removes the temporary directory and its contents from the user's home folder.
///
/// # Arguments
///
/// * `target_dir` - The path to the directory to be removed.
///
/// # Returns
/// * `Ok(())` if the directory was removed successfully.
/// * `Err(String)` if there was an error removing the directory.
pub fn remove_temp_dir(target_dir: &Path) -> Result<(), String> {
    if target_dir.exists() {
        fs::remove_dir_all(target_dir)
            .map_err(|e| format!("Failed to remove temporary directory: {}", e))?;
    }
    Ok(())
}

/// Creates a temporary directory in the user's home folder for the specified app.
///
/// # Arguments
///
/// * `app_name` - The name of the application for which the temporary directory is created.
///
/// # Returns
/// * `Ok(PathBuf)` containing the path to the created temporary directory.
/// * `Err(String)` if the directory could not be created.
pub fn create_temp_dir(app_name: &str) -> Result<PathBuf, String> {
    let home = home_dir().ok_or("Failed to find home directory")?;

    let temp_dir = home.join(format!(".{}-tmp", app_name));

    fs::create_dir_all(&temp_dir).map_err(|e| format!("Failed to create temp directory: {}", e))?;

    Ok(temp_dir)
}

/// Clones a GitHub repository into a specified directory.
///
/// # Arguments
///
/// * `github_url` - The URL of the GitHub repository to clone.
/// * `target_dir` - The directory where the repository will be cloned.
///
/// # Returns
/// * `Ok(())` if the repository was successfully cloned.
/// * `Err(String)` if there was an error during the cloning process.
pub fn clone_repo(github_url: &str, target_dir: &str) -> Result<(), String> {
    let status = Command::new("git")
        .args(["clone", github_url, target_dir])
        .status()
        .map_err(|e| format!("Failed to execute git: {}", e))?;

    if !status.success() {
        return Err("Failed to clone repository. Check URL and permissions.".to_string());
    }
    Ok(())
}
