//! Google Drive integration: detect macOS Google Drive FileProvider mounts,
//! save locally to its folder; Google Drive handles uploading independently.

use crate::PdfCraftApp;
use crate::editing::SaveTarget;
use std::path::{Path, PathBuf};

/// Detect the user's active Google Drive folder on macOS (via FileProvider or standard mounts).
pub fn detect_google_drive_root() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from)?;

    // 1. macOS FileProvider CloudStorage: ~/Library/CloudStorage/GoogleDrive-<email>/My Drive
    let cloud_storage = home.join("Library/CloudStorage");
    if cloud_storage.is_dir()
        && let Ok(entries) = std::fs::read_dir(&cloud_storage)
    {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            // Matches standard ASCII hyphen or non-breaking hyphen.
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

    // 2. Legacy / alternate Google Drive locations
    [
        home.join("Google Drive/My Drive"),
        home.join("Google Drive"),
        PathBuf::from("/Volumes/GoogleDrive/My Drive"),
        PathBuf::from("/Volumes/GoogleDrive"),
    ]
    .into_iter()
    .find(|candidate| candidate.is_dir())
}

/// Returns the dedicated "AlePdf" folder inside Google Drive, creating it if needed.
pub fn get_or_create_alepdf_drive_folder() -> Result<PathBuf, String> {
    let root = detect_google_drive_root()
        .ok_or_else(|| "Google Drive folder was not found in ~/Library/CloudStorage. Please make sure Google Drive is running.".to_string())?;

    let alepdf_dir = root.join("AlePdf");
    if !alepdf_dir.exists()
        && let Err(e) = std::fs::create_dir_all(&alepdf_dir)
    {
        return Err(format!("Could not create AlePdf folder in Google Drive: {e}"));
    }
    Ok(alepdf_dir)
}

/// Checks if a file path is located inside Google Drive.
pub fn is_path_in_google_drive(path: &Path) -> bool {
    if let Some(root) = detect_google_drive_root()
        && path.starts_with(&root)
    {
        return true;
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

    /// Save locally. An existing destination belonging to another document needs confirmation.
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
        // Google Drive handles uploading; a local save cannot confirm remote completion.
        if doc_path.as_deref().map(Path::new).is_some_and(is_path_in_google_drive) {
            if self.save_active(SaveTarget::InPlace) {
                self.notify(format!("Saved {doc_name} to your Google Drive folder. Google Drive handles uploading."));
            }
            return;
        }

        // If not yet in Google Drive, save a copy to the AlePdf Google Drive folder:
        match get_or_create_alepdf_drive_folder() {
            Ok(folder) => {
                let filename = if doc_name.to_ascii_lowercase().ends_with(".pdf") { doc_name.clone() } else { format!("{doc_name}.pdf") };
                let Some(filename) = Path::new(&filename).file_name() else {
                    self.notify_error("Invalid PDF filename");
                    return;
                };
                let dest_path = folder.join(filename);
                self.save_drive_copy(id, dest_path);
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
            let _ = std::process::Command::new("open").arg("-R").arg(&path_to_open).spawn();
        }
    }

    /// Open the Google Drive folder in macOS Finder.
    pub fn open_google_drive_folder(&mut self) {
        if let Some(folder) = detect_google_drive_root() {
            #[cfg(target_os = "macos")]
            {
                let _ = std::process::Command::new("open").arg(&folder).spawn();
            }
        } else {
            self.notify_tr("Google Drive folder not found");
        }
    }
}

/// A pending copy is bound to its original document even if tabs change.
#[derive(Clone)]
pub(crate) struct DriveCollision {
    id: pdfcraft_engine::DocId,
    destination: PathBuf,
}

#[derive(Clone, Copy)]
enum CollisionChoice {
    Replace,
    KeepBoth,
    Cancel,
}

/// Publish a complete staged file without replacing any existing destination, including a
/// dangling symlink. macOS uses exclusive rename so FileProvider volumes need not support
/// hard links. Other desktops use an exclusive hard link; unsupported filesystems fail safely.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn write_new_copy(destination: &str, bytes: &[u8]) -> std::io::Result<()> {
    use pdfcraft_platform::staging::{StagingName, create_staging, staging_suffixes};
    use std::io::Write;
    let path = Path::new(destination);
    let folder = path.parent().ok_or_else(|| std::io::Error::other("Missing destination folder"))?;
    let (staged, mut file) = create_staging(folder, "alepdf-copy", StagingName::TagThenSuffix, staging_suffixes())?;
    let result = (|| {
        file.write_all(bytes)?;
        #[cfg(target_os = "macos")]
        {
            use rustix::{
                fs::{CWD, RenameFlags, renameat_with},
                io::Errno,
            };
            // Match the ordinary save's fallback on volumes without full hardware flush.
            match file.sync_all() {
                Err(e) if matches!(Errno::from_io_error(&e), Some(Errno::NOTSUP | Errno::NOTTY)) => rustix::fs::fsync(&file)?,
                result => result?,
            }
            Ok(renameat_with(CWD, &staged, CWD, path, RenameFlags::NOREPLACE)?)
        }
        #[cfg(not(target_os = "macos"))]
        {
            file.sync_all()?;
            std::fs::hard_link(&staged, path)
        }
    })();
    drop(file);
    let _ = std::fs::remove_file(staged);
    result
}

