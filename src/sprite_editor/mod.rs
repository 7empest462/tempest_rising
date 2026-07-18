use crate::AppState;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};

pub struct SpriteEditorPlugin;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpriteTool {
    Pencil,
    Eraser,
    BucketFill,
    ColorPicker,
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
        Self {
            width,
            height,
            pixels: vec![[0, 0, 0, 0]; (width * height) as usize], // Transparent canvas
        }
    }
}

#[derive(Resource)]
pub struct SpriteEditorSettings {
    pub current_color: [u8; 4],
    pub tool: SpriteTool,
    pub filename: String,
    pub status_message: String,
}

impl Default for SpriteEditorSettings {
    fn default() -> Self {
        Self {
            current_color: [255, 0, 0, 255], // Red default
            tool: SpriteTool::Pencil,
            filename: "sprite.png".to_string(),
            status_message: "".to_string(),
        }
    }
}

impl Plugin for SpriteEditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpriteCanvas>()
            .init_resource::<SpriteEditorSettings>()
            .add_systems(
                EguiPrimaryContextPass,
                sprite_editor_ui.run_if(in_state(AppState::SpriteEditor)),
            );
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

fn resize_canvas(canvas: &mut SpriteCanvas, new_width: u32, new_height: u32) {
    canvas.width = new_width;
    canvas.height = new_height;
    canvas.pixels = vec![[0, 0, 0, 0]; (new_width * new_height) as usize];
}

