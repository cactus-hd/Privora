use crate::state::{LocalState, state_file_path};
use chrono::{Local, NaiveDate};
use eframe::egui::{self, Align, Color32, FontId, Layout, RichText, Stroke, Vec2};
use privora_core::{
    CreatedPackage, LicensePolicy, Manifest, PackageInfo, UnlockedPackage,
    create_package_with_policy, inspect_package, unlock_package,
};
use std::{
    fs,
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    thread,
};

const BG: Color32 = Color32::from_rgb(10, 12, 25);
const SIDEBAR: Color32 = Color32::from_rgb(12, 17, 34);
const PANEL: Color32 = Color32::from_rgb(17, 23, 43);
const PANEL_2: Color32 = Color32::from_rgb(21, 28, 51);
const BORDER: Color32 = Color32::from_rgb(42, 47, 76);
const PURPLE: Color32 = Color32::from_rgb(173, 87, 255);
const PURPLE_LIGHT: Color32 = Color32::from_rgb(204, 137, 255);
const TEXT: Color32 = Color32::from_rgb(244, 241, 250);
const MUTED: Color32 = Color32::from_rgb(156, 163, 187);
const GREEN: Color32 = Color32::from_rgb(71, 207, 160);
const RED: Color32 = Color32::from_rgb(255, 101, 126);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    Home,
    Create,
    Licenses,
    Files,
    Settings,
}

struct CreatedUi {
    path: PathBuf,
    created: CreatedPackage,
}

struct UnlockedUi {
    unlocked: UnlockedPackage,
}

pub struct PrivoraApp {
    page: Page,
    state: LocalState,
    selected_files: Vec<PathBuf>,
    package_name: String,
    valid_from_enabled: bool,
    valid_from_date: String,
    expiry_enabled: bool,
    expiry_date: String,
    max_activations: String,
    recipient_label: String,
    license_note: String,
    license_search: String,
    create_rx: Option<Receiver<Result<CreatedUi, String>>>,
    unlock_rx: Option<Receiver<Result<UnlockedUi, String>>>,
    busy: bool,
    status: Option<(bool, String)>,
    last_created: Option<CreatedUi>,
    selected_package: Option<PathBuf>,
    selected_package_info: Option<PackageInfo>,
    access_code: String,
    export_folder: String,
}