fn unused_copy_path(path: &Path) -> std::io::Result<PathBuf> {
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    for number in 1..=10_000 {
        let candidate = path.with_file_name(format!("{stem} ({number}).pdf"));
        match std::fs::symlink_metadata(&candidate) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(candidate),
            Err(e) => return Err(e),
            Ok(_) => {}
        }
    }
    Err(std::io::Error::other("No unused copy name found"))
}

impl PdfCraftApp {
    pub(crate) fn save_drive_copy(&mut self, id: pdfcraft_engine::DocId, destination: PathBuf) {
        match std::fs::symlink_metadata(&destination) {
            Ok(_) => self.drive_collision = Some(DriveCollision { id, destination }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => self.finish_drive_copy(id, &destination, false),
            Err(e) => self.notify_error(format!("Could not check destination: {e}")),
        }
    }

    fn finish_drive_copy(&mut self, id: pdfcraft_engine::DocId, path: &Path, replace: bool) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let dest = path.to_string_lossy();
            let saved = if replace { self.save_doc_to(id, &dest, SaveTarget::As) } else { self.save_doc_to_new(id, &dest) };
            if saved {
                self.notify("Saved to your Google Drive folder. Google Drive handles uploading.");
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (id, path, replace);
            self.notify_tr("Saving to a Google Drive folder requires the desktop app");
        }
    }

    fn resolve_drive_collision(&mut self, choice: CollisionChoice) {
        let Some(pending) = self.drive_collision.take() else { return };
        match choice {
            CollisionChoice::Cancel => {}
            CollisionChoice::Replace => self.finish_drive_copy(pending.id, &pending.destination, true),
            CollisionChoice::KeepBoth => match unused_copy_path(&pending.destination) {
                Ok(path) => self.finish_drive_copy(pending.id, &path, false),
                Err(e) => self.notify_error(format!("Could not keep both files: {e}")),
            },
        }
    }

