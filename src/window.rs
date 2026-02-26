use gtk4::gdk;
use gtk4::gdk_pixbuf;
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

use crate::export::{self, ExportFormat, ExportResolution};
use crate::gl_renderer;
use crate::palette;
use crate::shader_presets;
use crate::wallpaper;

pub struct WallrusWindow;

impl WallrusWindow {
    pub fn new(app: &adw::Application) -> adw::ApplicationWindow {
        let state = gl_renderer::new_shared_state();

        // --- Header bar ---
        let header = adw::HeaderBar::new();
        header.set_title_widget(Some(&gtk4::Label::new(Some("Wallrus"))));

        // Hamburger menu with About item
        let menu = gio::Menu::new();
        menu.append(Some("About Wallrus"), Some("win.show-about"));
        let menu_button = gtk4::MenuButton::new();
        menu_button.set_icon_name("open-menu-symbolic");
        menu_button.set_menu_model(Some(&menu));
        header.pack_end(&menu_button);

        // --- GL preview area ---
        let gl_area = gl_renderer::create_gl_area(state.clone());
        gl_area.set_size_request(320, 180);

        // Wrap in an AspectFrame so the preview keeps 16:9
        let aspect_frame = gtk4::AspectFrame::new(0.5, 0.5, 16.0 / 9.0, false);
        aspect_frame.set_child(Some(&gl_area));
        aspect_frame.set_vexpand(true);

        // Preview group — matches PreferencesGroup styling used on the left column
        let preview_group = adw::PreferencesGroup::new();
        preview_group.set_title("Preview");
        preview_group.add(&aspect_frame);
        preview_group.set_vexpand(true);

        // --- Shader preset dropdown ---
        let preset_names = shader_presets::preset_names();
        let preset_list = gtk4::StringList::new(preset_names);
        let preset_row = adw::ComboRow::new();
        preset_row.set_title("Type");
        preset_row.set_model(Some(&preset_list));
        preset_row.set_selected(0);

        // =====================================================================
        // Palette section — category dropdown + FlowBox thumbnail browser
        // =====================================================================

        // Load all categories at startup
        let all_categories: Rc<RefCell<palette::PaletteCategories>> =
            Rc::new(RefCell::new(palette::list_palette_categories()));
        let category_names: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(
            all_categories.borrow().keys().cloned().collect(),
        ));

        // Category dropdown
        let category_names_borrowed = category_names.borrow();
        let category_str_refs: Vec<&str> =
            category_names_borrowed.iter().map(|s| s.as_str()).collect();
        let category_string_list = gtk4::StringList::new(&category_str_refs);
        drop(category_names_borrowed);
        let category_row = adw::ComboRow::new();
        category_row.set_title("Category");
        category_row.set_model(Some(&category_string_list));
        if !category_names.borrow().is_empty() {
            category_row.set_selected(0);
        }

        // FlowBox for palette thumbnails
        let palette_flowbox = gtk4::FlowBox::new();
        palette_flowbox.set_selection_mode(gtk4::SelectionMode::Single);
        palette_flowbox.set_homogeneous(true);
        palette_flowbox.set_min_children_per_line(3);
        palette_flowbox.set_max_children_per_line(10);
        palette_flowbox.set_row_spacing(4);
        palette_flowbox.set_column_spacing(4);
        palette_flowbox.set_margin_start(4);
        palette_flowbox.set_margin_end(4);
        palette_flowbox.set_margin_top(4);
        palette_flowbox.set_margin_bottom(4);

        // Track paths for current FlowBox children
        let palette_paths: Rc<RefCell<Vec<PathBuf>>> = Rc::new(RefCell::new(Vec::new()));

        // Whether palette thumbnails use smooth (bilinear) scaling vs nearest-neighbor
        let smooth_gradient: Rc<Cell<bool>> = Rc::new(Cell::new(false));

        let palette_scroll = gtk4::ScrolledWindow::new();
        palette_scroll.set_child(Some(&palette_flowbox));
        palette_scroll.set_min_content_height(280);
        palette_scroll.set_max_content_height(280);
        palette_scroll.set_propagate_natural_height(false);
        palette_scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

        // --- Helper: populate FlowBox with images from a category ---
        // When `is_custom` is true, a delete button overlay is shown on each thumbnail.
        // `on_delete` is called after a palette is deleted to refresh the view.
        let populate_flowbox = {
            let flowbox = palette_flowbox.clone();
            let paths = palette_paths.clone();
            let smooth = smooth_gradient.clone();
            Rc::new(move |images: &[PathBuf], is_custom: bool, on_delete: Option<Rc<dyn Fn()>>| {
                // Clear existing children
                while let Some(child) = flowbox.first_child() {
                    flowbox.remove(&child);
                }
                paths.borrow_mut().clear();

                if images.is_empty() {
                    let msg = if is_custom {
                        "No saved palettes yet."
                    } else {
                        "No palettes in this category."
                    };
                    let label = gtk4::Label::new(Some(msg));
                    label.set_wrap(true);
                    label.set_justify(gtk4::Justification::Center);
                    label.add_css_class("dim-label");
                    label.set_margin_top(12);
                    label.set_margin_bottom(12);
                    flowbox.insert(&label, -1);
                    return;
                }

                for path in images {
                    let pixbuf_result = if smooth.get() {
                        gdk_pixbuf::Pixbuf::from_file_at_scale(
                            path.to_str().unwrap_or_default(),
                            80,
                            80,
                            false,
                        )
                    } else {
                        gdk_pixbuf::Pixbuf::from_file(path.to_str().unwrap_or_default())
                            .and_then(|pb| {
                                pb.scale_simple(80, 80, gdk_pixbuf::InterpType::Nearest)
                                    .ok_or_else(|| glib::Error::new(gdk_pixbuf::PixbufError::Failed, "scale_simple failed"))
                            })
                    };
                    match pixbuf_result {
                        Ok(pixbuf) => {
                            let texture = gdk::Texture::for_pixbuf(&pixbuf);
                            let image = gtk4::Picture::for_paintable(&texture);
                            image.set_size_request(80, 80);
                            image.set_content_fit(gtk4::ContentFit::Cover);

                            if is_custom {
                                // Wrap in overlay with delete button
                                let overlay = gtk4::Overlay::new();
                                overlay.set_child(Some(&image));

                                let delete_btn = gtk4::Button::from_icon_name("window-close-symbolic");
                                delete_btn.add_css_class("circular");
                                delete_btn.add_css_class("osd");
                                delete_btn.set_halign(gtk4::Align::End);
                                delete_btn.set_valign(gtk4::Align::Start);
                                delete_btn.set_margin_top(2);
                                delete_btn.set_margin_end(2);
                                delete_btn.set_tooltip_text(Some("Delete palette"));

                                let path_clone = path.clone();
                                let on_delete_clone = on_delete.clone();
                                delete_btn.connect_clicked(move |btn| {
                                    let win = btn.root().and_then(|r| r.downcast::<adw::ApplicationWindow>().ok());
                                    match palette::delete_palette_image(&path_clone) {
                                        Ok(()) => {
                                            if let Some(ref w) = win {
                                                show_toast(w, "Palette deleted");
                                            }
                                            if let Some(ref cb) = on_delete_clone {
                                                cb();
                                            }
                                        }
                                        Err(e) => {
                                            if let Some(ref w) = win {
                                                show_toast(w, &format!("Failed to delete: {}", e));
                                            } else {
                                                eprintln!("Failed to delete palette: {}", e);
                                            }
                                        }
                                    }
                                });

                                overlay.add_overlay(&delete_btn);
                                flowbox.insert(&overlay, -1);
                            } else {
                                flowbox.insert(&image, -1);
                            }
                            paths.borrow_mut().push(path.clone());
                        }
                        Err(e) => {
                            eprintln!(
                                "Failed to load palette thumbnail '{}': {}",
                                path.display(),
                                e
                            );
                        }
                    }
                }
            })
        };