impl PrivoraApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure_fonts(&cc.egui_ctx);
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = BG;
        visuals.window_fill = PANEL;
        visuals.extreme_bg_color = Color32::from_rgb(7, 9, 19);
        visuals.faint_bg_color = PANEL_2;
        visuals.selection.bg_fill = Color32::from_rgb(84, 45, 137);
        visuals.selection.stroke = Stroke::new(1.0, PURPLE_LIGHT);
        visuals.widgets.inactive.bg_fill = PANEL_2;
        visuals.widgets.inactive.fg_stroke.color = TEXT;
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(49, 39, 76);
        visuals.widgets.hovered.fg_stroke.color = TEXT;
        visuals.widgets.active.bg_fill = PURPLE;
        visuals.widgets.active.fg_stroke.color = Color32::WHITE;
        cc.egui_ctx.set_visuals(visuals);

        let mut state = LocalState::load();
        if state.export_folder.trim().is_empty() {
            state.export_folder = dirs::download_dir()
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| std::env::temp_dir()))
                .join("Privora-Unlocked")
                .to_string_lossy()
                .to_string();
        }
        let export_folder = state.export_folder.clone();
        Self {
            page: Page::Home,
            state,
            selected_files: Vec::new(),
            package_name: "Private Package".into(),
            valid_from_enabled: false,
            valid_from_date: Local::now().date_naive().format("%Y-%m-%d").to_string(),
            expiry_enabled: false,
            expiry_date: (Local::now().date_naive() + chrono::Duration::days(30))
                .format("%Y-%m-%d")
                .to_string(),
            max_activations: "3".into(),
            recipient_label: String::new(),
            license_note: String::new(),
            license_search: String::new(),
            create_rx: None,
            unlock_rx: None,
            busy: false,
            status: None,
            last_created: None,
            selected_package: None,
            selected_package_info: None,
            access_code: String::new(),
            export_folder,
        }
    }

    fn poll_workers(&mut self) {
        let create_result = self.create_rx.as_ref().and_then(|rx| rx.try_recv().ok());
        if let Some(result) = create_result {
            self.create_rx = None;
            self.busy = false;
            match result {
                Ok(created) => {
                    self.state.add_package(
                        created.created.manifest.clone(),
                        created.path.to_string_lossy().to_string(),
                        created.created.fingerprint.clone(),
                    );
                    self.status = Some((
                        true,
                        format!("Encrypted package created: {}", created.path.display()),
                    ));
                    self.last_created = Some(created);
                    if let Err(e) = self.state.save() {
                        self.status = Some((
                            false,
                            format!("Package created, but local history could not be saved: {e}"),
                        ));
                    }
                }
                Err(error) => self.status = Some((false, error)),
            }
        }

        let unlock_result = self.unlock_rx.as_ref().and_then(|rx| rx.try_recv().ok());
        if let Some(result) = unlock_result {
            self.unlock_rx = None;
            self.busy = false;
            match result {
                Ok(result) => {
                    let unlocked = result.unlocked;
                    let fingerprint = unlocked.package_fingerprint.clone();
                    *self.state.activation_counts.entry(fingerprint).or_insert(0) += 1;
                    self.state.add_unlock(
                        unlocked.manifest.name.clone(),
                        unlocked.output_dir.to_string_lossy().to_string(),
                        unlocked.written_files.len(),
                    );
                    self.export_folder = unlocked
                        .output_dir
                        .parent()
                        .unwrap_or(&unlocked.output_dir)
                        .to_string_lossy()
                        .to_string();
                    self.state.export_folder = self.export_folder.clone();
                    self.status = Some((
                        true,
                        format!(
                            "Unlocked {} file(s) to {}",
                            unlocked.written_files.len(),
                            unlocked.output_dir.display()
                        ),
                    ));
                    self.access_code.clear();
                    if let Err(e) = self.state.save() {
                        self.status = Some((
                            false,
                            format!(
                                "Files were extracted, but the local history could not be saved: {e}"
                            ),
                        ));
                    }
                }
                Err(error) => self.status = Some((false, error)),
            }
        }
    }

    fn draw_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("P")
                    .font(FontId::proportional(29.0))
                    .strong()
                    .color(PURPLE_LIGHT),
            );
            ui.vertical(|ui| {
                ui.label(RichText::new("Privora").size(21.0).strong().color(TEXT));
                ui.label(RichText::new("OFFLINE VAULT").size(11.5).color(MUTED));
            });
        });
        ui.add_space(26.0);
        let items = [
            (Page::Home, AppIcon::Home, "Home"),
            (Page::Create, AppIcon::Create, "Create Package"),
            (Page::Licenses, AppIcon::License, "Licenses"),
            (Page::Files, AppIcon::Folder, "Files / Unlock"),
            (Page::Settings, AppIcon::Settings, "Settings"),
        ];
        for (page, icon, label) in items {
            if nav_button(ui, self.page == page, icon, label) {
                self.page = page;
            }
            ui.add_space(5.0);
        }
        ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
            egui::Frame::new()
                .fill(Color32::from_rgb(15, 21, 42))
                .stroke(Stroke::new(1.0, BORDER))
                .corner_radius(egui::CornerRadius::same(10))
                .inner_margin(12)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        icon_widget(ui, AppIcon::Shield, 22.0, PURPLE_LIGHT);
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new("Local-first privacy")
                                    .size(14.0)
                                    .strong()
                                    .color(TEXT),
                            );
                            ui.label(
                                RichText::new("No account · No server")
                                    .size(12.5)
                                    .color(MUTED),
                            );
                        });
                    });
                });
            ui.add_space(12.0);
            ui.label(
                RichText::new("PRIVORA  •  ALPHA 0.3")
                    .size(12.5)
                    .color(MUTED),
            );
            ui.add_space(8.0);
        });
    }

    fn draw_home(&mut self, ui: &mut egui::Ui) {
        page_header(
            ui,
            "Your private workspace",
            "A calmer way to package and share files with access rules.",
        );

        egui::Frame::new()
            .fill(Color32::from_rgb(20, 17, 42))
            .stroke(Stroke::new(1.0, Color32::from_rgb(82, 57, 125)))
            .corner_radius(egui::CornerRadius::same(16))
            .inner_margin(egui::Margin::symmetric(23, 21))
            .show(ui, |ui| {
                ui.columns(2, |cols| {
                    cols[0].vertical(|ui| {
                        ui.label(RichText::new("PRIVATE BY DESIGN").size(11.5).strong().color(PURPLE_LIGHT));
                        ui.add_space(7.0);
                        ui.heading(RichText::new("Share what should stay yours.").size(30.0).strong().color(TEXT));
                        ui.add_space(8.0);
                        ui.label(RichText::new("Bundle documents, images, audio and video into one encrypted .vora package. Keep the access code separate.").size(14.0).color(MUTED));
                        ui.add_space(15.0);
                        if primary_button(ui, "Create a protected package  →").clicked() {
                            self.page = Page::Create;
                        }
                        ui.add_space(8.0);
                        ui.label(RichText::new("No account. No cloud upload. Files stay on this device while packaging.").size(12.0).color(MUTED));
                    });
                    cols[1].vertical_centered(|ui| {
                        ui.add_space(8.0);
                        egui::Frame::new()
                            .fill(Color32::from_rgb(31, 20, 61))
                            .stroke(Stroke::new(1.0, Color32::from_rgb(98, 64, 149)))
                            .corner_radius(egui::CornerRadius::same(22))
                            .inner_margin(egui::Margin::same(24))
                            .show(ui, |ui| {
                                icon_widget(ui, AppIcon::Package, 62.0, PURPLE_LIGHT);
                                ui.add_space(5.0);
                                icon_widget(ui, AppIcon::Shield, 25.0, GREEN);
                            });
                        ui.add_space(8.0);
                        ui.label(RichText::new("AES-256-GCM").size(16.0).strong().color(TEXT));
                        ui.label(RichText::new("Authenticated encryption · Encrypted manifest").size(12.5).color(MUTED));
                    });
                });
            });

        ui.add_space(16.0);
        ui.columns(2, |cols| {
            egui::Frame::new()
                .fill(Color32::from_rgb(19, 23, 45))
                .stroke(Stroke::new(1.0, Color32::from_rgb(67, 57, 102)))
                .corner_radius(egui::CornerRadius::same(13))
                .inner_margin(egui::Margin::symmetric(18, 16))
                .show(&mut cols[0], |ui| {
                    ui.horizontal(|ui| {
                        icon_widget(ui, AppIcon::Package, 37.0, PURPLE_LIGHT);
                        ui.vertical(|ui| {
                            ui.label(RichText::new("Protected packages").size(13.0).strong().color(TEXT));
                            ui.label(RichText::new(self.state.packages.len().to_string()).size(34.0).strong().color(TEXT));
                            ui.label(RichText::new("Created on this device").size(12.0).color(MUTED));
                        });
                    });
                    ui.add_space(7.0);
                    ui.label(RichText::new("Your package catalogue is stored locally. No account or cloud history is used.").size(12.5).color(MUTED));
                });
            cols[1].vertical(|ui| {
                stat_card(
                    ui,
                    "SUCCESSFUL UNLOCKS",
                    self.state.activation_counts.values().map(|v| *v as usize).sum::<usize>().to_string(),
                    "Counted in this local profile",
                );
                ui.add_space(8.0);
                stat_card(
                    ui,
                    "RECENT ACTIVITY",
                    self.state.unlocked.len().to_string(),
                    "Recent extraction records",
                );
            });
        });

        ui.add_space(22.0);
        ui.horizontal(|ui| {
            ui.heading(
                RichText::new("Recent packages")
                    .size(20.0)
                    .strong()
                    .color(TEXT),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .link(RichText::new("View all  →").color(PURPLE_LIGHT))
                    .clicked()
                {
                    self.page = Page::Licenses;
                }
            });
        });
        ui.add_space(9.0);
        if self.state.packages.is_empty() {
            egui::Frame::new()
                .fill(PANEL)
                .stroke(Stroke::new(1.0, BORDER))
                .corner_radius(egui::CornerRadius::same(12))
                .inner_margin(egui::Margin::symmetric(20, 22))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        icon_widget(ui, AppIcon::Package, 33.0, PURPLE_LIGHT);
                        ui.vertical(|ui| {
                            ui.label(RichText::new("Your first package starts here").strong().color(TEXT));
                            ui.label(RichText::new("Select files, define a validity window, then create a .vora package.").color(MUTED));
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if primary_button(ui, "Create package").clicked() { self.page = Page::Create; }
                        });
                    });
                });
        } else {
            egui::ScrollArea::vertical()
                .max_height(265.0)
                .show(ui, |ui| {
                    let records = self
                        .state
                        .packages
                        .iter()
                        .take(5)
                        .cloned()
                        .collect::<Vec<_>>();
                    for record in records {
                        package_row(ui, &record.manifest, &record.path);
                    }
                });
        }
        ui.add_space(14.0);
        egui::Frame::new()
            .fill(Color32::from_rgb(15, 23, 39))
            .stroke(Stroke::new(1.0, BORDER))
            .corner_radius(egui::CornerRadius::same(10))
            .inner_margin(12)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    icon_widget(ui, AppIcon::Shield, 20.0, GREEN);
                    ui.label(RichText::new("Privacy note").strong().color(TEXT));
                    ui.label(RichText::new("The package protects stored bytes; after a recipient extracts them, their later use cannot be controlled by this offline app.").size(12.5).color(MUTED));
                });
            });
    }

    fn draw_create(&mut self, ui: &mut egui::Ui) {
        page_header(
            ui,
            "Create New Package",
            "Choose the files first, then set an expiry date and local unlock limit.",
        );
        let mut dismiss_created = false;
        let mut save_license_requested = false;
        let mut copy_code_requested = false;
        if let Some(created) = &self.last_created {
            egui::Frame::new().fill(Color32::from_rgb(12, 39, 39)).stroke(Stroke::new(1.0, GREEN))
                .corner_radius(egui::CornerRadius::same(12)).inner_margin(14).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        icon_widget(ui, AppIcon::Check, 18.0, GREEN);
                        ui.label(RichText::new("Package created successfully").size(16.0).strong().color(GREEN));
                    });
                    ui.add_space(5.0);
                    ui.label(RichText::new(created.path.display().to_string()).size(12.5).color(TEXT));
                    ui.add_space(3.0);
                    ui.label(RichText::new("ACCESS CODE — this code unlocks the package").size(12.0).strong().color(MUTED));
                    ui.add(egui::Label::new(RichText::new(&created.created.access_code).font(FontId::monospace(14.0)).color(PURPLE_LIGHT)).selectable(true));
                    ui.add_space(5.0);
                    ui.label(RichText::new("Save the code somewhere safe. The .vora file does not contain it.").size(12.0).color(MUTED));
                    ui.horizontal_wrapped(|ui| {
                        if primary_button(ui, "Copy access code").clicked() {
                            copy_code_requested = true;
                        }
                        if ui.button("Save separate license note...").clicked() {
                            save_license_requested = true;
                        }
                        if ui.button("Dismiss").clicked() {
                            dismiss_created = true;
                        }
                    });
                    ui.label(RichText::new("A saved license note contains the secret in plain text. Share it separately from the .vora file.").size(11.5).color(MUTED));
                });
            ui.add_space(10.0);
        }
        if copy_code_requested {
            if let Some(code) = self
                .last_created
                .as_ref()
                .map(|created| created.created.access_code.clone())
            {
                ui.ctx().copy_text(code);
                self.status = Some((
                    true,
                    "Access code copied. Share it separately from the .vora file.".into(),
                ));
            }
        }
        if save_license_requested {
            if let Some(created) = &self.last_created {
                if let Some(license_path) = rfd::FileDialog::new()
                    .set_title("Save separate Privora license note")
                    .set_file_name("Privora-License.license.txt")
                    .add_filter("Privora license note", &["txt"])
                    .save_file()
                {
                    let expiry_text = created
                        .created
                        .manifest
                        .expires_on
                        .as_deref()
                        .unwrap_or("Never");
                    let note = format!(
                        "PRIVORA OFFLINE LICENSE NOTE — V3\n\nPackage: {}\nPackage ID: {}\nLicense code: {}\nRecipient label: {}\nValid from: {}\nExpires: {}\nLocal unlock limit: {}\nLicense note: {}\n\nKeep this code separate from the .vora package. Anyone who has both the package and code can decrypt the content.\nThe unlock counter is local to one app profile and is not a global person/device count.\n",
                        created.created.manifest.name,
                        created.created.manifest.package_id,
                        created.created.access_code,
                        created
                            .created
                            .manifest
                            .recipient_label
                            .as_deref()
                            .unwrap_or("Not specified"),
                        created
                            .created
                            .manifest
                            .valid_from
                            .as_deref()
                            .unwrap_or("Immediately"),
                        expiry_text,
                        created.created.manifest.max_activations,
                        created
                            .created
                            .manifest
                            .license_note
                            .as_deref()
                            .unwrap_or("None"),
                    );
                    match fs::write(&license_path, note) {
                        Ok(()) => {
                            self.status = Some((
                                true,
                                format!(
                                    "Separate license note saved to {}",
                                    license_path.display()
                                ),
                            ))
                        }
                        Err(error) => {
                            self.status =
                                Some((false, format!("Could not save license note: {error}")))
                        }
                    }
                }
            }
        }
        if dismiss_created {
            self.last_created = None;
        }

        ui.horizontal(|ui| {
            step_badge(ui, "1", "Content", !self.selected_files.is_empty());
            ui.add_space(8.0);
            ui.label(RichText::new("------").color(BORDER));
            ui.add_space(8.0);
            step_badge(ui, "2", "License", true);
            ui.add_space(8.0);
            ui.label(RichText::new("------").color(BORDER));
            ui.add_space(8.0);
            step_badge(ui, "3", "Create", false);
        });
        ui.add_space(14.0);
        ui.columns(2, |cols| {
            egui::Frame::new().fill(PANEL).stroke(Stroke::new(1.0, BORDER)).corner_radius(egui::CornerRadius::same(12)).inner_margin(16)
                .show(&mut cols[0], |ui| {
                    ui.heading(RichText::new("Content Files").size(17.0).strong().color(TEXT));
                    ui.label(RichText::new("Privora protects the original bytes; it does not change or recompress your files.").size(13.0).color(MUTED));
                    ui.add_space(10.0);
                    let frame = egui::Frame::new().fill(Color32::from_rgb(13, 18, 35)).stroke(Stroke::new(1.0, Color32::from_rgb(70, 60, 107)))
                        .corner_radius(egui::CornerRadius::same(10)).inner_margin(16);
                    frame.show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            icon_widget(ui, AppIcon::Upload, 34.0, PURPLE_LIGHT);
                            ui.label(RichText::new("Select or drop files to protect").size(14.0).strong().color(TEXT));
                            ui.label(RichText::new("Text · Image · Audio · Video · Other files").size(12.5).color(MUTED));
                            ui.add_space(8.0);
                            if primary_button(ui, "Browse Files").clicked() {
                                if let Some(files) = rfd::FileDialog::new().set_title("Choose content files").pick_files() {
                                    for file in files { if !self.selected_files.contains(&file) { self.selected_files.push(file); } }
                                }
                            }
                        });
                    });
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("Selected: {} file(s)", self.selected_files.len())).strong().color(TEXT));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.link(RichText::new("Clear all").color(RED)).clicked() { self.selected_files.clear(); }
                        });
                    });
                    let mut remove_index = None;
                    egui::ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
                        for (index, path) in self.selected_files.iter().enumerate() {
                            egui::Frame::new().fill(PANEL_2).corner_radius(egui::CornerRadius::same(7)).inner_margin(8).show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    icon_widget(ui, file_icon(path), 22.0, PURPLE_LIGHT);
                                    ui.vertical(|ui| {
                                        ui.label(RichText::new(path.file_name().and_then(|s| s.to_str()).unwrap_or("Unnamed file")).size(13.0).color(TEXT));
                                        let size = fs::metadata(path).map(|m| human_size(m.len())).unwrap_or_else(|_| "Unavailable".into());
                                        ui.label(RichText::new(size).size(11.5).color(MUTED));
                                    });
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        if icon_button(ui, AppIcon::Remove, 24.0, MUTED).clicked() { remove_index = Some(index); }
                                    });
                                });
                            });
                            ui.add_space(4.0);
                        }
                    });
                    if let Some(index) = remove_index { self.selected_files.remove(index); }
                });
            egui::Frame::new().fill(PANEL).stroke(Stroke::new(1.0, BORDER)).corner_radius(egui::CornerRadius::same(12)).inner_margin(16)
                .show(&mut cols[1], |ui| {
                    ui.heading(RichText::new("Package Settings").size(17.0).strong().color(TEXT));
                    ui.label(RichText::new("Set a package name and optional access rules. Privora generates a random access code for you.").size(13.0).color(MUTED));
                    ui.add_space(13.0);
                    ui.label(RichText::new("Package name").size(13.0).color(MUTED));
                    ui.add_sized([ui.available_width(), 35.0], egui::TextEdit::singleline(&mut self.package_name).hint_text("e.g. Design Course"));
                    ui.add_space(12.0);
                    ui.label(RichText::new("Recipient label (optional)").size(13.5).strong().color(TEXT));
                    ui.add_sized([ui.available_width(), 37.0], egui::TextEdit::singleline(&mut self.recipient_label).hint_text("e.g. Design team / Amir"));
                    ui.label(RichText::new("A private label for your own reference; it does not verify a person's identity.").size(11.5).color(MUTED));
                    ui.add_space(10.0);
                    ui.label(RichText::new("License note (optional)").size(13.5).strong().color(TEXT));
                    ui.add_sized([ui.available_width(), 70.0], egui::TextEdit::multiline(&mut self.license_note).desired_rows(3).hint_text("Purpose, internal reference, or handling instructions"));
                    ui.label(RichText::new("Up to 2,048 bytes. This note is encrypted inside the package.").size(11.5).color(MUTED));
                    ui.add_space(10.0);
                    ui.label(RichText::new("Validity window").size(13.5).strong().color(TEXT));
                    ui.checkbox(&mut self.valid_from_enabled, "Do not allow access before a date");
                    if self.valid_from_enabled {
                        ui.add_sized([ui.available_width(), 35.0], egui::TextEdit::singleline(&mut self.valid_from_date).hint_text("YYYY-MM-DD"));
                        ui.label(RichText::new("Access starts at the beginning of this local calendar date.").size(11.5).color(MUTED));
                    }
                    ui.add_space(6.0);
                    ui.checkbox(&mut self.expiry_enabled, "Set an expiration date");
                    if self.expiry_enabled {
                        ui.add_sized([ui.available_width(), 35.0], egui::TextEdit::singleline(&mut self.expiry_date).hint_text("YYYY-MM-DD"));
                        ui.label(RichText::new("The package remains valid through this local calendar date.").size(11.5).color(MUTED));
                    }
                    ui.add_space(10.0);
                    ui.label(RichText::new("Maximum successful unlocks on this installation").size(13.0).strong().color(TEXT));
                    ui.add_sized([ui.available_width(), 35.0], egui::TextEdit::singleline(&mut self.max_activations).hint_text("3"));
                    ui.add_space(12.0);
                    egui::Frame::new().fill(Color32::from_rgb(38, 28, 51)).corner_radius(egui::CornerRadius::same(8)).inner_margin(10).show(ui, |ui| {
                        ui.label(RichText::new("Offline limitation").size(13.0).strong().color(PURPLE_LIGHT));
                        ui.label(RichText::new("Offline limit: this counter lives on this app installation. It cannot count people globally across different computers.").size(12.5).color(MUTED));
                    });
                    ui.add_space(15.0);
                    if self.busy {
                        ui.horizontal(|ui| { ui.spinner(); ui.label(RichText::new("Creating encrypted package...").color(PURPLE_LIGHT)); });
                    } else if primary_button(ui, "Create encrypted .vora").clicked() {
                        self.begin_create();
                    }
                    if self.selected_files.is_empty() {
                        ui.label(RichText::new("Select at least one file to continue.").size(12.5).color(MUTED));
                    }
                });
        });
    }

    fn begin_create(&mut self) {
        if self.selected_files.is_empty() {
            self.status = Some((false, "Select at least one file first.".into()));
            return;
        }
        let max_activations = match self.max_activations.trim().parse::<u32>() {
            Ok(value) if (1..=100_000).contains(&value) => value,
            _ => {
                self.status = Some((
                    false,
                    "Activation limit must be a number between 1 and 100,000.".into(),
                ));
                return;
            }
        };
        let valid_from = if self.valid_from_enabled {
            match NaiveDate::parse_from_str(self.valid_from_date.trim(), "%Y-%m-%d") {
                Ok(date) => Some(date),
                Err(_) => {
                    self.status = Some((
                        false,
                        "Start date must use YYYY-MM-DD (for example 2027-01-31).".into(),
                    ));
                    return;
                }
            }
        } else {
            None
        };
        let expiry = if self.expiry_enabled {
            match NaiveDate::parse_from_str(self.expiry_date.trim(), "%Y-%m-%d") {
                Ok(date) if date >= Local::now().date_naive() => Some(date),
                Ok(_) => {
                    self.status = Some((false, "Expiration date cannot be in the past.".into()));
                    return;
                }
                Err(_) => {
                    self.status = Some((
                        false,
                        "Expiration must use YYYY-MM-DD (for example 2027-01-31).".into(),
                    ));
                    return;
                }
            }
        } else {
            None
        };
        if let (Some(start), Some(end)) = (valid_from, expiry) {
            if start > end {
                self.status = Some((
                    false,
                    "The start date must be on or before the expiration date.".into(),
                ));
                return;
            }
        }
        if self.recipient_label.trim().len() > 160 {
            self.status = Some((false, "Recipient label must be 160 bytes or less.".into()));
            return;
        }
        if self.license_note.len() > 2_048 {
            self.status = Some((false, "License note must be 2,048 bytes or less.".into()));
            return;
        }
        if self.package_name.trim().is_empty() {
            self.status = Some((false, "Enter a package name.".into()));
            return;
        }
        let Some(mut output_path) = rfd::FileDialog::new()
            .set_title("Save encrypted Privora package")
            .set_file_name("Privora-Package.vora")
            .add_filter("Privora encrypted package", &["vora"])
            .save_file()
        else {
            return;
        };
        if output_path
            .extension()
            .and_then(|x| x.to_str())
            .is_none_or(|ext| !ext.eq_ignore_ascii_case("vora"))
        {
            output_path.set_extension("vora");
        }
        let files = self.selected_files.clone();
        let name = self.package_name.trim().to_owned();
        let policy = LicensePolicy {
            valid_from,
            recipient_label: Some(self.recipient_label.trim().to_owned()),
            note: Some(self.license_note.trim().to_owned()),
        };
        let (tx, rx) = mpsc::channel();
        self.create_rx = Some(rx);
        self.busy = true;
        self.status = Some((
            true,
            "Building archive and encrypting every chunk...".into(),
        ));
        thread::spawn(move || {
            let result = create_package_with_policy(
                &files,
                &output_path,
                &name,
                expiry,
                max_activations,
                policy,
            )
            .map_err(|e| e.to_string())
            .map(|created| CreatedUi {
                path: output_path,
                created,
            });
            let _ = tx.send(result);
        });
    }

    fn draw_licenses(&mut self, ui: &mut egui::Ui) {
        page_header(
            ui,
            "Licenses",
            "Review the offline access rules attached to packages you created.",
        );
        egui::Frame::new()
            .fill(Color32::from_rgb(38, 28, 51))
            .stroke(Stroke::new(1.0, Color32::from_rgb(87, 53, 122)))
            .corner_radius(egui::CornerRadius::same(10))
            .inner_margin(12)
            .show(ui, |ui| {
                ui.label(RichText::new("Offline policy notice").strong().color(PURPLE_LIGHT));
                ui.label(RichText::new("Dates and unlock counts are checked on this device. Offline policies cannot enforce one shared person/device count or remotely revoke a copied package.").size(13.0).color(MUTED));
            });
        ui.add_space(12.0);
        ui.add_sized(
            [ui.available_width(), 40.0],
            egui::TextEdit::singleline(&mut self.license_search)
                .hint_text("Search by package, recipient label, or file path..."),
        );

        let query = self.license_search.trim().to_lowercase();
        let packages = self
            .state
            .packages
            .iter()
            .filter(|record| {
                query.is_empty()
                    || record.manifest.name.to_lowercase().contains(&query)
                    || record
                        .manifest
                        .recipient_label
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&query)
                    || record.path.to_lowercase().contains(&query)
            })
            .cloned()
            .collect::<Vec<_>>();

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!(
                    "{} of {} package(s)",
                    packages.len(),
                    self.state.packages.len()
                ))
                .strong()
                .color(TEXT),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if primary_button(ui, "+ Create package").clicked() {
                    self.page = Page::Create;
                }
            });
        });
        ui.add_space(8.0);

        if packages.is_empty() {
            if self.state.packages.is_empty() {
                empty_panel(
                    ui,
                    "No licenses yet",
                    "Create a .vora package to see its offline policy here.",
                );
            } else {
                empty_panel(
                    ui,
                    "No matching packages",
                    "Try a shorter search or clear the search field.",
                );
            }
            return;
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            for record in packages {
                let manifest = record.manifest;
                let current_count = self
                    .state
                    .activation_counts
                    .get(&record.fingerprint)
                    .copied()
                    .unwrap_or(0);
                let today = Local::now().date_naive();
                let expired = manifest
                    .expires_on
                    .as_deref()
                    .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
                    .is_some_and(|d| today > d);
                let scheduled = manifest
                    .valid_from
                    .as_deref()
                    .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
                    .is_some_and(|d| today < d);
                let limit_reached = current_count >= manifest.max_activations;

                egui::Frame::new()
                    .fill(PANEL)
                    .stroke(Stroke::new(1.0, BORDER))
                    .corner_radius(egui::CornerRadius::same(11))
                    .inner_margin(14)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            icon_widget(ui, AppIcon::Package, 24.0, PURPLE_LIGHT);
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new(&manifest.name)
                                        .size(14.0)
                                        .strong()
                                        .color(TEXT),
                                );
                                ui.label(
                                    RichText::new(format!(
                                        "{} file(s) · {}",
                                        manifest.file_count,
                                        shorten_path(&record.path, 54)
                                    ))
                                    .size(11.5)
                                    .color(MUTED),
                                );
                            });
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let (label, color) = if expired {
                                    ("EXPIRED", RED)
                                } else if scheduled {
                                    ("SCHEDULED", PURPLE_LIGHT)
                                } else if limit_reached {
                                    ("LOCAL LIMIT REACHED", RED)
                                } else {
                                    ("AVAILABLE", GREEN)
                                };
                                ui.label(RichText::new(label).size(10.5).strong().color(color));
                            });
                        });
                        ui.add_space(8.0);
                        ui.horizontal_wrapped(|ui| {
                            metadata_chip(
                                ui,
                                "Valid from",
                                manifest.valid_from.as_deref().unwrap_or("Immediately"),
                            );
                            metadata_chip(
                                ui,
                                "Expires",
                                manifest.expires_on.as_deref().unwrap_or("Never"),
                            );
                            metadata_chip(
                                ui,
                                "Local unlocks",
                                &format!("{current_count} / {}", manifest.max_activations),
                            );
                            if let Some(recipient) = &manifest.recipient_label {
                                metadata_chip(ui, "Recipient label", recipient);
                            }
                            metadata_chip(
                                ui,
                                "Created",
                                &manifest.created_at.chars().take(10).collect::<String>(),
                            );
                            if record.has_private_note {
                                metadata_chip(ui, "Encrypted note", "Included in .vora");
                            }
                        });
                    });
                ui.add_space(7.0);
            }
        });
    }

    fn draw_files(&mut self, ui: &mut egui::Ui) {
        page_header(
            ui,
            "Import & Unlock",
            "Unlock an encrypted .vora package using its access code, then choose where to extract its files.",
        );
        ui.columns(2, |cols| {
            egui::Frame::new().fill(PANEL).stroke(Stroke::new(1.0, BORDER)).corner_radius(egui::CornerRadius::same(12)).inner_margin(16).show(&mut cols[0], |ui| {
                ui.vertical_centered(|ui| {
                    icon_widget(ui, AppIcon::Folder, 42.0, PURPLE_LIGHT);
                    ui.heading(RichText::new("Import .vora File").size(18.0).strong().color(TEXT));
                    ui.label(RichText::new("Choose the package you received.").size(13.0).color(MUTED));
                    ui.add_space(8.0);
                    if primary_button(ui, "Browse .vora File").clicked() {
                        if let Some(path) = rfd::FileDialog::new().set_title("Open Privora package").add_filter("Privora package", &["vora"]).pick_file() {
                            self.selected_package = None;
                            self.selected_package_info = None;
                            match inspect_package(&path) {
                                Ok(info) => {
                                    self.selected_package = Some(path);
                                    self.selected_package_info = Some(info.clone());
                                    self.access_code.clear();
                                    let message = if info.metadata_encrypted {
                                        "Encrypted v2 header verified. Enter the access code to reveal package details and unlock."
                                    } else {
                                        "Legacy v1 package detected. Its metadata is not encrypted; enter the access code to unlock."
                                    };
                                    self.status = Some((true, message.into()));
                                }
                                Err(e) => self.status = Some((false, e.to_string())),
                            }
                        }
                    }
                });
                if let Some(path) = &self.selected_package {
                    if let Some(info) = &self.selected_package_info {
                        ui.add_space(12.0);
                        ui.separator();
                        ui.label(RichText::new(path.file_name().and_then(|n| n.to_str()).unwrap_or("Selected package")).size(15.0).strong().color(TEXT));
                        ui.label(RichText::new(path.display().to_string()).size(12.5).color(MUTED));
                        ui.add_space(5.0);
                        ui.horizontal_wrapped(|ui| {
                            metadata_chip(ui, "Format", &format!("v{}", info.format_version));
                            metadata_chip(ui, "Metadata", if info.metadata_encrypted { "Encrypted" } else { "Public (legacy)" });
                            metadata_chip(ui, "Access code", "Required");
                        });
                        ui.add_space(8.0);
                        egui::Frame::new().fill(Color32::from_rgb(13, 18, 35)).corner_radius(egui::CornerRadius::same(8)).inner_margin(10).show(ui, |ui| {
                            let message = if info.metadata_encrypted {
                                "Package details are hidden. After code verification, Privora checks the expiry and unlock policy."
                            } else {
                                "This legacy package exposes some header metadata. Create a new v2 package to conceal it."
                            };
                            ui.label(RichText::new(message).size(13.0).color(MUTED));
                        });
                    }
                }
            });
            egui::Frame::new().fill(PANEL).stroke(Stroke::new(1.0, BORDER)).corner_radius(egui::CornerRadius::same(12)).inner_margin(16).show(&mut cols[1], |ui| {
                ui.vertical_centered(|ui| {
                    icon_widget(ui, AppIcon::Shield, 38.0, PURPLE_LIGHT);
                    ui.heading(RichText::new("License Access").size(18.0).strong().color(TEXT));
                    ui.label(RichText::new("Enter the secret code that was shared with you separately.").size(13.0).color(MUTED));
                });
                ui.add_space(13.0);
                ui.label(RichText::new("Access code").size(13.0).color(MUTED));
                ui.add_sized([ui.available_width(), 38.0], egui::TextEdit::singleline(&mut self.access_code)
                    .font(FontId::monospace(13.0)).hint_text("PV2-XXXXX-XXXXX-..."));
                if ui.button("Import code from .license.txt").clicked() {
                    if let Some(path) = rfd::FileDialog::new().set_title("Choose Privora license note").add_filter("License note", &["txt"]).pick_file() {
                        match fs::read_to_string(path) {
                            Ok(text) => {
                                if let Some(line) = text.lines().find(|line| line.trim_start().starts_with("License code:")) {
                                    self.access_code = line.split_once(':').map(|(_, v)| v.trim().to_string()).unwrap_or_default();
                                    self.status = Some((true, "License code imported.".into()));
                                } else { self.status = Some((false, "No 'License code:' field found in that file.".into())); }
                            }
                            Err(e) => self.status = Some((false, format!("Could not read license note: {e}"))),
                        }
                    }
                }
                ui.add_space(12.0);
                ui.label(RichText::new("Extraction folder").size(13.0).color(MUTED));
                ui.horizontal(|ui| {
                    ui.label(RichText::new(shorten_path(&self.export_folder, 28)).size(12.5).color(TEXT));
                    if ui.button("Choose...").clicked() {
                        if let Some(path) = rfd::FileDialog::new().set_title("Choose extraction root folder").pick_folder() {
                            self.export_folder = path.to_string_lossy().to_string();
                            self.state.export_folder = self.export_folder.clone();
                            let _ = self.state.save();
                        }
                    }
                });
                ui.add_space(14.0);
                if self.busy {
                    ui.horizontal(|ui| { ui.spinner(); ui.label(RichText::new("Verifying and decrypting...").color(PURPLE_LIGHT)); });
                } else if primary_button(ui, "Verify code and extract").clicked() {
                    self.begin_unlock();
                }
                if self.selected_package.is_none() {
                    ui.label(RichText::new("Import a .vora package first.").size(12.5).color(MUTED));
                }
                ui.add_space(10.0);
                ui.label(RichText::new("Share the .vora file and its access code through separate, trusted channels.").size(12.5).color(MUTED));
            });
        });
        ui.add_space(15.0);
        ui.heading(
            RichText::new("Recently Unlocked")
                .size(18.0)
                .strong()
                .color(TEXT),
        );
        if self.state.unlocked.is_empty() {
            empty_panel(
                ui,
                "Nothing unlocked yet",
                "Successfully extracted packages are listed here.",
            );
        } else {
            for item in self.state.unlocked.iter().take(4) {
                egui::Frame::new()
                    .fill(PANEL)
                    .stroke(Stroke::new(1.0, BORDER))
                    .corner_radius(egui::CornerRadius::same(8))
                    .inner_margin(10)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            icon_widget(ui, AppIcon::Check, 17.0, GREEN);
                            ui.vertical(|ui| {
                                ui.label(RichText::new(&item.name).strong().color(TEXT));
                                ui.label(
                                    RichText::new(format!(
                                        "{} file(s) · {}",
                                        item.file_count, item.output_dir
                                    ))
                                    .size(11.5)
                                    .color(MUTED),
                                );
                            });
                        });
                    });
                ui.add_space(4.0);
            }
        }
    }

    fn begin_unlock(&mut self) {
        let Some(path) = self.selected_package.clone() else {
            self.status = Some((false, "Import a .vora file first.".into()));
            return;
        };
        if self.access_code.trim().is_empty() {
            self.status = Some((false, "Enter the access code.".into()));
            return;
        }
        let info = match inspect_package(&path) {
            Ok(info) => info,
            Err(e) => {
                self.status = Some((false, e.to_string()));
                return;
            }
        };
        let prior = self
            .state
            .activation_counts
            .get(&info.fingerprint)
            .copied()
            .unwrap_or(0);
        let code = self.access_code.trim().to_owned();
        let root = PathBuf::from(self.export_folder.trim());
        if self.export_folder.trim().is_empty() {
            self.status = Some((false, "Choose an extraction folder.".into()));
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.unlock_rx = Some(rx);
        self.busy = true;
        self.status = Some((
            true,
            "Checking policy and decrypting authenticated chunks...".into(),
        ));
        thread::spawn(move || {
            let result = unlock_package(&path, &code, &root, prior)
                .map(|unlocked| UnlockedUi { unlocked })
                .map_err(|e| e.to_string());
            let _ = tx.send(result);
        });
    }

    fn draw_settings(&mut self, ui: &mut egui::Ui) {
        page_header(ui, "Settings", "Local storage and offline behavior.");
        egui::Frame::new()
            .fill(PANEL)
            .stroke(Stroke::new(1.0, BORDER))
            .corner_radius(egui::CornerRadius::same(12))
            .inner_margin(18)
            .show(ui, |ui| {
                ui.heading(
                    RichText::new("Default extraction location")
                        .size(16.0)
                        .strong()
                        .color(TEXT),
                );
                ui.label(
                    RichText::new("Used when importing and unlocking a package.")
                        .size(13.0)
                        .color(MUTED),
                );
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [ui.available_width() - 100.0, 34.0],
                        egui::TextEdit::singleline(&mut self.export_folder),
                    );
                    if ui.button("Browse...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_title("Choose default extraction folder")
                            .pick_folder()
                        {
                            self.export_folder = path.to_string_lossy().to_string();
                        }
                    }
                });
                ui.add_space(8.0);
                if primary_button(ui, "Save Settings").clicked() {
                    self.state.export_folder = self.export_folder.clone();
                    match self.state.save() {
                        Ok(()) => self.status = Some((true, "Settings saved locally.".into())),
                        Err(e) => self.status = Some((false, e)),
                    }
                }
            });
        ui.add_space(12.0);
        egui::Frame::new().fill(PANEL).stroke(Stroke::new(1.0, BORDER)).corner_radius(egui::CornerRadius::same(12)).inner_margin(18).show(ui, |ui| {
            ui.heading(RichText::new("Local app data").size(16.0).strong().color(TEXT));
            ui.label(RichText::new(state_file_path().display().to_string()).size(12.5).color(MUTED));
            ui.label(RichText::new("This JSON stores package history, recipient labels and local counters. Secret access codes and private license-note text are not stored here.").size(13.0).color(MUTED));
        });
        ui.add_space(12.0);
        egui::Frame::new().fill(Color32::from_rgb(40, 23, 38)).stroke(Stroke::new(1.0, Color32::from_rgb(105, 48, 68))).corner_radius(egui::CornerRadius::same(12)).inner_margin(18).show(ui, |ui| {
            ui.heading(RichText::new("Security boundaries in this offline alpha").size(16.0).strong().color(RED));
            ui.label(RichText::new("• Unlock limits are per local app profile, not a global people/device limit.\n• Expiration depends on the system clock and is not resistant to clock rollback.\n• Creating and unlocking uses temporary plaintext ZIP files; secure erasure is not guaranteed.\n• Some creator-side metadata is stored as local JSON; the private license-note text is not copied there. Metadata inside the .vora v2 file is encrypted.\n• Once files are extracted as plaintext, Privora cannot control their later copying.\n• Anyone with both a .vora package and its access code can decrypt it.\n• Do not use this alpha for highly sensitive material before independent security review.").size(13.0).color(MUTED));
        });
    }
}

