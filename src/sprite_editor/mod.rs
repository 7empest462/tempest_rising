use crate::AppState;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};

pub struct SpriteEditorPlugin;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StudioMode {
    #[default]
    CustomFlag,
    CustomCrosshair,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpriteTool {
    Pencil,
    Eraser,
    BucketFill,
    ColorPicker,
    CircleRing,
}

#[derive(Resource)]
pub struct SpriteCanvas {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<[u8; 4]>, // RGBA pixels
}

impl Default for SpriteCanvas {
    fn default() -> Self {
        let width = 32;
        let height = 32;
        let mut canvas = Self {
            width,
            height,
            pixels: vec![[0, 0, 0, 0]; (width * height) as usize],
        };
        load_flag_preset(&mut canvas, "Faction Insignia");
        canvas
    }
}

#[derive(Resource)]
pub struct SpriteEditorSettings {
    pub mode: StudioMode,
    pub current_color: [u8; 4],
    pub tool: SpriteTool,
    pub filename: String,
    pub status_message: String,
    pub show_guides: bool,
}

impl Default for SpriteEditorSettings {
    fn default() -> Self {
        Self {
            mode: StudioMode::CustomFlag,
            current_color: [0, 255, 180, 255], // Neon Cyan default
            tool: SpriteTool::Pencil,
            filename: "assets/textures/custom_flag.png".to_string(),
            status_message: "Welcome to Custom Asset Studio! Paint Flags or Reticles.".to_string(),
            show_guides: true,
        }
    }
}

impl Plugin for SpriteEditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpriteCanvas>()
            .init_resource::<SpriteEditorSettings>()
            .add_systems(
                Update,
                handle_studio_input.run_if(in_state(AppState::SpriteEditor)),
            )
            .add_systems(
                EguiPrimaryContextPass,
                sprite_editor_ui.run_if(in_state(AppState::SpriteEditor)),
            );
    }
}

fn handle_studio_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        next_state.set(AppState::MainMenu);
    }
}

fn flood_fill(
    pixels: &mut [[u8; 4]],
    width: u32,
    height: u32,
    start_x: u32,
    start_y: u32,
    target_color: [u8; 4],
    replacement_color: [u8; 4],
) {
    if target_color == replacement_color {
        return;
    }
    let mut queue = Vec::new();
    queue.push((start_x, start_y));

    while let Some((x, y)) = queue.pop() {
        let idx = (y * width + x) as usize;
        if pixels[idx] == target_color {
            pixels[idx] = replacement_color;

            if x > 0 {
                queue.push((x - 1, y));
            }
            if x < width - 1 {
                queue.push((x + 1, y));
            }
            if y > 0 {
                queue.push((x, y - 1));
            }
            if y < height - 1 {
                queue.push((x, y + 1));
            }
        }
    }
}

fn draw_circle_ring(
    canvas: &mut SpriteCanvas,
    center_x: i32,
    center_y: i32,
    radius: i32,
    color: [u8; 4],
) {
    let mut x = radius;
    let mut y = 0;
    let mut err = 0;

    while x >= y {
        let points = [
            (center_x + x, center_y + y),
            (center_x + y, center_y + x),
            (center_x - y, center_y + x),
            (center_x - x, center_y + y),
            (center_x - x, center_y - y),
            (center_x - y, center_y - x),
            (center_x + y, center_y - x),
            (center_x + x, center_y - y),
        ];

        for (px, py) in points {
            if px >= 0 && px < canvas.width as i32 && py >= 0 && py < canvas.height as i32 {
                let idx = (py as u32 * canvas.width + px as u32) as usize;
                canvas.pixels[idx] = color;
            }
        }

        if err <= 0 {
            y += 1;
            err += 2 * y + 1;
        }
        if err > 0 {
            x -= 1;
            err -= 2 * x + 1;
        }
    }
}

fn resize_canvas(canvas: &mut SpriteCanvas, new_width: u32, new_height: u32) {
    canvas.width = new_width;
    canvas.height = new_height;
    canvas.pixels = vec![[0, 0, 0, 0]; (new_width * new_height) as usize];
}