        // Show "no palettes" message if no categories exist at all
        if category_names.borrow().is_empty() {
            let label = gtk4::Label::new(Some(
                "No palette images found.",
            ));
            label.set_wrap(true);
            label.set_justify(gtk4::Justification::Center);
            label.add_css_class("dim-label");
            label.set_margin_top(12);
            label.set_margin_bottom(12);
            palette_flowbox.insert(&label, -1);
        } else {
            // Populate with the first category
            let names = category_names.borrow();
            let cats = all_categories.borrow();
            if let Some(images) = cats.get(&names[0]) {
                let is_custom = palette::is_custom_category(&names[0]);
                populate_flowbox(images, is_custom, None);
            }
        }

        // Palette group
        let palette_group = adw::PreferencesGroup::new();
        palette_group.set_title("Palette");
        palette_group.add(&category_row);

        // Wrap the scrollable FlowBox in a ListBoxRow so it sits inside the
        // PreferencesGroup's rounded rectangle together with the category dropdown
        let palette_listbox_row = gtk4::ListBoxRow::new();
        palette_listbox_row.set_child(Some(&palette_scroll));
        palette_listbox_row.set_activatable(false);
        palette_listbox_row.set_selectable(false);
        palette_group.add(&palette_listbox_row);

        // --- Smooth gradient toggle ---
        let smooth_switch = gtk4::Switch::new();
        smooth_switch.set_active(false);
        smooth_switch.set_valign(gtk4::Align::Center);
        let smooth_row = adw::ActionRow::builder()
            .title("Show as smooth gradients")
            .build();
        smooth_row.add_suffix(&smooth_switch);
        smooth_row.set_activatable_widget(Some(&smooth_switch));
        palette_group.add(&smooth_row);

        // --- Color picker buttons (4, one per color band) ---
        let color_dialog = gtk4::ColorDialog::new();
        color_dialog.set_with_alpha(false);

        let default_colors: [[f32; 3]; 4] = [
            [0.80, 0.33, 0.00],
            [0.93, 0.53, 0.07],
            [1.00, 0.75, 0.15],
            [1.00, 0.92, 0.35],
        ];

        let color_buttons: Vec<gtk4::ColorDialogButton> = default_colors
            .iter()
            .map(|c| {
                let rgba = gdk::RGBA::new(c[0], c[1], c[2], 1.0);
                let btn = gtk4::ColorDialogButton::new(Some(color_dialog.clone()));
                btn.set_rgba(&rgba);
                btn
            })
            .collect();

        let color_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        color_box.set_halign(gtk4::Align::Center);
        color_box.set_margin_top(8);
        color_box.set_margin_bottom(8);
        for btn in &color_buttons {
            color_box.append(btn);
        }

        let save_palette_button = gtk4::Button::from_icon_name("document-save-symbolic");
        save_palette_button.add_css_class("flat");
        save_palette_button.add_css_class("circular");
        save_palette_button.set_tooltip_text(Some("Save as custom palette"));
        color_box.append(&save_palette_button);

        let color_picker_row = gtk4::ListBoxRow::new();
        color_picker_row.set_child(Some(&color_box));
        color_picker_row.set_activatable(false);
        color_picker_row.set_selectable(false);
        palette_group.add(&color_picker_row);

        // =====================================================================
        // Shader parameter sliders
        // =====================================================================

