use gtk4::gdk;
use gtk4::gdk_pixbuf;
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::RefCell;
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
        let all_categories = palette::list_palette_categories();
        let category_names: Vec<String> = all_categories.keys().cloned().collect();

        // Category dropdown
        let category_str_refs: Vec<&str> = category_names.iter().map(|s| s.as_str()).collect();
        let category_string_list = gtk4::StringList::new(&category_str_refs);
        let category_row = adw::ComboRow::new();
        category_row.set_title("Category");
        category_row.set_model(Some(&category_string_list));
        if !category_names.is_empty() {
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

        let palette_scroll = gtk4::ScrolledWindow::new();
        palette_scroll.set_child(Some(&palette_flowbox));
        palette_scroll.set_min_content_height(280);
        palette_scroll.set_max_content_height(280);
        palette_scroll.set_propagate_natural_height(false);
        palette_scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

        // --- Helper: populate FlowBox with images from a category ---
        let populate_flowbox = {
            let flowbox = palette_flowbox.clone();
            let paths = palette_paths.clone();
            move |images: &[PathBuf]| {
                // Clear existing children
                while let Some(child) = flowbox.first_child() {
                    flowbox.remove(&child);
                }
                paths.borrow_mut().clear();

                if images.is_empty() {
                    let label = gtk4::Label::new(Some("No palettes in this category."));
                    label.set_wrap(true);
                    label.set_justify(gtk4::Justification::Center);
                    label.add_css_class("dim-label");
                    label.set_margin_top(12);
                    label.set_margin_bottom(12);
                    flowbox.insert(&label, -1);
                    return;
                }

                for path in images {
                    match gdk_pixbuf::Pixbuf::from_file_at_scale(
                        path.to_str().unwrap_or_default(),
                        80,
                        80,
                        false,
                    ) {
                        Ok(pixbuf) => {
                            let texture = gdk::Texture::for_pixbuf(&pixbuf);
                            let image = gtk4::Picture::for_paintable(&texture);
                            image.set_size_request(80, 80);
                            image.set_content_fit(gtk4::ContentFit::Cover);

                            flowbox.insert(&image, -1);
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
            }
        };

        // Show "no palettes" message if no categories exist at all
        if category_names.is_empty() {
            let label = gtk4::Label::new(Some(
                "No palette images found.\nAdd folders with .png files to\n~/.local/share/wallrus/palettes/",
            ));
            label.set_wrap(true);
            label.set_justify(gtk4::Justification::Center);
            label.add_css_class("dim-label");
            label.set_margin_top(12);
            label.set_margin_bottom(12);
            palette_flowbox.insert(&label, -1);
        } else {
            // Populate with the first category
            if let Some(images) = all_categories.get(&category_names[0]) {
                populate_flowbox(images);
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

        // --- Color picker buttons (4, one per color band) ---
        let color_dialog = gtk4::ColorDialog::new();
        color_dialog.set_with_alpha(false);

        let default_colors: [[f32; 3]; 4] = [
            [0.11, 0.25, 0.60],
            [0.90, 0.35, 0.50],
            [0.20, 0.60, 0.40],
            [0.80, 0.70, 0.20],
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

        let resolution_list = gtk4::StringList::new(&[
            ExportResolution::Hd.label(),
            ExportResolution::Qhd.label(),
            ExportResolution::Uhd4k.label(),
        ]);
        let resolution_row = adw::ComboRow::new();
        resolution_row.set_title("Resolution");
        resolution_row.set_model(Some(&resolution_list));

        // Default to the resolution closest to the current display
        let default_res_idx = gdk::Display::default()
            .and_then(|display| {
                let monitors = display.monitors();
                // Find the largest monitor by pixel count
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
            .map(|(w, h)| ExportResolution::best_index_for_display(w, h))
            .unwrap_or(0);
        resolution_row.set_selected(default_res_idx);

        let export_png_button = gtk4::Button::with_label("Export PNG");
        export_png_button.add_css_class("suggested-action");
        export_png_button.set_tooltip_text(Some("Export as PNG (Ctrl+E)"));

        let export_jpg_button = gtk4::Button::with_label("Export JPEG");
        export_jpg_button.set_tooltip_text(Some("Export as JPEG (Ctrl+Shift+E)"));

        let set_wallpaper_button = gtk4::Button::with_label("Set as Wallpaper");
        set_wallpaper_button.add_css_class("suggested-action");
        set_wallpaper_button.set_tooltip_text(Some("Set as desktop wallpaper (Ctrl+Shift+W)"));

        let button_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        button_box.set_halign(gtk4::Align::Center);
        button_box.set_margin_top(8);
        button_box.set_margin_bottom(8);
        button_box.append(&export_png_button);
        button_box.append(&export_jpg_button);
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
        {
            let all_cats = all_categories.clone();
            let cat_names = category_names.clone();
            let populate = populate_flowbox.clone();
            category_row.connect_selected_notify(move |combo| {
                let idx = combo.selected() as usize;
                if let Some(name) = cat_names.get(idx) {
                    if let Some(images) = all_cats.get(name) {
                        populate(images);
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

        let make_export_handler = |format: ExportFormat| {
            let state = state.clone();
            let resolution_row = resolution_row.clone();
            let gl_area = gl_area.clone();
            let window_ref = window.clone();
            move |_button: &gtk4::Button| {
                let resolution = ExportResolution::from_index(resolution_row.selected());
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

                let export_dir = match export::default_export_dir() {
                    Ok(dir) => dir,
                    Err(e) => {
                        show_toast(&window_ref, &format!("Export failed: {}", e));
                        return;
                    }
                };

                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                let preset_name = {
                    let state_ref = state.borrow();
                    state_ref
                        .as_ref()
                        .map(|r| r.current_preset.clone())
                        .unwrap_or_else(|| "wallpaper".to_string())
                };

                let filename = format!(
                    "wallrus_{}_{}.{}",
                    preset_name.to_lowercase(),
                    timestamp,
                    format.extension()
                );
                let path = export_dir.join(&filename);

                match export::save_pixels(&pixels, w, h, &path, format) {
                    Ok(()) => {
                        show_toast(&window_ref, &format!("Saved to {}", path.display()));
                    }
                    Err(e) => {
                        show_toast(&window_ref, &format!("Export failed: {}", e));
                    }
                }
            }
        };

        export_png_button.connect_clicked(make_export_handler(ExportFormat::Png));
        export_jpg_button.connect_clicked(make_export_handler(ExportFormat::Jpeg));

        // --- Set as wallpaper handler ---
        {
            let state = state.clone();
            let resolution_row = resolution_row.clone();
            let gl_area = gl_area.clone();
            let window_ref = window.clone();
            set_wallpaper_button.connect_clicked(move |_| {
                let resolution = ExportResolution::from_index(resolution_row.selected());
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

                let bg_dir = match dirs::data_dir()
                    .or_else(|| dirs::home_dir().map(|h| h.join(".local/share")))
                {
                    Some(dir) => dir.join("backgrounds"),
                    None => {
                        show_toast(&window_ref, "Could not determine data directory");
                        return;
                    }
                };

                if let Err(e) = std::fs::create_dir_all(&bg_dir) {
                    show_toast(&window_ref, &format!("Failed to create directory: {}", e));
                    return;
                }

                let path = bg_dir.join("wallrus_current.png");

                match export::save_pixels(&pixels, w, h, &path, ExportFormat::Png) {
                    Ok(()) => match wallpaper::set_gnome_wallpaper(&path) {
                        Ok(()) => show_toast(&window_ref, "Wallpaper set!"),
                        Err(e) => show_toast(&window_ref, &format!("Failed: {}", e)),
                    },
                    Err(e) => {
                        show_toast(&window_ref, &format!("Failed to save: {}", e));
                    }
                }
            });
        }

        // =====================================================================
        // Keyboard shortcuts via GActions
        // =====================================================================

        let action_export_png = gio::SimpleAction::new("export-png", None);
        {
            let btn = export_png_button.clone();
            action_export_png.connect_activate(move |_, _| btn.emit_clicked());
        }
        window.add_action(&action_export_png);

        let action_export_jpg = gio::SimpleAction::new("export-jpeg", None);
        {
            let btn = export_jpg_button.clone();
            action_export_jpg.connect_activate(move |_, _| btn.emit_clicked());
        }
        window.add_action(&action_export_jpg);

        let action_set_wallpaper = gio::SimpleAction::new("set-wallpaper", None);
        {
            let btn = set_wallpaper_button.clone();
            action_set_wallpaper.connect_activate(move |_, _| btn.emit_clicked());
        }
        window.add_action(&action_set_wallpaper);

        app.set_accels_for_action("win.export-png", &["<Control>e"]);
        app.set_accels_for_action("win.export-jpeg", &["<Control><Shift>e"]);
        app.set_accels_for_action("win.set-wallpaper", &["<Control><Shift>w"]);

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