impl eframe::App for PrivoraApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_workers();
        if self.page == Page::Create {
            let dropped = ui.ctx().input(|input| {
                input
                    .raw
                    .dropped_files
                    .iter()
                    .map(|f| f.path().to_path_buf())
                    .collect::<Vec<_>>()
            });
            for path in dropped {
                if path.is_file() && !self.selected_files.contains(&path) {
                    self.selected_files.push(path);
                }
            }
        }
        let full_size = ui.available_size();
        egui::Frame::new()
            .fill(BG)
            .inner_margin(egui::Margin::same(0))
            .show(ui, |ui| {
                ui.set_min_size(full_size);
                ui.horizontal(|ui| {
                    ui.allocate_ui_with_layout(
                        Vec2::new(218.0, full_size.y),
                        Layout::top_down(Align::Min),
                        |ui| {
                            egui::Frame::new()
                                .fill(SIDEBAR)
                                .inner_margin(egui::Margin::same(16))
                                .show(ui, |ui| self.draw_sidebar(ui));
                        },
                    );
                    ui.vertical(|ui| {
                        let main_size = ui.available_size();
                        ui.set_min_size(main_size);
                        egui::Frame::new()
                            .fill(Color32::from_rgb(10, 13, 27))
                            .inner_margin(egui::Margin::symmetric(20, 12))
                            .stroke(Stroke::new(1.0, BORDER))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let section = match self.page {
                                        Page::Home => "Overview",
                                        Page::Create => "Create package",
                                        Page::Licenses => "Licenses",
                                        Page::Files => "Open package",
                                        Page::Settings => "Settings",
                                    };
                                    ui.label(
                                        RichText::new("PRIVORA")
                                            .size(12.5)
                                            .strong()
                                            .color(PURPLE_LIGHT),
                                    );
                                    ui.label(RichText::new("/  ").size(12.0).color(BORDER));
                                    ui.label(
                                        RichText::new(section).size(13.0).strong().color(TEXT),
                                    );
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        if self.busy {
                                            ui.spinner();
                                            ui.label(
                                                RichText::new("Processing locally...")
                                                    .size(12.5)
                                                    .color(PURPLE_LIGHT),
                                            );
                                        } else {
                                            egui::Frame::new()
                                                .fill(Color32::from_rgb(16, 40, 39))
                                                .stroke(Stroke::new(
                                                    1.0,
                                                    Color32::from_rgb(44, 91, 84),
                                                ))
                                                .corner_radius(egui::CornerRadius::same(20))
                                                .inner_margin(egui::Margin::symmetric(9, 4))
                                                .show(ui, |ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.label(
                                                            RichText::new("●")
                                                                .size(9.0)
                                                                .color(GREEN),
                                                        );
                                                        ui.label(
                                                            RichText::new("OFFLINE MODE")
                                                                .size(10.5)
                                                                .strong()
                                                                .color(GREEN),
                                                        );
                                                    });
                                                });
                                            ui.add_space(8.0);
                                            ui.label(
                                                RichText::new(
                                                    Local::now().format("%d %b %Y").to_string(),
                                                )
                                                .size(12.5)
                                                .color(MUTED),
                                            );
                                        }
                                    });
                                });
                            });
                        ui.add_space(12.0);
                        if let Some((success, message)) = self.status.clone() {
                            let color = if success { GREEN } else { RED };
                            egui::Frame::new()
                                .fill(if success {
                                    Color32::from_rgb(14, 37, 39)
                                } else {
                                    Color32::from_rgb(43, 20, 34)
                                })
                                .stroke(Stroke::new(1.0, color))
                                .corner_radius(egui::CornerRadius::same(8))
                                .inner_margin(9)
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        icon_widget(
                                            ui,
                                            if success {
                                                AppIcon::Check
                                            } else {
                                                AppIcon::Info
                                            },
                                            16.0,
                                            color,
                                        );
                                        ui.label(RichText::new(&message).size(13.0).color(TEXT));
                                        ui.with_layout(
                                            Layout::right_to_left(Align::Center),
                                            |ui| {
                                                if ui.small_button("x").clicked() {
                                                    self.status = None;
                                                }
                                            },
                                        );
                                    });
                                });
                            ui.add_space(10.0);
                        }
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| match self.page {
                                Page::Home => self.draw_home(ui),
                                Page::Create => self.draw_create(ui),
                                Page::Licenses => self.draw_licenses(ui),
                                Page::Files => self.draw_files(ui),
                                Page::Settings => self.draw_settings(ui),
                            });
                    });
                });
            });
    }
}

