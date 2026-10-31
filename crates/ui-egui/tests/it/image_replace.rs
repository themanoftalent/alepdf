use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use pdfcraft_ui_egui::{PdfCraftApp, SaveTarget};

// Two pages share both content and resources; each draws the same image twice.
fn shared_picture(form: bool) -> Vec<u8> {
    let content = b"q 80 0 0 40 20 160 cm /Im0 Do Q q 80 0 0 40 180 160 cm /Im0 Do Q BT /F1 12 Tf 20 30 Td (Caption) Tj ET";
    let stream = |dict: &str, bytes: &[u8]| {
        let mut out = format!("<< {dict} /Length {} >>\nstream\n", bytes.len()).into_bytes();
        out.extend_from_slice(bytes);
        out.extend_from_slice(b"\nendstream");
        out
    };
    let picture = if form {
        stream("/Type /XObject /Subtype /Form /BBox [0 0 1 1]", b"1 0 0 rg 0 0 1 1 re f")
    } else {
        stream("/Type /XObject /Subtype /Image /Width 2 /Height 1 /ColorSpace /DeviceRGB /BitsPerComponent 8", &[200, 40, 40, 200, 40, 40])
    };
    let objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 /MediaBox [0 0 300 240] >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /Contents 5 0 R /Resources 6 0 R >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /Contents 5 0 R /Resources 6 0 R >>".to_vec(),
        stream("", content),
        b"<< /XObject << /Im0 7 0 R >> /Font << /F1 8 0 R >> >>".to_vec(),
        picture,
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
    ];
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(object);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(b"xref\n0 9\n0000000000 65535 f \n");
    for offset in offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(format!("trailer\n<< /Size 9 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    out
}

fn selected_picture(language: &'static str, form: bool) -> Harness<'static, PdfCraftApp> {
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_eframe(move |_| {
        let mut app = PdfCraftApp::new();
        app.set_option("language", language).unwrap();
        app.open_bytes("shared.pdf", None, shared_picture(form)).unwrap();
        app.execute("edit.edit_text");
        app
    });
    h.run_steps(4);
    let r = h.state().views[0].page_screen_rect(0).unwrap();
    let point = egui::pos2(r.left() + r.width() * 0.2, r.top() + r.height() * 0.25);
    h.hover_at(point);
    h.run_steps(1);
    h.drag_at(point);
    h.run_steps(1);
    h.drop_at(point);
    h.run_steps(3);
    assert_eq!(h.state().views[0].image_selection.as_ref().map(|s| (s.page, s.index)), Some((0, 0)));
    h
}

fn replace_row(h: &mut Harness<'static, PdfCraftApp>, label: &str) {
    h.get_all_by_label(label).find(|n| n.rect().width() > 120.0).expect("a visible, labeled Replace Image row").click();
    h.run_steps(3);
}

#[test]
fn image_replace_labeled_action_png_jpeg_undo_save_and_shared_resources() {
    for (format, ext, language, label) in
        [(image::ImageFormat::Png, "png", "en", "Replace Image…"), (image::ImageFormat::Jpeg, "jpg", "tr", "Resmi Değiştir…")]
    {
        let dir = std::env::temp_dir().join(format!("alepdf-replace-{ext}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let replacement = dir.join(format!("replacement.{ext}"));
        image::RgbImage::from_pixel(5, 7, image::Rgb([20, 180, 40])).save_with_format(&replacement, format).unwrap();
        let mut h = selected_picture(language, false);
        let id = h.state().views[0].id;
        let original = h.state().session.get(id).unwrap().page_image_file(0, 0).unwrap();
        let bounds = h.state().session.get(id).unwrap().page_images(0)[0].rect;
        h.state_mut().pick_override = Some(vec![replacement.to_string_lossy().into_owned()]);
        replace_row(&mut h, label);
        let check = |app: &PdfCraftApp, width| {
            let doc = app.session.get(app.views[0].id).unwrap();
            let images = doc.page_images(0);
            assert_eq!(images[0].width, width);
            assert_eq!(images[0].rect, bounds, "replacement retains the original placement and aspect box");
            assert_eq!(doc.page_image_file(0, 1).unwrap(), original, "same-page shared instance is untouched");
            assert_eq!(doc.page_image_file(1, 0).unwrap(), original, "other page sharing content/resources is untouched");
            assert_eq!(doc.page_images(1).len(), 2);
            assert_eq!(doc.text_lines(0)[0].text, "Caption");
        };
        check(h.state(), 5);
        h.state_mut().undo();
        h.run_steps(2);
        check(h.state(), 2);
        h.state_mut().redo();
        h.run_steps(2);
        check(h.state(), 5);
        let saved = dir.join("saved-copy.pdf");
        h.state_mut().save_override = Some(saved.to_string_lossy().into_owned());
        assert!(h.state_mut().save_active(SaveTarget::As));
        let mut reopened = PdfCraftApp::new();
        reopened.open_bytes("saved-copy.pdf", None, std::fs::read(saved).unwrap()).unwrap();
        check(&reopened, 5);
        let _ = std::fs::remove_dir_all(dir);
    }
}

#[test]
fn image_replace_cancel_and_invalid_file_preserve_document() {
    let dir = std::env::temp_dir().join(format!("alepdf-replace-invalid-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let invalid = dir.join("broken.png");
    std::fs::write(&invalid, b"not an image").unwrap();
    for picks in [Vec::new(), vec![invalid.to_string_lossy().into_owned()]] {
        let mut h = selected_picture("en", false);
        let id = h.state().views[0].id;
        let before = h.state().session.get(id).unwrap().page_image_file(0, 0).unwrap();
        h.state_mut().pick_override = Some(picks);
        replace_row(&mut h, "Replace Image…");
        let doc = h.state().session.get(id).unwrap();
        assert_eq!(doc.page_image_file(0, 0).unwrap(), before);
        assert!(!doc.dirty, "cancel and invalid input must not create edits");
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn image_replace_grouped_vector_artwork_is_explained_and_disabled() {
    let h = selected_picture("en", true);
    let row = h.get_all_by_label("Replace Image…").find(|n| n.rect().width() > 120.0).expect("visible replacement row");
    assert!(row.accesskit_node().is_disabled());
    assert!(h.query_by_label("Grouped/vector artwork cannot be replaced with a bitmap.").is_some());
}