fn export_png(canvas: &SpriteCanvas, path: &str) -> Result<(), String> {
    if let Some(parent) = std::path::Path::new(path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut img_buf = image::ImageBuffer::new(canvas.width, canvas.height);
    for y in 0..canvas.height {
        for x in 0..canvas.width {
            let idx = (y * canvas.width + x) as usize;
            let c = canvas.pixels[idx];
            img_buf.put_pixel(x, y, image::Rgba(c));
        }
    }
    img_buf.save(path).map_err(|e| e.to_string())
}

fn load_flag_preset(canvas: &mut SpriteCanvas, preset_name: &str) {
    resize_canvas(canvas, 32, 32);
    let w = canvas.width;
    let h = canvas.height;

    match preset_name {
        "Faction Insignia" => {
            // Dark steel background
            for y in 0..h {
                for x in 0..w {
                    let idx = (y * w + x) as usize;
                    canvas.pixels[idx] = [25, 30, 42, 255];
                }
            }
            // Cyan Chevron & Star
            draw_circle_ring(canvas, 16, 16, 12, [0, 220, 255, 255]);
            for i in 6..=26 {
                let idx1 = (i * w + i) as usize;
                let idx2 = (i * w + (31 - i)) as usize;
                canvas.pixels[idx1] = [255, 180, 0, 255];
                canvas.pixels[idx2] = [255, 180, 0, 255];
            }
        }
        "Alien Rune" => {
            for y in 0..h {
                for x in 0..w {
                    let idx = (y * w + x) as usize;
                    canvas.pixels[idx] = [15, 10, 25, 255];
                }
            }
            draw_circle_ring(canvas, 16, 16, 10, [180, 0, 255, 255]);
            draw_circle_ring(canvas, 16, 16, 5, [0, 255, 120, 255]);
        }
        "Star & Stripes" => {
            for y in 0..h {
                for x in 0..w {
                    let idx = (y * w + x) as usize;
                    if y % 6 < 3 {
                        canvas.pixels[idx] = [200, 30, 40, 255];
                    } else {
                        canvas.pixels[idx] = [240, 240, 245, 255];
                    }
                }
            }
            for y in 0..16 {
                for x in 0..16 {
                    let idx = (y * w + x) as usize;
                    canvas.pixels[idx] = [20, 40, 120, 255];
                }
            }
        }
        _ => {}
    }
}

fn load_crosshair_preset(canvas: &mut SpriteCanvas, preset_name: &str) {
    resize_canvas(canvas, 32, 32);
    let w = canvas.width;
    let h = canvas.height;

    // Clear transparent
    canvas.pixels = vec![[0, 0, 0, 0]; (w * h) as usize];

    match preset_name {
        "Tactical Cross" => {
            let neon_green = [0, 255, 100, 255];
            let black_outline = [0, 0, 0, 255];

            // Outer crosshair legs (16,16 center)
            for i in 6..=11 {
                canvas.pixels[(16 * w + i) as usize] = neon_green;
                canvas.pixels[(16 * w + (31 - i)) as usize] = neon_green;
                canvas.pixels[(i * w + 16) as usize] = neon_green;
                canvas.pixels[((31 - i) * w + 16) as usize] = neon_green;
            }

            // Center dot
            canvas.pixels[(16 * w + 16) as usize] = neon_green;

            // Black outline
            for i in 6..=11 {
                for &offset in &[-1i32, 1i32] {
                    let y1 = (16 + offset) as u32;
                    canvas.pixels[(y1 * w + i) as usize] = black_outline;
                    canvas.pixels[(y1 * w + (31 - i)) as usize] = black_outline;
                    canvas.pixels[(i * w + y1) as usize] = black_outline;
                    canvas.pixels[((31 - i) * w + y1) as usize] = black_outline;
                }
            }
        }
        "Cyber Ring" => {
            draw_circle_ring(canvas, 16, 16, 9, [0, 220, 255, 255]);
            canvas.pixels[(16 * w + 16) as usize] = [255, 50, 50, 255];
        }
        "Sniper Scope" => {
            draw_circle_ring(canvas, 16, 16, 14, [255, 255, 255, 220]);
            for i in 2..=30 {
                canvas.pixels[(16 * w + i) as usize] = [255, 255, 255, 180];
                canvas.pixels[(i * w + 16) as usize] = [255, 255, 255, 180];
            }
            canvas.pixels[(16 * w + 16) as usize] = [255, 0, 0, 255];
        }
        _ => {}
    }
}

fn sprite_editor_ui(
    mut contexts: EguiContexts,
    mut next_state: ResMut<NextState<AppState>>,
    mut canvas: ResMut<SpriteCanvas>,
    mut settings: ResMut<SpriteEditorSettings>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    egui::Window::new("🎨 Custom Asset Studio - Flag & Reticle Painter")
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .default_width(780.0)
        .max_width(1050.0)
        .max_height(640.0)
        .collapsible(false)
        .resizable(true)
        .show(ctx, |ui| {
            // Top Header Navigation Bar
            ui.horizontal(|ui| {
                ui.heading("🎨 Custom Asset Studio");
                ui.add_space(15.0);

                if ui
                    .selectable_label(
                        settings.mode == StudioMode::CustomFlag,
                        "🚩 Custom Flag & Banner",
                    )
                    .clicked()
                {
                    settings.mode = StudioMode::CustomFlag;
                    settings.filename = "assets/textures/custom_flag.png".to_string();
                    load_flag_preset(&mut canvas, "Faction Insignia");
                    settings.status_message = "Switched to Custom Flag Mode.".to_string();
                }

                if ui
                    .selectable_label(
                        settings.mode == StudioMode::CustomCrosshair,
                        "🎯 Custom Weapon Crosshair",
                    )
                    .clicked()
                {
                    settings.mode = StudioMode::CustomCrosshair;
                    settings.filename = "assets/textures/custom_crosshair.png".to_string();
                    load_crosshair_preset(&mut canvas, "Tactical Cross");
                    settings.status_message = "Switched to Custom Crosshair Mode.".to_string();
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("🚪 Exit to Main Menu (ESC)").clicked() {
                        next_state.set(AppState::MainMenu);
                    }
                });
            });

            ui.separator();
            ui.add_space(5.0);

            egui::ScrollArea::both()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // Column 1: Tools & Palette
                        ui.vertical(|ui| {
                            ui.set_width(190.0);

                            ui.heading("🛠️ Tools");
                            ui.separator();
                            ui.selectable_value(&mut settings.tool, SpriteTool::Pencil, "✏ Pencil");
                            ui.selectable_value(
                                &mut settings.tool,
                                SpriteTool::Eraser,
                                "🧹 Eraser",
                            );
                            ui.selectable_value(
                                &mut settings.tool,
                                SpriteTool::BucketFill,
                                "🪣 Bucket Fill",
                            );
                            ui.selectable_value(
                                &mut settings.tool,
                                SpriteTool::ColorPicker,
                                "🧪 Eyedropper",
                            );
                            ui.selectable_value(
                                &mut settings.tool,
                                SpriteTool::CircleRing,
                                "⭕ Circle / Ring",
                            );

                            ui.checkbox(&mut settings.show_guides, "📐 Center Guides");

                            ui.add_space(10.0);
                            ui.heading("🎨 Color Palette");
                            ui.separator();

                            let palette = match settings.mode {
                                StudioMode::CustomFlag => vec![
                                    ("Dark Steel", [25, 30, 42, 255]),
                                    ("Gold", [255, 180, 0, 255]),
                                    ("Neon Cyan", [0, 220, 255, 255]),
                                    ("Crimson", [220, 30, 50, 255]),
                                    ("Forest Green", [30, 160, 60, 255]),
                                    ("Purple", [140, 30, 220, 255]),
                                    ("Black", [0, 0, 0, 255]),
                                    ("White", [255, 255, 255, 255]),
                                ],
                                StudioMode::CustomCrosshair => vec![
                                    ("Neon Green", [0, 255, 100, 255]),
                                    ("Laser Red", [255, 40, 40, 255]),
                                    ("Electric Blue", [0, 180, 255, 255]),
                                    ("Yellow", [255, 240, 0, 255]),
                                    ("Hot Pink", [255, 0, 150, 255]),
                                    ("White", [255, 255, 255, 255]),
                                    ("Black Outline", [0, 0, 0, 255]),
                                    ("Transparent", [0, 0, 0, 0]),
                                ],
                            };

                            egui::Grid::new("palette_grid")
                                .spacing(egui::vec2(5.0, 5.0))
                                .show(ui, |ui| {
                                    for (i, (name, color)) in palette.iter().enumerate() {
                                        let button = egui::Button::new("")
                                            .fill(egui::Color32::from_rgba_unmultiplied(
                                                color[0], color[1], color[2], color[3],
                                            ))
                                            .min_size(egui::vec2(26.0, 26.0));

                                        if ui.add(button).on_hover_text(*name).clicked() {
                                            settings.current_color = *color;
                                            if settings.tool == SpriteTool::Eraser {
                                                settings.tool = SpriteTool::Pencil;
                                            }
                                        }

                                        if (i + 1) % 4 == 0 {
                                            ui.end_row();
                                        }
                                    }
                                });

                            ui.add_space(10.0);
                            ui.heading("RGB Sliders");
                            ui.separator();
                            ui.add(
                                egui::Slider::new(&mut settings.current_color[0], 0..=255)
                                    .text("R"),
                            );
                            ui.add(
                                egui::Slider::new(&mut settings.current_color[1], 0..=255)
                                    .text("G"),
                            );
                            ui.add(
                                egui::Slider::new(&mut settings.current_color[2], 0..=255)
                                    .text("B"),
                            );
                            ui.add(
                                egui::Slider::new(&mut settings.current_color[3], 0..=255)
                                    .text("A"),
                            );
                        });

                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(10.0);

                        // Column 2: Canvas Drawing Area
                        ui.vertical(|ui| {
                            ui.heading("🖌️ Pixel Canvas");
                            ui.separator();

                            let canvas_display_size = 300.0;
                            let (response, painter) = ui.allocate_painter(
                                egui::vec2(canvas_display_size, canvas_display_size),
                                egui::Sense::click_and_drag(),
                            );

                            let rect = response.rect;
                            let cell_w = rect.width() / canvas.width as f32;
                            let cell_h = rect.height() / canvas.height as f32;

                            // 1. Checkerboard background for transparency
                            let checker_size = 10.0;
                            for y_pixel in 0..(canvas_display_size / checker_size) as i32 {
                                for x_pixel in 0..(canvas_display_size / checker_size) as i32 {
                                    let check_rect = egui::Rect::from_min_size(
                                        rect.min
                                            + egui::vec2(
                                                x_pixel as f32 * checker_size,
                                                y_pixel as f32 * checker_size,
                                            ),
                                        egui::vec2(checker_size, checker_size),
                                    );
                                    let color = if (x_pixel + y_pixel) % 2 == 0 {
                                        egui::Color32::from_gray(235)
                                    } else {
                                        egui::Color32::from_gray(215)
                                    };
                                    painter.rect_filled(check_rect, 0.0, color);
                                }
                            }

                            // 2. Draw canvas pixels
                            for y in 0..canvas.height {
                                for x in 0..canvas.width {
                                    let idx = (y * canvas.width + x) as usize;
                                    let c = canvas.pixels[idx];
                                    if c[3] > 0 {
                                        let cell_rect = egui::Rect::from_min_size(
                                            rect.min
                                                + egui::vec2(x as f32 * cell_w, y as f32 * cell_h),
                                            egui::vec2(cell_w + 0.5, cell_h + 0.5),
                                        );
                                        painter.rect_filled(
                                            cell_rect,
                                            0.0,
                                            egui::Color32::from_rgba_unmultiplied(
                                                c[0], c[1], c[2], c[3],
                                            ),
                                        );
                                    }
                                }
                            }

                            // 3. Grid lines
                            for x in 0..=canvas.width {
                                let x_pos = rect.min.x + x as f32 * cell_w;
                                painter.line_segment(
                                    [egui::pos2(x_pos, rect.min.y), egui::pos2(x_pos, rect.max.y)],
                                    egui::Stroke::new(0.5, egui::Color32::from_gray(180)),
                                );
                            }
                            for y in 0..=canvas.height {
                                let y_pos = rect.min.y + y as f32 * cell_h;
                                painter.line_segment(
                                    [egui::pos2(rect.min.x, y_pos), egui::pos2(rect.max.x, y_pos)],
                                    egui::Stroke::new(0.5, egui::Color32::from_gray(180)),
                                );
                            }

                            // 4. Center Guides
                            if settings.show_guides {
                                let center_x = rect.min.x + (canvas.width as f32 * 0.5) * cell_w;
                                let center_y = rect.min.y + (canvas.height as f32 * 0.5) * cell_h;
                                painter.line_segment(
                                    [
                                        egui::pos2(center_x, rect.min.y),
                                        egui::pos2(center_x, rect.max.y),
                                    ],
                                    egui::Stroke::new(
                                        1.2,
                                        egui::Color32::from_rgba_unmultiplied(255, 0, 100, 180),
                                    ),
                                );
                                painter.line_segment(
                                    [
                                        egui::pos2(rect.min.x, center_y),
                                        egui::pos2(rect.max.x, center_y),
                                    ],
                                    egui::Stroke::new(
                                        1.2,
                                        egui::Color32::from_rgba_unmultiplied(255, 0, 100, 180),
                                    ),
                                );
                            }

                            // 5. Handle Pointer Interactivity
                            if let Some(pointer_pos) = response.interact_pointer_pos() {
                                let relative_pos = pointer_pos - rect.min;
                                let px = (relative_pos.x / cell_w).floor() as i32;
                                let py = (relative_pos.y / cell_h).floor() as i32;

                                if px >= 0
                                    && px < canvas.width as i32
                                    && py >= 0
                                    && py < canvas.height as i32
                                {
                                    let px = px as u32;
                                    let py = py as u32;

                                    match settings.tool {
                                        SpriteTool::Pencil => {
                                            let idx = (py * canvas.width + px) as usize;
                                            canvas.pixels[idx] = settings.current_color;
                                        }
                                        SpriteTool::Eraser => {
                                            let idx = (py * canvas.width + px) as usize;
                                            canvas.pixels[idx] = [0, 0, 0, 0];
                                        }
                                        SpriteTool::ColorPicker => {
                                            let idx = (py * canvas.width + px) as usize;
                                            let c = canvas.pixels[idx];
                                            if c[3] > 0 {
                                                settings.current_color = c;
                                            }
                                        }
                                        SpriteTool::BucketFill => {
                                            if response.clicked() {
                                                let target_color = canvas.pixels
                                                    [(py * canvas.width + px) as usize];
                                                let w = canvas.width;
                                                let h = canvas.height;
                                                flood_fill(
                                                    &mut canvas.pixels,
                                                    w,
                                                    h,
                                                    px,
                                                    py,
                                                    target_color,
                                                    settings.current_color,
                                                );
                                            }
                                        }
                                        SpriteTool::CircleRing => {
                                            if response.clicked() {
                                                let cw = canvas.width as i32 / 2;
                                                let ch = canvas.height as i32 / 2;
                                                let radius = (px as i32 - cw).abs().max(1);
                                                draw_circle_ring(
                                                    &mut canvas,
                                                    cw,
                                                    ch,
                                                    radius,
                                                    settings.current_color,
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        });

                        ui.add_space(15.0);
                        ui.separator();
                        ui.add_space(15.0);

                        // Column 3: Canvas Presets & Save/Export
                        ui.vertical(|ui| {
                            ui.heading("⚡ Presets & Export");
                            ui.separator();

                            ui.label("Resolution Preset:");
                            ui.horizontal(|ui| {
                                if ui.button("16x16").clicked() {
                                    resize_canvas(&mut canvas, 16, 16);
                                }
                                if ui.button("32x32").clicked() {
                                    resize_canvas(&mut canvas, 32, 32);
                                }
                                if ui.button("64x64").clicked() {
                                    resize_canvas(&mut canvas, 64, 64);
                                }
                            });

                            ui.add_space(10.0);
                            ui.label("Quick Templates:");
                            match settings.mode {
                                StudioMode::CustomFlag => {
                                    if ui.button("🚩 Faction Insignia").clicked() {
                                        load_flag_preset(&mut canvas, "Faction Insignia");
                                    }
                                    if ui.button("🌀 Alien Rune").clicked() {
                                        load_flag_preset(&mut canvas, "Alien Rune");
                                    }
                                    if ui.button("🇺🇸 Star & Stripes").clicked() {
                                        load_flag_preset(&mut canvas, "Star & Stripes");
                                    }
                                }
                                StudioMode::CustomCrosshair => {
                                    if ui.button("🎯 Tactical Cross").clicked() {
                                        load_crosshair_preset(&mut canvas, "Tactical Cross");
                                    }
                                    if ui.button("⭕ Cyber Ring").clicked() {
                                        load_crosshair_preset(&mut canvas, "Cyber Ring");
                                    }
                                    if ui.button("🔭 Sniper Scope").clicked() {
                                        load_crosshair_preset(&mut canvas, "Sniper Scope");
                                    }
                                }
                            }

                            if ui.button("🗑 Clear Canvas").clicked() {
                                let w = canvas.width;
                                let h = canvas.height;
                                canvas.pixels = vec![[0, 0, 0, 0]; (w * h) as usize];
                                settings.status_message = "Canvas cleared!".to_string();
                            }

                            ui.add_space(20.0);
                            ui.heading("💾 Save Texture");
                            ui.separator();
                            ui.label("Destination Path:");
                            ui.text_edit_singleline(&mut settings.filename);

                            if ui
                                .add_sized([160.0, 32.0], egui::Button::new("💾 Save Asset PNG"))
                                .clicked()
                            {
                                match export_png(&canvas, &settings.filename) {
                                    Ok(_) => {
                                        settings.status_message =
                                            format!("✅ Saved texture to {}!", settings.filename);
                                    }
                                    Err(e) => {
                                        settings.status_message = format!("❌ Error: {}", e);
                                    }
                                }
                            }

                            if !settings.status_message.is_empty() {
                                ui.add_space(10.0);
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(&settings.status_message)
                                            .color(egui::Color32::from_rgb(100, 255, 140))
                                            .strong(),
                                    )
                                    .wrap(),
                                );
                            }
                        });
                    });
                });
        });
}