        // --- Angle slider ---
        let angle_scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 0.0, 360.0, 1.0);
        angle_scale.set_value(45.0);
        angle_scale.set_hexpand(true);
        angle_scale.set_draw_value(true);
        angle_scale.set_value_pos(gtk4::PositionType::Right);

        let angle_row = adw::ActionRow::builder().title("Angle").build();
        angle_row.add_suffix(&angle_scale);

        // --- Scale slider ---
        let scale_scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 0.1, 5.0, 0.1);
        scale_scale.set_value(1.0);
        scale_scale.set_hexpand(true);
        scale_scale.set_draw_value(true);
        scale_scale.set_value_pos(gtk4::PositionType::Right);

        let scale_row = adw::ActionRow::builder().title("Scale").build();
        scale_row.add_suffix(&scale_scale);

        // --- Speed slider ---
        let speed_scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 0.0, 3.0, 0.1);
        speed_scale.set_value(1.0);
        speed_scale.set_hexpand(true);
        speed_scale.set_draw_value(true);
        speed_scale.set_value_pos(gtk4::PositionType::Right);

        let speed_row = adw::ActionRow::builder().title("Speed").build();
        speed_row.add_suffix(&speed_scale);

        // --- Blend slider ---
        let blend_scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 0.0, 1.0, 0.01);
        blend_scale.set_value(0.5);
        blend_scale.set_hexpand(true);
        blend_scale.set_draw_value(true);
        blend_scale.set_value_pos(gtk4::PositionType::Right);

        let blend_row = adw::ActionRow::builder().title("Blend").build();
        blend_row.add_suffix(&blend_scale);

        // Hint labels below the blend slider
        let blend_hints = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        let hard_label = gtk4::Label::new(Some("hard"));
        hard_label.add_css_class("dim-label");
        hard_label.add_css_class("caption");
        hard_label.set_halign(gtk4::Align::Start);
        hard_label.set_hexpand(true);
        let smooth_label = gtk4::Label::new(Some("smooth"));
        smooth_label.add_css_class("dim-label");
        smooth_label.add_css_class("caption");
        smooth_label.set_halign(gtk4::Align::End);
        blend_hints.append(&hard_label);
        blend_hints.append(&smooth_label);
        blend_hints.set_margin_start(12);
        blend_hints.set_margin_end(12);
        blend_hints.set_margin_bottom(4);

        let blend_hint_row = gtk4::ListBoxRow::new();
        blend_hint_row.set_child(Some(&blend_hints));
        blend_hint_row.set_activatable(false);
        blend_hint_row.set_selectable(false);

        // --- Center slider ---
        let center_scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, -1.0, 1.0, 0.01);
        center_scale.set_value(0.0);
        center_scale.set_hexpand(true);
        center_scale.set_draw_value(true);
        center_scale.set_value_pos(gtk4::PositionType::Right);

        let center_row = adw::ActionRow::builder().title("Center").build();
        center_row.add_suffix(&center_scale);
        let center_reset = gtk4::Button::from_icon_name("edit-clear-symbolic");
        center_reset.add_css_class("flat");
        center_reset.add_css_class("circular");
        center_reset.set_valign(gtk4::Align::Center);
        center_reset.set_tooltip_text(Some("Reset to 0"));
        center_row.add_suffix(&center_reset);

        // Hint labels below the center slider
        let center_hints = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        let center_left_label = gtk4::Label::new(Some("left"));
        center_left_label.add_css_class("dim-label");
        center_left_label.add_css_class("caption");
        center_left_label.set_halign(gtk4::Align::Start);
        center_left_label.set_hexpand(true);
        let center_right_label = gtk4::Label::new(Some("right"));
        center_right_label.add_css_class("dim-label");
        center_right_label.add_css_class("caption");
        center_right_label.set_halign(gtk4::Align::End);
        center_hints.append(&center_left_label);
        center_hints.append(&center_right_label);
        center_hints.set_margin_start(12);
        center_hints.set_margin_end(12);
        center_hints.set_margin_bottom(4);

        let center_hint_row = gtk4::ListBoxRow::new();
        center_hint_row.set_child(Some(&center_hints));
        center_hint_row.set_activatable(false);
        center_hint_row.set_selectable(false);

        // --- Controls group ---
        let controls_group = adw::PreferencesGroup::new();
        controls_group.set_title("Pattern");
        controls_group.add(&preset_row);
        controls_group.add(&angle_row);
        controls_group.add(&scale_row);
        controls_group.add(&speed_row);
        controls_group.add(&blend_row);
        controls_group.add(&blend_hint_row);
        controls_group.add(&center_row);
        controls_group.add(&center_hint_row);

        // =====================================================================
        // Effects section — fullscreen effects applied to all shaders
        // =====================================================================

        // --- Distortion type dropdown ---
        let distort_list = gtk4::StringList::new(&["None", "Swirl", "Ripple"]);
        let distort_row = adw::ComboRow::new();
        distort_row.set_title("Type");
        distort_row.set_model(Some(&distort_list));
        distort_row.set_selected(0);

        // --- Distortion strength slider ---
        let distort_strength_scale =
            gtk4::Scale::with_range(gtk4::Orientation::Horizontal, -10.0, 10.0, 0.1);
        distort_strength_scale.set_value(0.0);
        distort_strength_scale.set_hexpand(true);
        distort_strength_scale.set_draw_value(true);
        distort_strength_scale.set_value_pos(gtk4::PositionType::Right);
        let distort_strength_row = adw::ActionRow::builder().title("Strength").build();
        distort_strength_row.add_suffix(&distort_strength_scale);
        let distort_strength_reset = gtk4::Button::from_icon_name("edit-clear-symbolic");
        distort_strength_reset.add_css_class("flat");
        distort_strength_reset.add_css_class("circular");
        distort_strength_reset.set_valign(gtk4::Align::Center);
        distort_strength_reset.set_tooltip_text(Some("Reset to 0"));
        distort_strength_row.add_suffix(&distort_strength_reset);
        distort_strength_row.set_visible(false); // hidden when "None"

        // Hint labels below the strength slider (only for Swirl)
        let distort_strength_hints = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        let left_label = gtk4::Label::new(Some("left"));
        left_label.add_css_class("dim-label");
        left_label.add_css_class("caption");
        left_label.set_halign(gtk4::Align::Start);
        left_label.set_hexpand(true);
        let right_label = gtk4::Label::new(Some("right"));
        right_label.add_css_class("dim-label");
        right_label.add_css_class("caption");
        right_label.set_halign(gtk4::Align::End);
        distort_strength_hints.append(&left_label);
        distort_strength_hints.append(&right_label);
        distort_strength_hints.set_margin_start(12);
        distort_strength_hints.set_margin_end(12);
        distort_strength_hints.set_margin_bottom(4);

        let distort_strength_hint_row = gtk4::ListBoxRow::new();
        distort_strength_hint_row.set_child(Some(&distort_strength_hints));
        distort_strength_hint_row.set_activatable(false);
        distort_strength_hint_row.set_selectable(false);
        distort_strength_hint_row.set_visible(false); // hidden when "None"

        // --- Ripple frequency slider ---
        let ripple_freq_scale =
            gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 1.0, 30.0, 0.5);
        ripple_freq_scale.set_value(15.0);
        ripple_freq_scale.set_hexpand(true);
        ripple_freq_scale.set_draw_value(true);
        ripple_freq_scale.set_value_pos(gtk4::PositionType::Right);

        let ripple_freq_row = adw::ActionRow::builder().title("Frequency").build();
        ripple_freq_row.add_suffix(&ripple_freq_scale);
        ripple_freq_row.set_visible(false); // hidden unless "Ripple"

        // Hint labels below the frequency slider
        let ripple_freq_hints = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        let sparse_label = gtk4::Label::new(Some("sparse"));
        sparse_label.add_css_class("dim-label");
        sparse_label.add_css_class("caption");
        sparse_label.set_halign(gtk4::Align::Start);
        sparse_label.set_hexpand(true);
        let dense_label = gtk4::Label::new(Some("dense"));
        dense_label.add_css_class("dim-label");
        dense_label.add_css_class("caption");
        dense_label.set_halign(gtk4::Align::End);
        ripple_freq_hints.append(&sparse_label);
        ripple_freq_hints.append(&dense_label);
        ripple_freq_hints.set_margin_start(12);
        ripple_freq_hints.set_margin_end(12);
        ripple_freq_hints.set_margin_bottom(4);

        let ripple_freq_hint_row = gtk4::ListBoxRow::new();
        ripple_freq_hint_row.set_child(Some(&ripple_freq_hints));
        ripple_freq_hint_row.set_activatable(false);
        ripple_freq_hint_row.set_selectable(false);
        ripple_freq_hint_row.set_visible(false); // hidden unless "Ripple"

        // --- Noise slider ---
        let noise_scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, -1.0, 1.0, 0.01);
        noise_scale.set_value(0.0);
        noise_scale.set_hexpand(true);
        noise_scale.set_draw_value(true);
        noise_scale.set_value_pos(gtk4::PositionType::Right);

        let noise_row = adw::ActionRow::builder().title("Noise").build();
        noise_row.add_suffix(&noise_scale);
        let noise_reset = gtk4::Button::from_icon_name("edit-clear-symbolic");
        noise_reset.add_css_class("flat");
        noise_reset.add_css_class("circular");
        noise_reset.set_valign(gtk4::Align::Center);
        noise_reset.set_tooltip_text(Some("Reset to 0"));
        noise_row.add_suffix(&noise_reset);

        // Hint labels below the noise slider
        let noise_hints = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        let darker_label = gtk4::Label::new(Some("darker"));
        darker_label.add_css_class("dim-label");
        darker_label.add_css_class("caption");
        darker_label.set_halign(gtk4::Align::Start);
        darker_label.set_hexpand(true);
        let lighter_label = gtk4::Label::new(Some("lighter"));
        lighter_label.add_css_class("dim-label");
        lighter_label.add_css_class("caption");
        lighter_label.set_halign(gtk4::Align::End);
        noise_hints.append(&darker_label);
        noise_hints.append(&lighter_label);
        noise_hints.set_margin_start(12);
        noise_hints.set_margin_end(12);
        noise_hints.set_margin_bottom(4);

        let noise_hint_row = gtk4::ListBoxRow::new();
        noise_hint_row.set_child(Some(&noise_hints));
        noise_hint_row.set_activatable(false);
        noise_hint_row.set_selectable(false);

        // --- Dither toggle ---
        let dither_switch = gtk4::Switch::new();
        dither_switch.set_active(false);
        dither_switch.set_valign(gtk4::Align::Center);

        let dither_row = adw::ActionRow::builder().title("Dither").build();
        dither_row.add_suffix(&dither_switch);
        dither_row.set_activatable_widget(Some(&dither_switch));

        let distortion_group = adw::PreferencesGroup::new();
        distortion_group.set_title("Distortion");
        distortion_group.add(&distort_row);
        distortion_group.add(&distort_strength_row);
        distortion_group.add(&distort_strength_hint_row);
        distortion_group.add(&ripple_freq_row);
        distortion_group.add(&ripple_freq_hint_row);

        let effects_group = adw::PreferencesGroup::new();
        effects_group.set_title("Effects");
        effects_group.add(&noise_row);
        effects_group.add(&noise_hint_row);
        effects_group.add(&dither_row);

        // =====================================================================
        // Lighting section
        // =====================================================================

        // --- Lighting type dropdown ---
        let lighting_list = gtk4::StringList::new(&["None", "Bevel", "Gradient", "Vignette"]);
        let lighting_row = adw::ComboRow::new();
        lighting_row.set_title("Type");
        lighting_row.set_model(Some(&lighting_list));
        lighting_row.set_selected(0);

        // --- Lighting strength slider ---
        let light_strength_scale =
            gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 0.0, 1.0, 0.01);
        light_strength_scale.set_value(0.0);
        light_strength_scale.set_hexpand(true);
        light_strength_scale.set_draw_value(true);
        light_strength_scale.set_value_pos(gtk4::PositionType::Right);

        let light_strength_row = adw::ActionRow::builder().title("Strength").build();
        light_strength_row.add_suffix(&light_strength_scale);
        light_strength_row.set_visible(false); // hidden when "None"

        // Hint labels below the strength slider
        let light_strength_hints = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        let ls_off_label = gtk4::Label::new(Some("off"));
        ls_off_label.add_css_class("dim-label");
        ls_off_label.add_css_class("caption");
        ls_off_label.set_halign(gtk4::Align::Start);
        ls_off_label.set_hexpand(true);
        let ls_strong_label = gtk4::Label::new(Some("strong"));
        ls_strong_label.add_css_class("dim-label");
        ls_strong_label.add_css_class("caption");
        ls_strong_label.set_halign(gtk4::Align::End);
        light_strength_hints.append(&ls_off_label);
        light_strength_hints.append(&ls_strong_label);
        light_strength_hints.set_margin_start(12);
        light_strength_hints.set_margin_end(12);
        light_strength_hints.set_margin_bottom(4);

        let light_strength_hint_row = gtk4::ListBoxRow::new();
        light_strength_hint_row.set_child(Some(&light_strength_hints));
        light_strength_hint_row.set_activatable(false);
        light_strength_hint_row.set_selectable(false);
        light_strength_hint_row.set_visible(false); // hidden when "None"

        // --- Bevel width slider ---
        let bevel_width_scale =
            gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 0.01, 0.15, 0.01);
        bevel_width_scale.set_value(0.05);
        bevel_width_scale.set_hexpand(true);
        bevel_width_scale.set_draw_value(true);
        bevel_width_scale.set_value_pos(gtk4::PositionType::Right);

        let bevel_width_row = adw::ActionRow::builder().title("Width").build();
        bevel_width_row.add_suffix(&bevel_width_scale);
        bevel_width_row.set_visible(false); // only visible for "Bevel"

        // Hint labels below the width slider
        let bevel_width_hints = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        let bw_thin_label = gtk4::Label::new(Some("thin"));
        bw_thin_label.add_css_class("dim-label");
        bw_thin_label.add_css_class("caption");
        bw_thin_label.set_halign(gtk4::Align::Start);
        bw_thin_label.set_hexpand(true);
        let bw_wide_label = gtk4::Label::new(Some("wide"));
        bw_wide_label.add_css_class("dim-label");
        bw_wide_label.add_css_class("caption");
        bw_wide_label.set_halign(gtk4::Align::End);
        bevel_width_hints.append(&bw_thin_label);
        bevel_width_hints.append(&bw_wide_label);
        bevel_width_hints.set_margin_start(12);
        bevel_width_hints.set_margin_end(12);
        bevel_width_hints.set_margin_bottom(4);

        let bevel_width_hint_row = gtk4::ListBoxRow::new();
        bevel_width_hint_row.set_child(Some(&bevel_width_hints));
        bevel_width_hint_row.set_activatable(false);
        bevel_width_hint_row.set_selectable(false);
        bevel_width_hint_row.set_visible(false); // only visible for "Bevel"

        // --- Light angle slider ---
        let light_angle_scale =
            gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 0.0, 360.0, 1.0);
        light_angle_scale.set_value(45.0);
        light_angle_scale.set_hexpand(true);
        light_angle_scale.set_draw_value(true);
        light_angle_scale.set_value_pos(gtk4::PositionType::Right);

        let light_angle_row = adw::ActionRow::builder().title("Angle").build();
        light_angle_row.add_suffix(&light_angle_scale);
        light_angle_row.set_visible(false); // only visible for "Gradient"

        let lighting_group = adw::PreferencesGroup::new();
        lighting_group.set_title("Lighting");
        lighting_group.add(&lighting_row);
        lighting_group.add(&light_strength_row);
        lighting_group.add(&light_strength_hint_row);
        lighting_group.add(&bevel_width_row);
        lighting_group.add(&bevel_width_hint_row);
        lighting_group.add(&light_angle_row);

        // =====================================================================
        // Export section
        // =====================================================================

        // Detect the largest monitor's resolution for the "Display" option
        let display_dims: (u32, u32) = gdk::Display::default()
            .and_then(|display| {
                let monitors = display.monitors();
                let mut best: Option<(i32, i32)> = None;
                for i in 0..monitors.n_items() {
                    if let Some(obj) = monitors.item(i) {
                        if let Some(mon) = obj.downcast_ref::<gdk::Monitor>() {
                            let geom = mon.geometry();
                            let pixels = geom.width() * geom.height();
                            if best.map_or(true, |(bw, bh)| pixels > bw * bh) {
                                best = Some((geom.width(), geom.height()));
                            }
                        }
                    }
                }
                best
            })
            .map(|(w, h)| (w as u32, h as u32))
            .unwrap_or((1920, 1080));

        let display_label = format!("Display ({}x{})", display_dims.0, display_dims.1);
        let resolution_list = gtk4::StringList::new(&[
            &display_label,
            "1080p (1920x1080)",
            "1440p (2560x1440)",
            "4K (3840x2160)",
        ]);
        let resolution_row = adw::ComboRow::new();
        resolution_row.set_title("Resolution");
        resolution_row.set_model(Some(&resolution_list));
        resolution_row.set_selected(0); // Default to Display

        let export_button = gtk4::Button::with_label("Export");
        export_button.set_tooltip_text(Some("Export image (Ctrl+E)"));

        let set_wallpaper_button = gtk4::Button::with_label("Set as Wallpaper");
        set_wallpaper_button.add_css_class("suggested-action");
        set_wallpaper_button.set_tooltip_text(Some("Set as desktop wallpaper (Ctrl+Shift+W)"));

        let button_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        button_box.set_halign(gtk4::Align::Center);
        button_box.set_margin_top(8);
        button_box.set_margin_bottom(8);
        button_box.append(&export_button);
        button_box.append(&set_wallpaper_button);

        let export_group = adw::PreferencesGroup::new();
        export_group.set_title("Export");
        export_group.add(&resolution_row);

        // =====================================================================
        // Layout — two columns: controls (left), preview + export (right)
        // =====================================================================

        // Left column: palette + shader controls (scrollable)
        let left_box = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        left_box.set_margin_start(12);
        left_box.set_margin_end(6);
        left_box.set_margin_top(0);
        left_box.set_margin_bottom(12);
        left_box.append(&palette_group);
        left_box.append(&controls_group);
        left_box.append(&distortion_group);

        let left_scroll = gtk4::ScrolledWindow::new();
        left_scroll.set_child(Some(&left_box));
        left_scroll.set_vexpand(true);
        left_scroll.set_hscrollbar_policy(gtk4::PolicyType::Never);
        left_scroll.set_propagate_natural_height(true);
        left_scroll.set_min_content_width(320);

        // Right column: preview + export
        let right_box = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        right_box.set_margin_start(6);
        right_box.set_margin_end(12);
        right_box.set_margin_top(0);
        right_box.set_margin_bottom(12);
        right_box.set_hexpand(true);
        right_box.append(&preview_group);
        right_box.append(&effects_group);
        right_box.append(&lighting_group);
        right_box.append(&export_group);
        right_box.append(&button_box);

        // Two-column horizontal layout
        let columns_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        columns_box.set_vexpand(true);
        columns_box.append(&left_scroll);
        columns_box.append(&right_box);

        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&header);
        toolbar_view.set_content(Some(&columns_box));

        let toast_overlay = adw::ToastOverlay::new();
        toast_overlay.set_child(Some(&toolbar_view));

        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("Wallrus")
            .default_width(1300)
            .default_height(900)
            .content(&toast_overlay)
            .build();

        // --- Tick callback for continuous rendering ---
        let gl_area_tick = gl_area.clone();
        gl_area.add_tick_callback(move |_widget, _clock| {
            gl_area_tick.queue_render();
            glib::ControlFlow::Continue
        });

        // =====================================================================
        // Signal connections
        // =====================================================================

        // --- Category dropdown: repopulate FlowBox when category changes ---
        // Also used by save/delete to refresh the current view.

        // Shared refresh: reloads categories from disk and repopulates the current view.
        let refresh_current_category: Rc<RefCell<Option<Rc<dyn Fn()>>>> =
            Rc::new(RefCell::new(None));

        {
            let all_cats = all_categories.clone();
            let cat_names = category_names.clone();
            let populate = populate_flowbox.clone();
            let category_row_ref = category_row.clone();
            let refresh_ref = refresh_current_category.clone();

            let do_refresh: Rc<dyn Fn()> = Rc::new(move || {
                // Reload from disk
                let new_cats = palette::list_palette_categories();
                let new_names: Vec<String> = new_cats.keys().cloned().collect();

                // Capture current selection BEFORE replacing the model (set_model resets index to 0)
                let prev_idx = category_row_ref.selected() as usize;
                let prev_name = cat_names.borrow().get(prev_idx).cloned();

                // Update the dropdown model
                let str_refs: Vec<&str> = new_names.iter().map(|s| s.as_str()).collect();
                let new_model = gtk4::StringList::new(&str_refs);
                category_row_ref.set_model(Some(&new_model));

                *cat_names.borrow_mut() = new_names.clone();
                *all_cats.borrow_mut() = new_cats;

                // Find the previously selected category in the new list, or default to 0
                let new_idx = prev_name
                    .as_ref()
                    .and_then(|name| new_names.iter().position(|n| n == name))
                    .unwrap_or(0);

                if !new_names.is_empty() {
                    // Repopulate flowbox for the selected category
                    let cats = all_cats.borrow();
                    if let Some(images) = cats.get(&new_names[new_idx]) {
                        let is_custom = palette::is_custom_category(&new_names[new_idx]);
                        populate(images, is_custom, Some(refresh_ref.borrow().as_ref().unwrap().clone()));
                    }
                    // Set selected after populating to avoid double-trigger
                    category_row_ref.set_selected(new_idx as u32);
                }
            });

            *refresh_current_category.borrow_mut() = Some(do_refresh.clone());

            // Connect category dropdown change
            let all_cats2 = all_categories.clone();
            let cat_names2 = category_names.clone();
            let populate2 = populate_flowbox.clone();
            let refresh_for_delete = refresh_current_category.clone();
            category_row.connect_selected_notify(move |combo| {
                let idx = combo.selected() as usize;
                let names = cat_names2.borrow();
                if let Some(name) = names.get(idx) {
                    let cats = all_cats2.borrow();
                    if let Some(images) = cats.get(name) {
                        let is_custom = palette::is_custom_category(name);
                        let on_delete = if is_custom {
                            Some(refresh_for_delete.borrow().as_ref().unwrap().clone())
                        } else {
                            None
                        };
                        populate2(images, is_custom, on_delete);
                    }
                }
            });

            // Connect smooth gradient toggle change
            let all_cats3 = all_categories.clone();
            let cat_names3 = category_names.clone();
            let populate3 = populate_flowbox.clone();
            let smooth_state = smooth_gradient.clone();
            let category_row_ref2 = category_row.clone();
            let refresh_for_smooth = refresh_current_category.clone();
            smooth_switch.connect_active_notify(move |switch| {
                smooth_state.set(switch.is_active());
                let idx = category_row_ref2.selected() as usize;
                let names = cat_names3.borrow();
                if let Some(name) = names.get(idx) {
                    let cats = all_cats3.borrow();
                    if let Some(images) = cats.get(name) {
                        let is_custom = palette::is_custom_category(name);
                        let on_delete = if is_custom {
                            Some(refresh_for_smooth.borrow().as_ref().unwrap().clone())
                        } else {
                            None
                        };
                        populate3(images, is_custom, on_delete);
                    }
                }
            });
        }

        // --- Save palette button handler ---
        {
            let color_btns = color_buttons.clone();
            let window_ref = window.clone();
            let refresh = refresh_current_category.clone();
            let cat_names_ref = category_names.clone();
            let category_row_ref = category_row.clone();
            save_palette_button.connect_clicked(move |_| {
                let colors: [[f32; 3]; 4] = [
                    {
                        let c = color_btns[0].rgba();
                        [c.red(), c.green(), c.blue()]
                    },
                    {
                        let c = color_btns[1].rgba();
                        [c.red(), c.green(), c.blue()]
                    },
                    {
                        let c = color_btns[2].rgba();
                        [c.red(), c.green(), c.blue()]
                    },
                    {
                        let c = color_btns[3].rgba();
                        [c.red(), c.green(), c.blue()]
                    },
                ];

                match palette::save_palette_image(&colors) {
                    Ok(_) => {
                        // Refresh categories and switch to Custom
                        if let Some(ref cb) = *refresh.borrow() {
                            cb();
                        }
                        // Switch to Custom category
                        let names = cat_names_ref.borrow();
                        if let Some(idx) = names.iter().position(|n| palette::is_custom_category(n)) {
                            category_row_ref.set_selected(idx as u32);
                        }
                        show_toast(&window_ref, "Palette saved");
                    }
                    Err(e) => {
                        show_toast(&window_ref, &format!("Failed to save palette: {}", e));
                    }
                }
            });
        }

        // --- Palette selection: extract colors from selected palette image ---
        {
            let paths = palette_paths.clone();
            let state = state.clone();
            let cb0 = color_buttons[0].clone();
            let cb1 = color_buttons[1].clone();
            let cb2 = color_buttons[2].clone();
            let cb3 = color_buttons[3].clone();
            palette_flowbox.connect_child_activated(move |_flowbox, child| {
                let idx = child.index() as usize;
                let paths_ref = paths.borrow();
                if let Some(path) = paths_ref.get(idx) {
                    match palette::extract_colors_from_image(path) {
                        Ok(colors) => {
                            if let Some(ref mut renderer) = *state.borrow_mut() {
                                renderer.color1 = colors[0];
                                renderer.color2 = colors[1];
                                renderer.color3 = colors[2];
                                renderer.color4 = colors[3];
                            }
                            // Update color picker buttons to reflect extracted colors
                            cb0.set_rgba(&gdk::RGBA::new(
                                colors[0][0],
                                colors[0][1],
                                colors[0][2],
                                1.0,
                            ));
                            cb1.set_rgba(&gdk::RGBA::new(
                                colors[1][0],
                                colors[1][1],
                                colors[1][2],
                                1.0,
                            ));
                            cb2.set_rgba(&gdk::RGBA::new(
                                colors[2][0],
                                colors[2][1],
                                colors[2][2],
                                1.0,
                            ));
                            cb3.set_rgba(&gdk::RGBA::new(
                                colors[3][0],
                                colors[3][1],
                                colors[3][2],
                                1.0,
                            ));
                        }
                        Err(e) => {
                            eprintln!("Failed to extract colors from '{}': {}", path.display(), e);
                        }
                    }
                }
            });
        }

        // --- Color picker manual change handlers ---
        {
            let state = state.clone();
            color_buttons[0].connect_rgba_notify(move |btn| {
                let rgba = btn.rgba();
                if let Some(ref mut renderer) = *state.borrow_mut() {
                    renderer.color1 = [rgba.red(), rgba.green(), rgba.blue()];
                }
            });
        }
        {
            let state = state.clone();
            color_buttons[1].connect_rgba_notify(move |btn| {
                let rgba = btn.rgba();
                if let Some(ref mut renderer) = *state.borrow_mut() {
                    renderer.color2 = [rgba.red(), rgba.green(), rgba.blue()];
                }
            });
        }
        {
            let state = state.clone();
            color_buttons[2].connect_rgba_notify(move |btn| {
                let rgba = btn.rgba();
                if let Some(ref mut renderer) = *state.borrow_mut() {
                    renderer.color3 = [rgba.red(), rgba.green(), rgba.blue()];
                }
            });
        }
        {
            let state = state.clone();
            color_buttons[3].connect_rgba_notify(move |btn| {
                let rgba = btn.rgba();
                if let Some(ref mut renderer) = *state.borrow_mut() {
                    renderer.color4 = [rgba.red(), rgba.green(), rgba.blue()];
                }
            });
        }

        // --- Update visibility of shader controls based on preset ---
        let update_control_visibility = {
            let angle_row = angle_row.clone();
            let scale_row = scale_row.clone();
            let scale_scale = scale_scale.clone();
            let speed_row = speed_row.clone();
            let speed_scale = speed_scale.clone();
            let center_row = center_row.clone();
            let center_hint_row = center_hint_row.clone();
            let center_scale = center_scale.clone();
            move |name: &str| {
                let controls = shader_presets::controls_for(name);
                angle_row.set_visible(controls.has_angle);
                scale_row.set_visible(controls.has_scale);
                speed_row.set_visible(controls.has_speed);
                center_row.set_visible(controls.has_center);
                center_hint_row.set_visible(controls.has_center);
                // Reset center slider to default when switching presets
                center_scale.set_value(0.0);
                // Update scale slider range per preset
                let (smin, smax, sstep, sdefault) = controls.scale_range;
                scale_scale.set_range(smin, smax);
                scale_scale.set_increments(sstep, sstep * 10.0);
                scale_scale.set_value(sdefault);
                // Update speed/time slider label and range per preset
                speed_row.set_title(controls.speed_label);
                let (min, max, step, default) = controls.speed_range;
                speed_scale.set_range(min, max);
                speed_scale.set_increments(step, step * 10.0);
                speed_scale.set_value(default);
            }
        };

        update_control_visibility("Bars");

        // --- Preset change ---
        {
            let state = state.clone();
            let gl_area = gl_area.clone();
            preset_row.connect_selected_notify(move |combo| {
                let idx = combo.selected();
                let names = shader_presets::preset_names();
                if let Some(name) = names.get(idx as usize) {
                    update_control_visibility(name);
                    if let Some(ref mut renderer) = *state.borrow_mut() {
                        gl_area.make_current();
                        if let Err(e) = renderer.load_preset(name) {
                            eprintln!("Failed to load preset '{}': {}", name, e);
                        }
                    }
                }
            });
        }

        // --- Angle change ---
        {
            let state = state.clone();
            angle_scale.connect_value_changed(move |scale| {
                let radians = (scale.value() as f32).to_radians();
                if let Some(ref mut renderer) = *state.borrow_mut() {
                    renderer.angle = radians;
                }
            });
        }

        // --- Scale change ---
        {
            let state = state.clone();
            scale_scale.connect_value_changed(move |scale| {
                if let Some(ref mut renderer) = *state.borrow_mut() {
                    renderer.scale = scale.value() as f32;
                }
            });
        }

        // --- Speed change ---
        {
            let state = state.clone();
            speed_scale.connect_value_changed(move |scale| {
                if let Some(ref mut renderer) = *state.borrow_mut() {
                    renderer.speed = scale.value() as f32;
                }
            });
        }

        // --- Blend change ---
        {
            let state = state.clone();
            blend_scale.connect_value_changed(move |scale| {
                if let Some(ref mut renderer) = *state.borrow_mut() {
                    renderer.blend = scale.value() as f32;
                }
            });
        }

        // --- Center change ---
        {
            let state = state.clone();
            center_scale.connect_value_changed(move |scale| {
                if let Some(ref mut renderer) = *state.borrow_mut() {
                    renderer.center = scale.value() as f32;
                }
            });
        }

        // --- Center reset ---
        {
            let center_scale = center_scale.clone();
            center_reset.connect_clicked(move |_| {
                center_scale.set_value(0.0);
            });
        }

        // --- Distortion type change ---
        {
            let state = state.clone();
            let distort_strength_row = distort_strength_row.clone();
            let distort_strength_scale = distort_strength_scale.clone();
            let distort_strength_hint_row = distort_strength_hint_row.clone();
            let ripple_freq_row = ripple_freq_row.clone();
            let ripple_freq_hint_row = ripple_freq_hint_row.clone();
            distort_row.connect_selected_notify(move |combo| {
                let idx = combo.selected();
                let distort_type = idx as i32; // 0=None, 1=Swirl, 2=Ripple
                if let Some(ref mut renderer) = *state.borrow_mut() {
                    renderer.distort_type = distort_type;
                    if distort_type == 0 {
                        renderer.distort_strength = 0.0;
                    }
                }
                // Visibility
                distort_strength_row.set_visible(distort_type != 0);
                if distort_type == 0 {
                    distort_strength_scale.set_value(0.0);
                }
                distort_strength_hint_row.set_visible(distort_type == 1); // Swirl only
                ripple_freq_row.set_visible(distort_type == 2); // Ripple only
                ripple_freq_hint_row.set_visible(distort_type == 2);
            });
        }

        // --- Distortion strength change ---
        {
            let state = state.clone();
            distort_strength_scale.connect_value_changed(move |scale| {
                if let Some(ref mut renderer) = *state.borrow_mut() {
                    renderer.distort_strength = scale.value() as f32;
                }
            });
        }

        // --- Distortion strength reset ---
        {
            let distort_strength_scale = distort_strength_scale.clone();
            distort_strength_reset.connect_clicked(move |_| {
                distort_strength_scale.set_value(0.0);
            });
        }

        // --- Ripple frequency change ---
        {
            let state = state.clone();
            ripple_freq_scale.connect_value_changed(move |scale| {
                if let Some(ref mut renderer) = *state.borrow_mut() {
                    renderer.ripple_freq = scale.value() as f32;
                }
            });
        }

        // --- Noise change ---
        {
            let state = state.clone();
            noise_scale.connect_value_changed(move |scale| {
                if let Some(ref mut renderer) = *state.borrow_mut() {
                    renderer.noise = scale.value() as f32;
                }
            });
        }

        // --- Noise reset ---
        {
            let noise_scale = noise_scale.clone();
            noise_reset.connect_clicked(move |_| {
                noise_scale.set_value(0.0);
            });
        }

        // --- Dither change ---
        {
            let state = state.clone();
            dither_switch.connect_active_notify(move |switch| {
                if let Some(ref mut renderer) = *state.borrow_mut() {
                    renderer.dither = if switch.is_active() { 1.0 } else { 0.0 };
                }
            });
        }

        // --- Lighting type change ---
        {
            let state = state.clone();
            let light_strength_row = light_strength_row.clone();
            let light_strength_hint_row = light_strength_hint_row.clone();
            let bevel_width_row = bevel_width_row.clone();
            let bevel_width_hint_row = bevel_width_hint_row.clone();
            let light_angle_row = light_angle_row.clone();
            lighting_row.connect_selected_notify(move |combo| {
                let idx = combo.selected() as i32; // 0=None, 1=Bevel, 2=Gradient, 3=Vignette
                if let Some(ref mut renderer) = *state.borrow_mut() {
                    renderer.lighting_type = idx;
                }
                // Visibility logic per the specification table
                let show_strength = idx != 0;
                light_strength_row.set_visible(show_strength);
                light_strength_hint_row.set_visible(show_strength);
                bevel_width_row.set_visible(idx == 1);
                bevel_width_hint_row.set_visible(idx == 1);
                light_angle_row.set_visible(idx == 2);
            });
        }

        // --- Light strength change ---
        {
            let state = state.clone();
            light_strength_scale.connect_value_changed(move |scale| {
                if let Some(ref mut renderer) = *state.borrow_mut() {
                    renderer.light_strength = scale.value() as f32;
                }
            });
        }

        // --- Bevel width change ---
        {
            let state = state.clone();
            bevel_width_scale.connect_value_changed(move |scale| {
                if let Some(ref mut renderer) = *state.borrow_mut() {
                    renderer.bevel_width = scale.value() as f32;
                }
            });
        }

        // --- Light angle change ---
        {
            let state = state.clone();
            light_angle_scale.connect_value_changed(move |scale| {
                // Convert degrees to radians, with offset so 0° = light from top
                let degrees = scale.value() as f32;
                let radians = (degrees - 90.0).to_radians();
                if let Some(ref mut renderer) = *state.borrow_mut() {
                    renderer.light_angle = radians;
                }
            });
        }

        // =====================================================================
        // Export handlers
        // =====================================================================

        let make_export_handler = {
            let state = state.clone();
            let resolution_row = resolution_row.clone();
            let gl_area = gl_area.clone();
            let window_ref = window.clone();
            move |_button: &gtk4::Button| {
                let resolution =
                    ExportResolution::from_index(resolution_row.selected(), display_dims);
                let (w, h) = resolution.dimensions();

                gl_area.make_current();

                let pixels = {
                    let state_ref = state.borrow();
                    match state_ref.as_ref() {
                        Some(renderer) => renderer.render_to_pixels(w as i32, h as i32),
                        None => {
                            show_toast(&window_ref, "Renderer not initialized");
                            return;
                        }
                    }
                };

                let preset_name = {
                    let state_ref = state.borrow();
                    state_ref
                        .as_ref()
                        .map(|r| r.current_preset.clone())
                        .unwrap_or_else(|| "wallpaper".to_string())
                };

                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                // Default filename uses JPEG; user can switch filter to PNG in the dialog
                let filename = format!(
                    "wallrus_{}_{}.jpg",
                    preset_name.to_lowercase(),
                    timestamp,
                );

                let dialog = gtk4::FileDialog::new();
                dialog.set_initial_name(Some(&filename));

                // Default to the user's Pictures folder (portal-safe hint)
                if let Some(pictures_dir) = glib::user_special_dir(glib::UserDirectory::Pictures) {
                    dialog.set_initial_folder(Some(&gio::File::for_path(pictures_dir)));
                }

                // Offer both PNG and JPEG filters; user picks in the dialog
                let png_filter = gtk4::FileFilter::new();
                png_filter.set_name(Some("PNG images"));
                png_filter.add_mime_type("image/png");
                png_filter.add_suffix("png");

                let jpeg_filter = gtk4::FileFilter::new();
                jpeg_filter.set_name(Some("JPEG images"));
                jpeg_filter.add_mime_type("image/jpeg");
                jpeg_filter.add_suffix("jpg");
                jpeg_filter.add_suffix("jpeg");

                let filters = gio::ListStore::new::<gtk4::FileFilter>();
                filters.append(&jpeg_filter);
                filters.append(&png_filter);
                dialog.set_filters(Some(&filters));
                dialog.set_default_filter(Some(&jpeg_filter));

                let window_clone = window_ref.clone();
                dialog.save(
                    Some(&window_ref),
                    None::<&gio::Cancellable>,
                    move |result| match result {
                        Ok(file) => {
                            if let Some(path) = file.path() {
                                // Determine format from the file extension
                                let format = ExportFormat::from_extension(
                                    path.extension()
                                        .and_then(|e| e.to_str())
                                        .unwrap_or("jpg"),
                                );
                                match export::save_pixels(&pixels, w, h, &path, format) {
                                    Ok(()) => {
                                        show_toast(
                                            &window_clone,
                                            &format!("Saved to {}", path.display()),
                                        );
                                    }
                                    Err(e) => {
                                        show_toast(
                                            &window_clone,
                                            &format!("Export failed: {}", e),
                                        );
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            // User cancelled — not an error worth reporting
                            if !e.matches(gio::IOErrorEnum::Cancelled) {
                                show_toast(&window_clone, &format!("Export failed: {}", e));
                            }
                        }
                    },
                );
            }
        };

        export_button.connect_clicked(make_export_handler);

        // --- Set as wallpaper handler ---
        // Shared logic for all wallpaper modes (Both / LightOnly / DarkOnly).
        // --- Set as wallpaper handler (uses XDG Desktop Portal) ---
        {
            let state = state.clone();
            let resolution_row = resolution_row.clone();
            let gl_area = gl_area.clone();
            let window_ref = window.clone();
            set_wallpaper_button.connect_clicked(move |_| {
                let resolution =
                    ExportResolution::from_index(resolution_row.selected(), display_dims);
                let (w, h) = resolution.dimensions();

                gl_area.make_current();

                let pixels = {
                    let state_ref = state.borrow();
                    match state_ref.as_ref() {
                        Some(renderer) => renderer.render_to_pixels(w as i32, h as i32),
                        None => {
                            show_toast(&window_ref, "Renderer not initialized");
                            return;
                        }
                    }
                };

                // Save to a temporary file and hand it to the portal
                let tmp_path = std::env::temp_dir().join("wallrus_wallpaper.png");
                if let Err(e) = export::save_pixels(&pixels, w, h, &tmp_path, ExportFormat::Png) {
                    show_toast(&window_ref, &format!("Failed to save: {}", e));
                    return;
                }

                let window_ref2 = window_ref.clone();
                glib::MainContext::default().spawn_local(async move {
                    match wallpaper::set_wallpaper(&tmp_path).await {
                        Ok(()) => show_toast(&window_ref2, "Wallpaper set!"),
                        Err(e) => show_toast(&window_ref2, &format!("Failed: {}", e)),
                    }
                });
            });
        }

        // =====================================================================
        // Keyboard shortcuts via GActions
        // =====================================================================

        let action_export = gio::SimpleAction::new("export", None);
        {
            let btn = export_button.clone();
            action_export.connect_activate(move |_, _| btn.emit_clicked());
        }
        window.add_action(&action_export);

        let action_set_wallpaper = gio::SimpleAction::new("set-wallpaper", None);
        {
            let btn = set_wallpaper_button.clone();
            action_set_wallpaper.connect_activate(move |_, _| btn.emit_clicked());
        }
        window.add_action(&action_set_wallpaper);

        app.set_accels_for_action("win.export", &["<Control>e"]);
        app.set_accels_for_action("win.set-wallpaper", &["<Control><Shift>w"]);

        // --- About dialog action ---
        let action_about = gio::SimpleAction::new("show-about", None);
        {
            let window_ref = window.clone();
            action_about.connect_activate(move |_, _| {
                let about = adw::AboutWindow::builder()
                    .application_name("Wallrus")
                    .application_icon("io.github.megakode.Wallrus")
                    .developer_name("Peter Boné")
                    .version(env!("CARGO_PKG_VERSION"))
                    .website("https://github.com/megakode/wallrus")
                    .issue_url("https://github.com/megakode/wallrus/issues")
                    .license_type(gtk4::License::Gpl30)
                    .copyright("© 2026 Peter Boné")
                    .developers(vec!["Peter Boné"])
                    .transient_for(&window_ref)
                    .modal(true)
                    .build();
                about.present();
            });
        }
        window.add_action(&action_about);

        window
    }
}

/// Show a toast notification on the window.
/// Expects the window content to be a ToastOverlay (set up during construction).
fn show_toast(window: &adw::ApplicationWindow, message: &str) {
    let toast = adw::Toast::new(message);
    toast.set_timeout(3);

    if let Some(content) = window.content() {
        if let Some(overlay) = content.downcast_ref::<adw::ToastOverlay>() {
            overlay.add_toast(toast);
        } else {
            eprintln!("Toast: {}", message);
        }
    }
}