fn export_png(canvas: &SpriteCanvas, path: &str) -> Result<(), String> {
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

fn sprite_editor_ui(
    mut contexts: EguiContexts,
    mut next_state: ResMut<NextState<AppState>>,
    mut canvas: ResMut<SpriteCanvas>,
    mut settings: ResMut<SpriteEditorSettings>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    egui::Window::new("Sprite Editor Canvas")
        .default_width(550.0)
        .default_height(400.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Left Column: Toolbar and Color Palette
                ui.vertical(|ui| {
                    ui.heading("Tools");
                    ui.separator();
                    ui.selectable_value(&mut settings.tool, SpriteTool::Pencil, "✏ Pencil");
                    ui.selectable_value(&mut settings.tool, SpriteTool::Eraser, "🧹 Eraser");
                    ui.selectable_value(&mut settings.tool, SpriteTool::BucketFill, "🪣 Fill");
                    ui.selectable_value(&mut settings.tool, SpriteTool::ColorPicker, "🧪 Picker");

                    ui.add_space(10.0);
                    ui.heading("Palette");
                    ui.separator();

                    // Simple grid for color palette
                    egui::Grid::new("color_palette")
                        .spacing(egui::vec2(4.0, 4.0))
                        .show(ui, |ui| {
                            let colors = [
                                ("Red", [255, 0, 0, 255]),
                                ("Green", [0, 255, 0, 255]),
                                ("Blue", [0, 0, 255, 255]),
                                ("Yellow", [255, 255, 0, 255]),
                                ("Cyan", [0, 255, 255, 255]),
                                ("Magenta", [255, 0, 255, 255]),
                                ("Black", [0, 0, 0, 255]),
                                ("White", [255, 255, 255, 255]),
                            ];

                            for (i, (name, color)) in colors.iter().enumerate() {
                                let r = color[0];
                                let g = color[1];
                                let b = color[2];
                                let a = color[3];

                                let button = egui::Button::new("")
                                    .fill(egui::Color32::from_rgba_unmultiplied(r, g, b, a))
                                    .min_size(egui::vec2(24.0, 24.0));

                                if ui.add(button).on_hover_text(*name).clicked() {
                                    settings.current_color = *color;
                                    settings.tool = SpriteTool::Pencil; // Auto switch to pencil when picking color
                                }

                                if (i + 1) % 4 == 0 {
                                    ui.end_row();
                                }
                            }
                        });

                    ui.add_space(10.0);
                    ui.heading("Color Picker");
                    ui.separator();
                    let preview_r = settings.current_color[0];
                    let preview_g = settings.current_color[1];
                    let preview_b = settings.current_color[2];
                    let preview_a = settings.current_color[3];
                    ui.horizontal(|ui| {
                        let (preview_rect, _) =
                            ui.allocate_exact_size(egui::vec2(32.0, 32.0), egui::Sense::hover());
                        ui.painter().rect_filled(
                            preview_rect,
                            4.0,
                            egui::Color32::from_rgba_unmultiplied(
                                preview_r, preview_g, preview_b, preview_a,
                            ),
                        );

                        // RGB sliders
                        ui.vertical(|ui| {
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
                    });
                });

                ui.add_space(15.0);
                ui.separator();
                ui.add_space(15.0);

                // Center Column: The Paint Canvas
                ui.vertical(|ui| {
                    ui.heading("Canvas");
                    ui.separator();

                    let canvas_display_size = 320.0;
                    let (response, painter) = ui.allocate_painter(
                        egui::vec2(canvas_display_size, canvas_display_size),
                        egui::Sense::click_and_drag(),
                    );

                    let rect = response.rect;
                    let cell_w = rect.width() / canvas.width as f32;
                    let cell_h = rect.height() / canvas.height as f32;

                    // 1. Draw Checkerboard background
                    let checker_size = 8.0;
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
                                egui::Color32::from_gray(245)
                            } else {
                                egui::Color32::from_gray(225)
                            };
                            painter.rect_filled(check_rect, 0.0, color);
                        }
                    }

                    // 2. Draw pixels
                    for y in 0..canvas.height {
                        for x in 0..canvas.width {
                            let idx = (y * canvas.width + x) as usize;
                            let c = canvas.pixels[idx];
                            if c[3] > 0 {
                                // If alpha > 0
                                let cell_rect = egui::Rect::from_min_size(
                                    rect.min + egui::vec2(x as f32 * cell_w, y as f32 * cell_h),
                                    egui::vec2(cell_w + 0.5, cell_h + 0.5), // overlapping slightly to avoid gaps
                                );
                                painter.rect_filled(
                                    cell_rect,
                                    0.0,
                                    egui::Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]),
                                );
                            }
                        }
                    }

                    // 3. Draw grid lines (subtle)
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

                    // 4. Handle Draw Inputs
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
                                        let target_color =
                                            canvas.pixels[(py * canvas.width + px) as usize];
                                        if target_color != settings.current_color {
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
                                }
                            }
                        }
                    }
                });

                ui.add_space(15.0);
                ui.separator();
                ui.add_space(15.0);

                // Right Column: Canvas settings, PNG export
                ui.vertical(|ui| {
                    ui.heading("Settings");
                    ui.separator();

                    ui.label("Grid Size:");
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

                    if ui.button("Clear Canvas").clicked() {
                        let w = canvas.width;
                        let h = canvas.height;
                        canvas.pixels = vec![[0, 0, 0, 0]; (w * h) as usize];
                        settings.status_message = "Canvas cleared!".to_string();
                    }

                    ui.add_space(20.0);
                    ui.heading("Export Sprite");
                    ui.separator();
                    ui.label("Path:");
                    ui.text_edit_singleline(&mut settings.filename);

                    if ui.button("💾 Save PNG").clicked() {
                        match export_png(&canvas, &settings.filename) {
                            Ok(_) => {
                                settings.status_message = "Saved PNG successfully!".to_string();
                            }
                            Err(e) => {
                                settings.status_message = format!("Error: {}", e);
                            }
                        }
                    }

                    if !settings.status_message.is_empty() {
                        ui.add_space(10.0);
                        ui.add(egui::Label::new(
                            egui::RichText::new(&settings.status_message)
                                .color(egui::Color32::from_rgb(100, 255, 100))
                                .strong(),
                        ));
                    }

                    ui.add_space(30.0);
                    if ui.button("🚪 Exit").clicked() {
                        next_state.set(AppState::MainMenu);
                    }
                });
            });
        });
}