#[derive(Clone, Copy)]
enum AppIcon {
    Home,
    Create,
    License,
    Folder,
    Settings,
    Shield,
    Package,
    Upload,
    Check,
    Info,
    Remove,
    Document,
    Image,
    Audio,
    Video,
    File,
}

fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    #[cfg(target_os = "windows")]
    let candidates = vec![
        std::env::var_os("WINDIR").map(|p| PathBuf::from(p).join("Fonts").join("segoeui.ttf")),
        Some(PathBuf::from(r"C:\Windows\Fonts\arial.ttf")),
    ];
    #[cfg(target_os = "macos")]
    let candidates = vec![
        Some(PathBuf::from("/System/Library/Fonts/SFNS.ttf")),
        Some(PathBuf::from(
            "/System/Library/Fonts/Supplemental/Arial.ttf",
        )),
    ];
    #[cfg(target_os = "linux")]
    let candidates = vec![
        Some(PathBuf::from(
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        )),
        Some(PathBuf::from(
            "/usr/share/fonts/truetype/liberation2/LiberationSans-Regular.ttf",
        )),
    ];
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    let candidates: Vec<Option<PathBuf>> = Vec::new();

    if let Some(bytes) = candidates
        .into_iter()
        .flatten()
        .find_map(|path| fs::read(path).ok())
    {
        fonts.font_data.insert(
            "privora-platform-ui".to_owned(),
            egui::FontData::from_owned(bytes).into(),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "privora-platform-ui".to_owned());
    }
    ctx.set_fonts(fonts);

    let mut style = (*ctx.style_of(egui::Theme::Dark)).clone();
    style
        .text_styles
        .insert(egui::TextStyle::Body, FontId::proportional(14.0));
    style
        .text_styles
        .insert(egui::TextStyle::Button, FontId::proportional(14.0));
    style
        .text_styles
        .insert(egui::TextStyle::Small, FontId::proportional(12.0));
    style
        .text_styles
        .insert(egui::TextStyle::Heading, FontId::proportional(24.0));
    style.spacing.item_spacing = egui::vec2(10.0, 8.0);
    style.spacing.button_padding = egui::vec2(14.0, 9.0);
    ctx.set_style_of(egui::Theme::Dark, style);
}

