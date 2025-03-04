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

    let temp_dir = home.join(format!(".cache/nephelios/.{}-tmp", app_name));

    fs::create_dir_all(&temp_dir).map_err(|e| format!("Failed to create temp directory: {}", e))?;

    Ok(temp_dir)
}

/// Modifies the GitHub URL to include the specified username.
///
/// # Arguments
///
/// * `github_url` - The original GitHub URL to be modified.
///
/// # Returns
/// * A modified GitHub URL with the username prefixed.
pub fn modify_github_url(github_url: &str) -> String {
    let prefix = "https://damien-mathieu1@github.com/";
    // Remove the existing "https://github.com/" prefix if present
    if let Some(pos) = github_url.find("https://github.com/") {
        let modified_url = format!(
            "{}{}",
            prefix,
            &github_url[pos + "https://github.com/".len()..]
        );
        return modified_url;
    }
    github_url.to_string()
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
    let github_url = modify_github_url(github_url);

    let status = Command::new("git")
        .args(["clone", &github_url, target_dir])
        .status()
        .map_err(|e| format!("Failed to execute git: {}", e))?;

    if !status.success() {
        return Err("Failed to clone repository. Check URL and permissions.".to_string());
    }
    Ok(())
}
