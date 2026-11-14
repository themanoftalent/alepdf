//! The right-click menu of every text field: Cut, Copy, Paste and Select All.
//!
//! egui's text field has no menu of its own, so a field answered only the keyboard and the Edit
//! menu. This adds the menu in one place, for every field the app draws: [`show`] runs at the end
//! of each frame, sees a right-press on the field that has the keyboard, and opens the menu at the
//! pointer. A choice is not carried out here: it becomes the input its key would have been (a
//! Copy event, ⌘A, a paste request to the system), queued for the next frame, so the field
//! handles it exactly as it handles the key (a password field still copies nothing).
//!
//! Two things egui does on the way are undone:
//! - any press in a field moves the caret there and drops the selection, so the right-press
//!   would leave nothing to cut or copy. The selection as of the frame before is put back.
//! - pressing a menu item takes the keyboard from the field, which drops the selection again.
//!   The field gets the keyboard and its selection back before the queued input arrives.

use egui::text::CCursorRange;
use egui::text_edit::TextEditState;
use pdfcraft_engine::commands::Shortcut;

/// The field a menu is open for.
#[derive(Clone, Copy)]
struct Target {
    field: egui::Id,
    /// The layer the field is drawn on: the menu belongs to it, so it works above a dialog.
    layer: egui::LayerId,
    /// What the menu acts on: the field's selection (or caret) when the menu opened.
    range: Option<CCursorRange>,
}

#[derive(Clone, Copy, Default)]
struct State {
    /// The text field that had the keyboard at the end of the last frame, and its selection.
    seen: Option<(egui::Id, Option<CCursorRange>)>,
    target: Option<Target>,
}

#[derive(Clone, Copy)]
enum Choice {
    Cut,
    Copy,
    Paste,
    SelectAll,
}

fn state_id() -> egui::Id {
    egui::Id::unique("text-field-menu-state")
}

fn menu_id() -> egui::Id {
    egui::Id::unique("text-field-menu")
}

fn selection(ctx: &egui::Context, field: egui::Id) -> Option<CCursorRange> {
    TextEditState::load(ctx, field).and_then(|state| state.cursor.char_range())
}

fn select(ctx: &egui::Context, field: egui::Id, range: CCursorRange) {
    if let Some(mut state) = TextEditState::load(ctx, field) {
        state.cursor.set_char_range(Some(range));
        state.store(ctx, field);
    }
}