fn nav_button(ui: &mut egui::Ui, selected: bool, icon: AppIcon, label: &str) -> bool {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 42.0), egui::Sense::click());
    let fill = if selected {
        Color32::from_rgb(58, 36, 91)
    } else if response.hovered() {
        Color32::from_rgb(27, 31, 55)
    } else {
        SIDEBAR
    };
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same(9), fill);
    if selected {
        let border = Stroke::new(1.0, Color32::from_rgb(104, 66, 159));
        let a = rect.left_top();
        let b = rect.right_top();
        let c = rect.right_bottom();
        let d = rect.left_bottom();
        ui.painter().line_segment([a, b], border);
        ui.painter().line_segment([b, c], border);
        ui.painter().line_segment([c, d], border);
        ui.painter().line_segment([d, a], border);
    }
    let icon_rect = egui::Rect::from_min_size(rect.min + egui::vec2(13.0, 12.0), Vec2::splat(18.0));
    paint_icon(
        ui.painter(),
        icon_rect,
        icon,
        if selected { PURPLE_LIGHT } else { MUTED },
    );
    ui.painter().text(
        rect.min + egui::vec2(45.0, rect.height() / 2.0),
        egui::Align2::LEFT_CENTER,
        label,
        FontId::proportional(14.0),
        if selected { TEXT } else { MUTED },
    );
    response.clicked()
}

