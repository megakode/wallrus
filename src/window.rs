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
        let preset_dropdown = gtk4::DropDown::new(Some(preset_list), gtk4::Expression::NONE);
        preset_dropdown.set_selected(0);

        let preset_row = adw::ActionRow::builder().title("Shader").build();
        preset_row.add_suffix(&preset_dropdown);
        preset_row.set_activatable_widget(Some(&preset_dropdown));

        // =====================================================================
        // Palette section — category dropdown + FlowBox thumbnail browser
        // =====================================================================

        // Load all categories at startup
        let all_categories = palette::list_palette_categories();
        let category_names: Vec<String> = all_categories.keys().cloned().collect();

        // Category dropdown
        let category_str_refs: Vec<&str> = category_names.iter().map(|s| s.as_str()).collect();
        let category_string_list = gtk4::StringList::new(&category_str_refs);
        let category_dropdown =
            gtk4::DropDown::new(Some(category_string_list), gtk4::Expression::NONE);
        if !category_names.is_empty() {
            category_dropdown.set_selected(0);
        }

        let category_row = adw::ActionRow::builder().title("Category").build();
        category_row.add_suffix(&category_dropdown);
        category_row.set_activatable_widget(Some(&category_dropdown));

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
        palette_scroll.set_min_content_height(200);
        palette_scroll.set_max_content_height(200);
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
        palette_group.add(&palette_scroll);

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

        // --- Swirl slider ---
        let swirl_scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, -10.0, 10.0, 0.1);
        swirl_scale.set_value(0.0);
        swirl_scale.set_hexpand(true);
        swirl_scale.set_draw_value(true);
        swirl_scale.set_value_pos(gtk4::PositionType::Right);

        let swirl_row = adw::ActionRow::builder().title("Swirl").build();
        swirl_row.add_suffix(&swirl_scale);

        // --- Controls group ---
        let controls_group = adw::PreferencesGroup::new();
        controls_group.set_title("Shader");
        controls_group.add(&preset_row);
        controls_group.add(&angle_row);
        controls_group.add(&scale_row);
        controls_group.add(&speed_row);
        controls_group.add(&blend_row);
        controls_group.add(&swirl_row);

        // =====================================================================
        // Export section
        // =====================================================================

        let resolution_list = gtk4::StringList::new(&[
            ExportResolution::Hd.label(),
            ExportResolution::Qhd.label(),
            ExportResolution::Uhd4k.label(),
        ]);
        let resolution_dropdown =
            gtk4::DropDown::new(Some(resolution_list), gtk4::Expression::NONE);
        resolution_dropdown.set_selected(0);

        let resolution_row = adw::ActionRow::builder().title("Resolution").build();
        resolution_row.add_suffix(&resolution_dropdown);
        resolution_row.set_activatable_widget(Some(&resolution_dropdown));

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
            .default_width(1100)
            .default_height(700)
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
            category_dropdown.connect_selected_notify(move |dropdown| {
                let idx = dropdown.selected() as usize;
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
                        }
                        Err(e) => {
                            eprintln!("Failed to extract colors from '{}': {}", path.display(), e);
                        }
                    }
                }
            });
        }

        // --- Update visibility of shader controls based on preset ---
        let update_control_visibility = {
            let angle_row = angle_row.clone();
            let scale_row = scale_row.clone();
            let speed_row = speed_row.clone();
            let speed_scale = speed_scale.clone();
            move |name: &str| {
                let controls = shader_presets::controls_for(name);
                angle_row.set_visible(controls.has_angle);
                scale_row.set_visible(controls.has_scale);
                speed_row.set_visible(controls.has_speed);
                // Update speed/time slider label and range per preset
                speed_row.set_title(controls.speed_label);
                let (min, max, step, default) = controls.speed_range;
                speed_scale.set_range(min, max);
                speed_scale.set_increments(step, step * 10.0);
                speed_scale.set_value(default);
            }
        };

        update_control_visibility("Gradient");

        // --- Preset change ---
        {
            let state = state.clone();
            let gl_area = gl_area.clone();
            preset_dropdown.connect_selected_notify(move |dropdown| {
                let idx = dropdown.selected();
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

        // --- Swirl change ---
        {
            let state = state.clone();
            swirl_scale.connect_value_changed(move |scale| {
                if let Some(ref mut renderer) = *state.borrow_mut() {
                    renderer.swirl = scale.value() as f32;
                }
            });
        }

        // =====================================================================
        // Export handlers
        // =====================================================================

        let make_export_handler = |format: ExportFormat| {
            let state = state.clone();
            let resolution_dropdown = resolution_dropdown.clone();
            let gl_area = gl_area.clone();
            let window_ref = window.clone();
            move |_button: &gtk4::Button| {
                let resolution = ExportResolution::from_index(resolution_dropdown.selected());
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
            let resolution_dropdown = resolution_dropdown.clone();
            let gl_area = gl_area.clone();
            let window_ref = window.clone();
            set_wallpaper_button.connect_clicked(move |_| {
                let resolution = ExportResolution::from_index(resolution_dropdown.selected());
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