/// Opens, draws and answers the text-field menu. Call once per frame, after every text field is
/// drawn. `queue` is input for the start of the next frame (`PdfCraftApp::menu_input`).
pub(crate) fn show(ctx: &egui::Context, queue: &mut Vec<egui::Event>) {
    let mut state: State = ctx.data(|d| d.get_temp(state_id())).unwrap_or_default();
    // A field takes the keyboard on any press, so the field under a right-press has it by now.
    let focused = ctx.memory(|m| m.focused()).filter(|&id| TextEditState::load(ctx, id).is_some());
    let (pressed, released) = ctx.input(|i| {
        let secondary = egui::PointerButton::Secondary;
        (i.pointer.button_pressed(secondary).then(|| i.pointer.interact_pos()).flatten(), i.pointer.button_released(secondary))
    });

    let mut open = None;
    if let (Some(at), Some(field)) = (pressed, focused)
        && let Some(response) = ctx.read_response(field)
        // A read-only field takes no input, so it has nothing for the menu to do.
        && response.enabled()
        && response.sense.senses_click()
        && response.rect.contains(at)
    {
        let before = state.seen.filter(|(seen, _)| *seen == field).and_then(|(_, range)| range).filter(|range| !range.is_empty());
        if let Some(range) = before {
            select(ctx, field, range);
        }
        state.target = Some(Target { field, layer: response.layer_id, range: before.or_else(|| selection(ctx, field)) });
        open = Some(egui::SetOpenCommand::Bool(true));
    }

    if let Some(target) = state.target {
        // Moving the mouse while the button is down drags the selection; the menu's stays.
        if released && let Some(range) = target.range {
            select(ctx, target.field, range);
        }
        let mut choice = None;
        let shown = egui::Popup::new(menu_id(), ctx.clone(), egui::PopupAnchor::PointerFixed, target.layer)
            .kind(egui::PopupKind::Menu)
            .layout(egui::Layout::top_down_justified(egui::Align::Min))
            .style(egui::containers::menu::menu_style)
            .gap(0.0)
            .open_memory(open)
            .show(|ui| {
                let selected = target.range.is_some_and(|range| !range.is_empty());
                let mut item = |ui: &mut egui::Ui, label: &str, shortcut: Shortcut, enabled: bool, pick: Choice| {
                    let button = egui::Button::new(label).shortcut_text(crate::commands::shortcut_label(ui.ctx(), shortcut));
                    if ui.add_enabled(enabled, button).clicked() {
                        choice = Some(pick);
                    }
                };
                item(ui, tl!("Cut"), Shortcut::cmd("X"), selected, Choice::Cut);
                item(ui, tl!("Copy"), crate::commands::COPY, selected, Choice::Copy);
                item(ui, tl!("Paste"), Shortcut::cmd("V"), true, Choice::Paste);
                ui.separator();
                item(ui, tl!("Select All"), crate::commands::SELECT_ALL, true, Choice::SelectAll);
            });
        if let Some(choice) = choice {
            if let Some(range) = target.range {
                select(ctx, target.field, range);
            }
            ctx.memory_mut(|m| m.request_focus(target.field));
            match choice {
                Choice::Cut => queue.push(egui::Event::Cut),
                Choice::Copy => queue.push(egui::Event::Copy),
                // The system answers with a paste event, which the field receives as it does ⌘V's.
                Choice::Paste => ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste),
                Choice::SelectAll => queue.extend([true, false].map(|pressed| egui::Event::Key {
                    key: egui::Key::A,
                    physical_key: None,
                    pressed,
                    repeat: false,
                    modifiers: egui::Modifiers::COMMAND,
                })),
            }
            egui::Popup::close_id(ctx, menu_id());
            state.target = None;
            ctx.request_repaint();
        } else if shown.is_none() {
            // Closed by a click elsewhere, Escape, or another menu opening.
            state.target = None;
        }
    }

    // After the menu's own changes, so a selection it restored is what the next frame compares.
    state.seen = focused.map(|field| (field, selection(ctx, field)));
    ctx.data_mut(|d| d.insert_temp(state_id(), state));
}

#[cfg(test)]
mod tests {
    use egui_kittest::kittest::{NodeT as _, Queryable as _};

    /// Two text fields and the menu, with the queue delivered the way `PdfCraftApp::logic` does.
    #[derive(Default)]
    struct Form {
        name: String,
        notes: String,
        read_only: String,
        queue: Vec<egui::Event>,
        /// What the frame asked the system for: copied text and paste requests.
        copied: Vec<String>,
        paste_requests: usize,
        modal: bool,
    }

    fn fields(ui: &mut egui::Ui, form: &mut Form) {
        let label = ui.label("Name:").id;
        ui.add(egui::TextEdit::singleline(&mut form.name)).labelled_by(label);
        let label = ui.label("Notes:").id;
        ui.add(egui::TextEdit::multiline(&mut form.notes)).labelled_by(label);
        let label = ui.label("Fixed:").id;
        ui.add(egui::TextEdit::singleline(&mut form.read_only).interactive(false)).labelled_by(label);
        ui.label("Outside");
    }