fn icon_button(ui: &mut egui::Ui, icon: AppIcon, size: f32, color: Color32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::click());
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, egui::CornerRadius::same(5), PANEL_2);
    }
    let pad = size * 0.22;
    let icon_rect = rect.shrink(pad);
    paint_icon(
        ui.painter(),
        icon_rect,
        icon,
        if response.hovered() { TEXT } else { color },
    );
    response
}

fn icon_widget(ui: &mut egui::Ui, icon: AppIcon, size: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
    paint_icon(ui.painter(), rect, icon, color);
}

fn paint_icon(painter: &egui::Painter, rect: egui::Rect, icon: AppIcon, color: Color32) {
    let scale = rect.width().min(rect.height()) / 24.0;
    let p = |x: f32, y: f32| {
        egui::pos2(
            rect.center().x + (x - 12.0) * scale,
            rect.center().y + (y - 12.0) * scale,
        )
    };
    let stroke = Stroke::new((1.65 * scale).max(1.1), color);
    let line =
        |a: (f32, f32), b: (f32, f32)| painter.line_segment([p(a.0, a.1), p(b.0, b.1)], stroke);
    match icon {
        AppIcon::Home => {
            line((3.0, 11.0), (12.0, 3.0));
            line((12.0, 3.0), (21.0, 11.0));
            line((5.0, 10.0), (5.0, 21.0));
            line((5.0, 21.0), (19.0, 21.0));
            line((19.0, 21.0), (19.0, 10.0));
            line((10.0, 21.0), (10.0, 15.0));
            line((10.0, 15.0), (14.0, 15.0));
            line((14.0, 15.0), (14.0, 21.0));
        }
        AppIcon::Create => {
            line((6.0, 3.0), (15.0, 3.0));
            line((15.0, 3.0), (19.0, 7.0));
            line((19.0, 7.0), (19.0, 21.0));
            line((19.0, 21.0), (6.0, 21.0));
            line((6.0, 21.0), (6.0, 3.0));
            line((15.0, 3.0), (15.0, 7.0));
            line((15.0, 7.0), (19.0, 7.0));
            line((9.0, 13.0), (16.0, 13.0));
            line((12.5, 9.5), (12.5, 16.5));
        }
        AppIcon::License | AppIcon::Shield => {
            line((12.0, 2.5), (20.0, 6.0));
            line((20.0, 6.0), (19.0, 14.0));
            line((19.0, 14.0), (15.5, 19.0));
            line((15.5, 19.0), (12.0, 21.5));
            line((12.0, 21.5), (8.5, 19.0));
            line((8.5, 19.0), (5.0, 14.0));
            line((5.0, 14.0), (4.0, 6.0));
            line((4.0, 6.0), (12.0, 2.5));
            if matches!(icon, AppIcon::License) {
                line((8.0, 12.0), (11.0, 15.0));
                line((11.0, 15.0), (16.0, 9.0));
            }
        }
        AppIcon::Folder => {
            line((3.0, 6.0), (10.0, 6.0));
            line((10.0, 6.0), (12.0, 8.5));
            line((12.0, 8.5), (21.0, 8.5));
            line((21.0, 8.5), (20.0, 19.0));
            line((20.0, 19.0), (4.0, 19.0));
            line((4.0, 19.0), (3.0, 6.0));
            line((3.5, 10.5), (20.5, 10.5));
        }
        AppIcon::Settings => {
            painter.circle_stroke(p(12.0, 12.0), 5.0 * scale, stroke);
            for (a, b) in [
                ((12.0, 2.5), (12.0, 5.0)),
                ((12.0, 19.0), (12.0, 21.5)),
                ((2.5, 12.0), (5.0, 12.0)),
                ((19.0, 12.0), (21.5, 12.0)),
                ((5.3, 5.3), (7.1, 7.1)),
                ((16.9, 16.9), (18.7, 18.7)),
                ((16.9, 7.1), (18.7, 5.3)),
                ((5.3, 18.7), (7.1, 16.9)),
            ] {
                line(a, b);
            }
        }
        AppIcon::Package => {
            line((12.0, 2.5), (21.0, 7.0));
            line((21.0, 7.0), (12.0, 11.5));
            line((12.0, 11.5), (3.0, 7.0));
            line((3.0, 7.0), (12.0, 2.5));
            line((3.0, 7.0), (3.0, 17.0));
            line((3.0, 17.0), (12.0, 21.5));
            line((12.0, 21.5), (21.0, 17.0));
            line((21.0, 17.0), (21.0, 7.0));
            line((12.0, 11.5), (12.0, 21.5));
            line((7.5, 4.8), (16.5, 9.2));
        }
        AppIcon::Upload => {
            line((4.0, 15.0), (4.0, 20.0));
            line((4.0, 20.0), (20.0, 20.0));
            line((20.0, 20.0), (20.0, 15.0));
            line((12.0, 16.0), (12.0, 4.0));
            line((12.0, 4.0), (7.0, 9.0));
            line((12.0, 4.0), (17.0, 9.0));
        }
        AppIcon::Check => {
            line((4.0, 12.0), (9.0, 17.0));
            line((9.0, 17.0), (20.0, 6.0));
        }
        AppIcon::Info => {
            painter.circle_stroke(p(12.0, 12.0), 9.0 * scale, stroke);
            painter.circle_filled(p(12.0, 7.0), 1.1 * scale, color);
            line((12.0, 11.0), (12.0, 17.0));
        }
        AppIcon::Remove => {
            line((6.0, 6.0), (18.0, 18.0));
            line((18.0, 6.0), (6.0, 18.0));
        }
        AppIcon::Document | AppIcon::File => {
            line((6.0, 3.0), (15.0, 3.0));
            line((15.0, 3.0), (19.0, 7.0));
            line((19.0, 7.0), (19.0, 21.0));
            line((19.0, 21.0), (6.0, 21.0));
            line((6.0, 21.0), (6.0, 3.0));
            line((15.0, 3.0), (15.0, 7.0));
            line((15.0, 7.0), (19.0, 7.0));
            line((9.0, 12.0), (16.0, 12.0));
            line((9.0, 16.0), (16.0, 16.0));
        }
        AppIcon::Image => {
            line((3.0, 4.0), (21.0, 4.0));
            line((21.0, 4.0), (21.0, 20.0));
            line((21.0, 20.0), (3.0, 20.0));
            line((3.0, 20.0), (3.0, 4.0));
            painter.circle_stroke(p(16.0, 8.0), 1.8 * scale, stroke);
            line((4.0, 17.0), (9.0, 12.0));
            line((9.0, 12.0), (12.0, 15.0));
            line((12.0, 15.0), (15.0, 12.0));
            line((15.0, 12.0), (20.0, 17.0));
        }
        AppIcon::Audio => {
            line((4.0, 9.0), (8.0, 9.0));
            line((8.0, 9.0), (13.0, 5.0));
            line((13.0, 5.0), (13.0, 19.0));
            line((13.0, 19.0), (8.0, 15.0));
            line((8.0, 15.0), (4.0, 15.0));
            line((4.0, 15.0), (4.0, 9.0));
            line((16.0, 9.0), (18.0, 12.0));
            line((18.0, 12.0), (16.0, 15.0));
            line((18.0, 6.0), (21.0, 12.0));
            line((21.0, 12.0), (18.0, 18.0));
        }
        AppIcon::Video => {
            line((3.0, 6.0), (15.0, 6.0));
            line((15.0, 6.0), (15.0, 18.0));
            line((15.0, 18.0), (3.0, 18.0));
            line((3.0, 18.0), (3.0, 6.0));
            line((15.0, 10.0), (21.0, 7.0));
            line((21.0, 7.0), (21.0, 17.0));
            line((21.0, 17.0), (15.0, 14.0));
            line((7.0, 9.0), (11.0, 12.0));
            line((11.0, 12.0), (7.0, 15.0));
            line((7.0, 15.0), (7.0, 9.0));
        }
    }
}

