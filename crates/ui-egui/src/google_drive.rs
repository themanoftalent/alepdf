//! Google Drive integration: detect macOS Google Drive FileProvider mounts,
//! save documents to Google Drive, and sync automatically with cloud storage.

use std::path::{Path, PathBuf};
use crate::PdfCraftApp;
use crate::editing::SaveTarget;

/// Detect the user's active Google Drive folder on macOS (via FileProvider or standard mounts).
pub fn detect_google_drive_root() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from)?;

    // 1. macOS FileProvider CloudStorage: ~/Library/CloudStorage/GoogleDrive-<email>/My Drive
    let cloud_storage = home.join("Library/CloudStorage");
    if cloud_storage.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&cloud_storage) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                // Matches standard ASCII hyphen or non-breaking hyphen
                if name_str.starts_with("GoogleDrive-") || name_str.starts_with("GoogleDrive‑") {
                    let my_drive = entry.path().join("My Drive");
                    if my_drive.is_dir() {
                        return Some(my_drive);
                    }
                    if entry.path().is_dir() {
                        return Some(entry.path());
                    }
                }
            }
        }
    }

    // 2. Legacy / alternate Google Drive locations
    for candidate in [
        home.join("Google Drive/My Drive"),
        home.join("Google Drive"),
        PathBuf::from("/Volumes/GoogleDrive/My Drive"),
        PathBuf::from("/Volumes/GoogleDrive"),
    ] {
        if candidate.is_dir() {
            return Some(candidate);
        }
    }

    None
}

/// Returns the dedicated "AlePdf" folder inside Google Drive, creating it if needed.
pub fn get_or_create_alepdf_drive_folder() -> Result<PathBuf, String> {
    let root = detect_google_drive_root()
        .ok_or_else(|| "Google Drive folder was not found in ~/Library/CloudStorage. Please make sure Google Drive is running.".to_string())?;

    let alepdf_dir = root.join("AlePdf");
    if !alepdf_dir.exists() {
        if let Err(e) = std::fs::create_dir_all(&alepdf_dir) {
            return Err(format!("Could not create AlePdf folder in Google Drive: {e}"));
        }
    }
    Ok(alepdf_dir)
}

/// Checks if a file path is located inside Google Drive.
pub fn is_path_in_google_drive(path: &Path) -> bool {
    if let Some(root) = detect_google_drive_root() {
        if path.starts_with(&root) {
            return true;
        }
    }
    let s = path.to_string_lossy();
    s.contains("CloudStorage/GoogleDrive") || s.contains("/Google Drive/")
}

impl PdfCraftApp {
    /// Returns true if the currently active document is stored inside Google Drive.
    pub fn is_active_in_google_drive(&self) -> bool {
        let Some(active) = self.active else { return false };
        let Some(view) = self.views.get(active) else { return false };
        let Some(doc) = self.session.get(view.id) else { return false };
        doc.path.as_deref().map(Path::new).is_some_and(is_path_in_google_drive)
    }

    /// Primary action: Save the document to Google Drive if not yet there, or sync immediately if it is.
    pub fn save_or_sync_to_google_drive(&mut self) {
        let Some(active) = self.active else {
            self.notify_tr("No active document to save to Google Drive");
            return;
        };
        let Some(view) = self.views.get(active) else { return };
        let Some(doc) = self.session.get(view.id) else { return };
        let id = view.id;
        let doc_name = doc.name.clone();
        let doc_path = doc.path.clone();

        // If the document is ALREADY in Google Drive, perform an in-place save
        // which immediately triggers macOS FileProvider to sync the file to Google Drive cloud.
        if doc_path.as_deref().map(Path::new).is_some_and(is_path_in_google_drive) {
            if self.save_active(SaveTarget::InPlace) {
                self.notify(format!("☁ Synced with Google Drive: {doc_name}"));
            }
            return;
        }

        // If not yet in Google Drive, save a copy to the AlePdf Google Drive folder:
        match get_or_create_alepdf_drive_folder() {
            Ok(folder) => {
                let filename = if doc_name.to_ascii_lowercase().ends_with(".pdf") {
                    doc_name.clone()
                } else {
                    format!("{doc_name}.pdf")
                };
                let dest_path = folder.join(&filename);
                let dest_str = dest_path.to_string_lossy().to_string();

                #[cfg(not(target_arch = "wasm32"))]
                {
                    if self.save_doc_to(id, &dest_str, SaveTarget::As) {
                        self.notify(format!("☁ Saved to Google Drive (AlePdf/{filename}) — Cloud sync is active"));
                    }
                }
                #[cfg(target_arch = "wasm32")]
                {
                    self.notify_tr("Google Drive sync requires the desktop app");
                }
            }
            Err(err) => {
                self.notify_error(err);
            }
        }
    }

    /// Reveal the current document or AlePdf folder in macOS Finder.
    pub fn reveal_active_in_google_drive(&mut self) {
        let path_to_open = if let Some(active) = self.active
            && let Some(view) = self.views.get(active)
            && let Some(doc) = self.session.get(view.id)
            && let Some(p) = &doc.path
            && is_path_in_google_drive(Path::new(p))
        {
            PathBuf::from(p)
        } else if let Some(folder) = detect_google_drive_root() {
            folder
        } else {
            PathBuf::from("~/Library/CloudStorage")
        };

        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open")
                .arg("-R")
                .arg(&path_to_open)
                .spawn();
        }
    }

    /// Open the Google Drive folder in macOS Finder.
    pub fn open_google_drive_folder(&mut self) {
        if let Some(folder) = detect_google_drive_root() {
            #[cfg(target_os = "macos")]
            {
                let _ = std::process::Command::new("open")
                    .arg(&folder)
                    .spawn();
            }
        } else {
            self.notify_tr("Google Drive folder not found");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_google_drive_detection() {
        let detected = detect_google_drive_root();
        if let Some(path) = detected {
            assert!(path.is_dir() || path.to_string_lossy().contains("GoogleDrive"));
            assert!(is_path_in_google_drive(&path.join("test.pdf")));
        }
    }

    #[test]
    fn test_is_path_in_google_drive_patterns() {
        assert!(is_path_in_google_drive(Path::new("/Users/me/Library/CloudStorage/GoogleDrive-user@gmail.com/My Drive/doc.pdf")));
        assert!(is_path_in_google_drive(Path::new("/Users/me/Google Drive/My Drive/doc.pdf")));
        assert!(!is_path_in_google_drive(Path::new("/Users/me/Documents/doc.pdf")));
    }
}