    fn harness(form: Form) -> egui_kittest::Harness<'static, Form> {
        let mut h = egui_kittest::Harness::new_ui_state(
            |ui, form: &mut Form| {
                let ctx = ui.ctx().clone();
                let events = std::mem::take(&mut form.queue);
                ctx.input_mut(|i| i.events.extend(events));
                if form.modal {
                    egui::Modal::new(egui::Id::unique("dialog")).show(&ctx, |ui| fields(ui, form));
                } else {
                    fields(ui, form);
                }
                super::show(&ctx, &mut form.queue);
                ctx.output(|o| {
                    for command in &o.commands {
                        if let egui::OutputCommand::CopyText(text) = command {
                            form.copied.push(text.clone());
                        }
                    }
                });
                form.paste_requests += ctx.viewport(|v| v.commands.iter().filter(|c| matches!(c, egui::ViewportCommand::RequestPaste)).count());
            },
            form,
        );
        h.run();
        h
    }

    /// The text field whose text is `value`.
    fn field<'a>(h: &'a egui_kittest::Harness<'static, Form>, value: &'a str) -> egui_kittest::Node<'a> {
        use egui::accesskit::Role;
        h.get_by(|node| matches!(node.role(), Role::TextInput | Role::MultilineTextInput) && node.value().as_deref() == Some(value))
    }

    /// A menu item by its name; its accessible label continues with the shortcut.
    fn item<'a>(h: &'a egui_kittest::Harness<'static, Form>, name: &'a str) -> egui_kittest::Node<'a> {
        h.get_by(move |node| node.label().is_some_and(|label| label.starts_with(name)))
    }

    fn menu_open(h: &egui_kittest::Harness<'static, Form>) -> bool {
        h.query_all_by_label_contains("Paste").next().is_some()
    }

    fn key(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
        egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }
    }

    /// Focus the field and select everything in it with the keyboard.
    fn select_all(h: &mut egui_kittest::Harness<'static, Form>, value: &str) {
        field(h, value).click();
        h.run();
        h.event(key(egui::Key::A, egui::Modifiers::COMMAND));
        h.run();
    }

    fn choose(h: &mut egui_kittest::Harness<'static, Form>, value: &str, item: &str) {
        field(h, value).click_secondary();
        h.run();
        self::item(h, item).click();
        h.run();
    }

    const TEXT: &str = "Çığ öğle şükür İSTANBUL ∑ x² ≤ ∞";

    #[test]
    fn copy_puts_the_selection_a_right_press_would_have_dropped_on_the_clipboard() {
        let mut h = harness(Form { name: TEXT.into(), ..Default::default() });
        select_all(&mut h, TEXT);
        choose(&mut h, TEXT, "Copy");
        assert_eq!(h.state().copied, [TEXT], "Turkish letters and mathematical symbols arrive unchanged");
        assert_eq!(h.state().name, TEXT, "copying changes nothing");
        assert!(!menu_open(&h), "the menu closed");
    }

    #[test]
    fn cut_copies_and_removes_the_selection() {
        let mut h = harness(Form { name: TEXT.into(), ..Default::default() });
        select_all(&mut h, TEXT);
        choose(&mut h, TEXT, "Cut");
        assert_eq!(h.state().copied, [TEXT]);
        assert_eq!(h.state().name, "");
    }

    #[test]
    fn multiline_text_is_copied_with_its_line_breaks() {
        let notes = "birinci satır\nikinci satır\n\tüçüncü";
        let mut h = harness(Form { notes: notes.into(), ..Default::default() });
        select_all(&mut h, notes);
        choose(&mut h, notes, "Copy");
        assert_eq!(h.state().copied, [notes]);
    }

    #[test]
    fn without_a_selection_cut_and_copy_are_disabled() {
        let mut h = harness(Form { name: "abc".into(), ..Default::default() });
        field(&h, "abc").click_secondary();
        h.run();
        for (name, enabled) in [("Cut", false), ("Copy", false), ("Paste", true), ("Select All", true)] {
            assert_eq!(!item(&h, name).accesskit_node().is_disabled(), enabled, "{name}");
        }
    }

    #[test]
    fn select_all_then_copy_work_on_a_field_that_was_not_focused() {
        let mut h = harness(Form { name: "abc".into(), notes: "first\nsecond".into(), ..Default::default() });
        choose(&mut h, "first\nsecond", "Select All");
        h.run();
        choose(&mut h, "first\nsecond", "Copy");
        assert_eq!(h.state().copied, ["first\nsecond"], "the right-clicked field is the one the menu acts on");
    }

    #[test]
    fn paste_asks_the_system_and_the_answer_replaces_the_selection() {
        let mut h = harness(Form { name: "old".into(), ..Default::default() });
        select_all(&mut h, "old");
        choose(&mut h, "old", "Paste");
        assert_eq!(h.state().paste_requests, 1);
        // What the windowing integration sends back for the request.
        h.event(egui::Event::Paste("yapıştırılan\nmetin".into()));
        h.run();
        assert_eq!(h.state().name, "yapıştırılan metin", "a single-line field takes pasted line breaks as spaces, as with ⌘V");
    }

    #[test]
    fn pasted_multiline_text_keeps_its_line_breaks_in_a_multiline_field() {
        let mut h = harness(Form { notes: "x".into(), ..Default::default() });
        select_all(&mut h, "x");
        choose(&mut h, "x", "Paste");
        h.event(egui::Event::Paste("bir\niki\nüç".into()));
        h.run();
        assert_eq!(h.state().notes, "bir\niki\nüç");
    }

    #[test]
    fn a_read_only_field_and_plain_labels_get_no_menu() {
        let mut h = harness(Form { read_only: "fixed".into(), ..Default::default() });
        field(&h, "fixed").click_secondary();
        h.run();
        assert!(!menu_open(&h));
        h.get_by_label("Outside").click_secondary();
        h.run();
        assert!(!menu_open(&h));
    }

    #[test]
    fn a_click_elsewhere_closes_the_menu_and_does_nothing() {
        let mut h = harness(Form { name: "abc".into(), ..Default::default() });
        select_all(&mut h, "abc");
        field(&h, "abc").click_secondary();
        h.run();
        assert!(menu_open(&h));
        h.get_by_label("Outside").click();
        h.run();
        assert!(!menu_open(&h));
        assert!(h.state().copied.is_empty() && h.state().queue.is_empty() && h.state().name == "abc");
    }

    #[test]
    fn the_menu_works_on_a_field_inside_a_modal_dialog() {
        let mut h = harness(Form { name: "dialog text".into(), modal: true, ..Default::default() });
        select_all(&mut h, "dialog text");
        choose(&mut h, "dialog text", "Copy");
        assert_eq!(h.state().copied, ["dialog text"]);
    }

    #[test]
    fn a_long_selection_is_copied_whole() {
        let long = "satır ğüşiöç 0123456789\n".repeat(20_000);
        let mut h = harness(Form { notes: long.clone(), ..Default::default() });
        // The field is far taller than the window: press near its top, which is in view.
        let top = field(&h, &long).accesskit_node().bounding_box().unwrap();
        let pos = egui::pos2(top.x0 as f32 + 20.0, top.y0 as f32 + 20.0);
        let click = |h: &mut egui_kittest::Harness<'static, Form>, button| {
            h.event(egui::Event::PointerMoved(pos));
            h.run();
            for pressed in [true, false] {
                h.event(egui::Event::PointerButton { pos, button, pressed, modifiers: egui::Modifiers::NONE });
                h.run();
            }
        };
        click(&mut h, egui::PointerButton::Primary);
        h.event(key(egui::Key::A, egui::Modifiers::COMMAND));
        h.run();
        click(&mut h, egui::PointerButton::Secondary);
        item(&h, "Copy").click();
        h.run();
        assert_eq!(h.state().copied.len(), 1);
        assert!(h.state().copied[0] == long, "all {} bytes arrive", long.len());
    }
}