fn primary_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(
            RichText::new(label)
                .size(14.0)
                .strong()
                .color(Color32::WHITE),
        )
        .fill(PURPLE)
        .stroke(Stroke::new(1.0, PURPLE_LIGHT)),
    )
}

fn page_header(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.add_space(4.0);
    ui.heading(RichText::new(title).size(27.0).strong().color(TEXT));
    ui.label(RichText::new(subtitle).size(14.0).color(MUTED));
    ui.add_space(16.0);
}

fn stat_card(ui: &mut egui::Ui, title: &str, value: String, sub: &str) {
    egui::Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(egui::CornerRadius::same(11))
        .inner_margin(15)
        .show(ui, |ui| {
            ui.label(RichText::new(title).size(10.5).strong().color(MUTED));
            ui.add_space(5.0);
            ui.label(RichText::new(value).size(25.0).strong().color(TEXT));
            ui.label(RichText::new(sub).size(11.5).color(MUTED));
        });
}

fn empty_panel(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    egui::Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(egui::CornerRadius::same(10))
        .inner_margin(20)
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                icon_widget(ui, AppIcon::Shield, 26.0, PURPLE_LIGHT);
                ui.label(RichText::new(title).size(15.0).strong().color(TEXT));
                ui.label(RichText::new(subtitle).size(13.0).color(MUTED));
            });
        });
}