    pub(crate) fn show_drive_collision(&mut self, ctx: &egui::Context) {
        let Some(pending) = &self.drive_collision else { return };
        let mut choice = None;
        let modal = crate::widgets::modal(ctx, "drive-collision").show(ctx, |ui| {
            ui.set_max_width(440.0);
            ui.heading("A file with this name already exists");
            ui.label(pending.destination.display().to_string());
            ui.label("Replace the existing file, or keep both with a numbered filename.");
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    choice = Some(CollisionChoice::Cancel);
                }
                if ui.button("Keep both").clicked() {
                    choice = Some(CollisionChoice::KeepBoth);
                }
                if ui.button("Replace").clicked() {
                    choice = Some(CollisionChoice::Replace);
                }
            });
        });
        if modal.should_close() && choice.is_none() {
            choice = Some(CollisionChoice::Cancel);
        }
        if let Some(choice) = choice {
            self.resolve_drive_collision(choice);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PDF: &[u8] = b"%PDF-1.7\n1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] >> endobj\ntrailer << /Root 1 0 R >>\n%%EOF";

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let suffix = pdfcraft_platform::staging::staging_suffixes().next().unwrap();
            let path = std::env::temp_dir().join(format!("alepdf-drive-test-{suffix}"));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn collision_cancel_keep_both_replace_and_repeated_save() {
        let dir = Fixture::new();
        let folder = dir.0.join("Google Drive");
        std::fs::create_dir(&folder).unwrap();
        let dest = folder.join("one.pdf");
        std::fs::write(&dest, b"existing document").unwrap();
        std::fs::write(folder.join("one (1).pdf"), b"earlier copy").unwrap();
        let mut app = PdfCraftApp::new();
        app.open_bytes("one.pdf", None, PDF.to_vec()).unwrap();
        let id = app.views[app.active.unwrap()].id;
        app.save_drive_copy(id, dest.clone());
        assert!(app.drive_collision.is_some());
        app.resolve_drive_collision(CollisionChoice::Cancel);
        assert_eq!(std::fs::read(&dest).unwrap(), b"existing document");
        assert!(app.session.get(id).unwrap().path.is_none());
        assert!(!folder.join("one (2).pdf").exists());

        app.save_drive_copy(id, dest.clone());
        app.resolve_drive_collision(CollisionChoice::KeepBoth);
        let copy = folder.join("one (2).pdf");
        assert!(std::fs::read(&copy).unwrap().starts_with(b"%PDF-"));
        assert_eq!(std::fs::read(&dest).unwrap(), b"existing document");
        assert_eq!(std::fs::read(folder.join("one (1).pdf")).unwrap(), b"earlier copy");
        app.save_or_sync_to_google_drive();
        assert!(app.drive_collision.is_none());
        assert!(!folder.join("one (3).pdf").exists());
        assert_eq!(app.session.get(id).unwrap().path.as_deref(), copy.to_str());

        app.save_drive_copy(id, dest.clone());
        app.resolve_drive_collision(CollisionChoice::Replace);
        assert!(std::fs::read(&dest).unwrap().starts_with(b"%PDF-"));
        assert_eq!(app.session.get(id).unwrap().path.as_deref(), dest.to_str());
    }

    #[test]
    fn collision_dialog_cancel_does_not_write() {
        use egui_kittest::kittest::Queryable;
        let dir = Fixture::new();
        let dest = dir.0.join("one.pdf");
        std::fs::write(&dest, b"original").unwrap();
        let target = dest.clone();
        let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1200.0, 900.0)).build_eframe(move |_cc| {
            let mut app = PdfCraftApp::new();
            app.set_option("language", "en").unwrap();
            app.open_bytes("one.pdf", None, PDF.to_vec()).unwrap();
            let id = app.views[app.active.unwrap()].id;
            app.save_drive_copy(id, target);
            app
        });
        h.run_steps(4);
        assert!(h.query_by_label("Replace").is_some());
        assert!(h.query_by_label("Keep both").is_some());
        h.get_by_label("Cancel").click();
        h.run_steps(2);
        assert!(h.state().drive_collision.is_none());
        assert_eq!(std::fs::read(&dest).unwrap(), b"original");
        let id = h.state().views[h.state().active.unwrap()].id;
        h.state_mut().save_drive_copy(id, dest.clone());
        h.run_steps(3);
        h.key_press(egui::Key::Escape);
        h.run_steps(2);
        assert!(h.state().drive_collision.is_none());
        assert_eq!(std::fs::read(dest).unwrap(), b"original");
    }

    #[test]
    fn exclusive_copy_never_overwrites_and_cleans_staging_files() {
        let dir = Fixture::new();
        let dest = dir.0.join("one.pdf");
        write_new_copy(dest.to_str().unwrap(), b"first").unwrap();
        assert_eq!(write_new_copy(dest.to_str().unwrap(), b"second").unwrap_err().kind(), std::io::ErrorKind::AlreadyExists);
        assert_eq!(std::fs::read(&dest).unwrap(), b"first");
        assert_eq!(std::fs::read_dir(&dir.0).unwrap().count(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn dangling_symlink_is_a_collision() {
        let dir = Fixture::new();
        let dest = dir.0.join("one.pdf");
        std::os::unix::fs::symlink(dir.0.join("missing"), &dest).unwrap();
        assert!(write_new_copy(dest.to_str().unwrap(), b"new").is_err());
        let mut app = PdfCraftApp::new();
        app.open_bytes("one.pdf", None, PDF.to_vec()).unwrap();
        let id = app.views[app.active.unwrap()].id;
        app.save_drive_copy(id, dest.clone());
        assert!(app.drive_collision.is_some());
        app.resolve_drive_collision(CollisionChoice::Cancel);
        assert!(std::fs::symlink_metadata(dest).unwrap().file_type().is_symlink());
    }

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