fn package_row(ui: &mut egui::Ui, manifest: &Manifest, path: &str) {
    egui::Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(egui::CornerRadius::same(8))
        .inner_margin(10)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                icon_widget(ui, AppIcon::Package, 22.0, PURPLE_LIGHT);
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(&manifest.name)
                            .size(14.0)
                            .strong()
                            .color(TEXT),
                    );
                    ui.label(
                        RichText::new(format!(
                            "{} file(s) · {} total",
                            manifest.file_count,
                            human_size(manifest.total_file_bytes)
                        ))
                        .size(11.5)
                        .color(MUTED),
                    );
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(manifest.expires_on.as_deref().unwrap_or("No expiry"))
                            .size(11.5)
                            .color(MUTED),
                    );
                    ui.label(RichText::new("ENCRYPTED").size(10.5).strong().color(GREEN));
                });
            });
            ui.label(
                RichText::new(path)
                    .size(10.5)
                    .color(Color32::from_rgb(116, 123, 151)),
            );
        });
    ui.add_space(5.0);
}

fn step_badge(ui: &mut egui::Ui, number: &str, label: &str, done: bool) {
    let color = if done { PURPLE_LIGHT } else { MUTED };
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!(" {number} "))
                .size(13.0)
                .strong()
                .color(Color32::WHITE)
                .background_color(if done { PURPLE } else { PANEL_2 }),
        );
        ui.label(RichText::new(label).size(13.0).color(color));
    });
}

fn metadata_chip(ui: &mut egui::Ui, label: &str, value: &str) {
    egui::Frame::new()
        .fill(PANEL_2)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(8, 5))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{label}:")).size(11.5).color(MUTED));
                ui.label(RichText::new(value).size(11.5).strong().color(TEXT));
            });
        });
}

fn human_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    let value = bytes as f64;
    if value < KB {
        format!("{bytes} B")
    } else if value < KB * KB {
        format!("{:.1} KB", value / KB)
    } else if value < KB * KB * KB {
        format!("{:.1} MB", value / (KB * KB))
    } else {
        format!("{:.2} GB", value / (KB * KB * KB))
    }
}

fn file_icon(path: &std::path::Path) -> AppIcon {
    match path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "txt" | "md" | "pdf" | "doc" | "docx" | "rtf" | "csv" => AppIcon::Document,
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" | "svg" => AppIcon::Image,
        "mp3" | "wav" | "m4a" | "ogg" | "aac" | "flac" => AppIcon::Audio,
        "mp4" | "mov" | "mkv" | "avi" | "webm" => AppIcon::Video,
        _ => AppIcon::File,
    }
}

fn shorten_path(path: &str, max: usize) -> String {
    if path.chars().count() <= max {
        return path.to_owned();
    }
    let tail: String = path
        .chars()
        .rev()
        .take(max.saturating_sub(3))
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    format!("...{tail}")
}
