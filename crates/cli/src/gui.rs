use super::{load_model, save_model, BoxModel};
use eframe::egui::{self, Color32, Pos2, Sense, Shape, Stroke, Vec2};
use openjewelcad_cad_ffi::Kernel;
use rfd::FileDialog;
use std::error::Error;
use std::path::PathBuf;

pub(super) fn launch() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1120.0, 720.0])
            .with_min_inner_size([900.0, 600.0]),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    eframe::run_native(
        "Open Jewel CAD",
        options,
        Box::new(|_| Ok(Box::new(CadApp::new()))),
    )
}

struct CadApp {
    width_mm: f64,
    depth_mm: f64,
    height_mm: f64,
    project_path: String,
    yaw: f32,
    elevation: f32,
    zoom: f32,
    status: String,
    status_is_error: bool,
}

impl CadApp {
    fn new() -> Self {
        let (status, status_is_error) = match Kernel::from_environment() {
            Ok(_) => (
                "OCCT kernel ready. Edit the box and regenerate.".into(),
                false,
            ),
            Err(error) => (format!("CAD kernel unavailable: {error}"), true),
        };
        Self {
            width_mm: 10.0,
            depth_mm: 10.0,
            height_mm: 2.0,
            project_path: String::new(),
            yaw: 0.65,
            elevation: 0.42,
            zoom: 1.0,
            status,
            status_is_error,
        }
    }

    fn new_project(&mut self) {
        self.width_mm = 10.0;
        self.depth_mm = 10.0;
        self.height_mm = 2.0;
        self.project_path.clear();
        self.set_status("New box. Set dimensions, then save or regenerate.", false);
    }

    fn open_project(&mut self) {
        let Some(path) = FileDialog::new()
            .add_filter("Open Jewel CAD project", &["ojcad"])
            .pick_file()
        else {
            return;
        };
        match load_model(&path) {
            Ok(model) => {
                self.width_mm = model.width_mm.as_mm();
                self.depth_mm = model.depth_mm.as_mm();
                self.height_mm = model.height_mm.as_mm();
                self.project_path = path.to_string_lossy().into_owned();
                self.set_status("Project loaded. Regenerate to check it with OCCT.", false);
            }
            Err(error) => self.set_status(format!("Could not open project: {error}"), true),
        }
    }

    fn save_project(&mut self) {
        let path = self.project_path.trim();
        if path.is_empty() {
            self.save_project_as();
            return;
        }
        self.write_project(PathBuf::from(path));
    }

    fn save_project_as(&mut self) {
        let Some(mut path) = FileDialog::new()
            .add_filter("Open Jewel CAD project", &["ojcad"])
            .set_file_name("jewelry_box.ojcad")
            .save_file()
        else {
            return;
        };
        if path.extension().is_none() {
            path.set_extension("ojcad");
        }
        self.project_path = path.to_string_lossy().into_owned();
        self.write_project(path);
    }

    fn write_project(&mut self, path: PathBuf) {
        match BoxModel::new(self.width_mm, self.depth_mm, self.height_mm)
            .and_then(|model| save_model(&path, model).map_err(Into::into))
        {
            Ok(()) => self.set_status(format!("Saved project to {}", path.display()), false),
            Err(error) => self.set_status(format!("Could not save project: {error}"), true),
        }
    }

    fn regenerate(&mut self) {
        let result: Result<(f64, bool), Box<dyn Error>> = (|| {
            let model = BoxModel::new(self.width_mm, self.depth_mm, self.height_mm)?;
            let kernel = Kernel::from_environment()?;
            let shape = kernel.box_mm(
                model.width_mm.as_mm(),
                model.depth_mm.as_mm(),
                model.height_mm.as_mm(),
            )?;
            Ok((shape.volume_mm3()?, shape.is_valid()?))
        })();

        match result {
            Ok((volume, true)) => self.set_status(
                format!("OCCT solid is valid · volume {volume:.3} mm³"),
                false,
            ),
            Ok((volume, false)) => self.set_status(
                format!("OCCT created the shape, but it is invalid · volume {volume:.3} mm³"),
                true,
            ),
            Err(error) => self.set_status(format!("Could not regenerate shape: {error}"), true),
        }
    }

    fn set_status(&mut self, message: impl Into<String>, is_error: bool) {
        self.status = message.into();
        self.status_is_error = is_error;
    }
}

impl eframe::App for CadApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.heading("Open Jewel CAD");
                ui.separator();
                if ui.button("New").clicked() {
                    self.new_project();
                }
                if ui.button("Open…").clicked() {
                    self.open_project();
                }
                if ui.button("Save").clicked() {
                    self.save_project();
                }
                if ui.button("Save As…").clicked() {
                    self.save_project_as();
                }
                ui.separator();
                if ui.button("Regenerate solid").clicked() {
                    self.regenerate();
                }
            });
            ui.separator();
            let full_width = ui.available_width();
            let body_height = (ui.available_height() - 34.0).max(300.0);
            let sidebar_width = 255.0;
            let preview_width = (full_width - sidebar_width - 16.0).max(350.0);
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(
                    Vec2::new(sidebar_width, body_height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| self.show_sidebar(ui),
                );
                ui.add_space(12.0);
                ui.allocate_ui_with_layout(
                    Vec2::new(preview_width, body_height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| self.show_preview(ui),
                );
            });

            ui.separator();
            let color = if self.status_is_error {
                Color32::from_rgb(220, 105, 90)
            } else {
                Color32::from_rgb(120, 190, 145)
            };
            ui.label(egui::RichText::new(&self.status).color(color));
        });
    }
}

impl CadApp {
    fn show_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.heading("Box feature");
        ui.label("Dimensions are in millimetres.");
        ui.add_space(12.0);

        let dimensions_changed = dimension_field(ui, "Width", &mut self.width_mm)
            | dimension_field(ui, "Depth", &mut self.depth_mm)
            | dimension_field(ui, "Height", &mut self.height_mm);
        if dimensions_changed {
            self.set_status(
                "Dimensions changed. Regenerate the solid to validate it.",
                false,
            );
        }

        ui.add_space(12.0);
        ui.separator();
        ui.label("Project file");
        ui.add(
            egui::TextEdit::singleline(&mut self.project_path)
                .hint_text("Save As… to choose a file"),
        );

        ui.add_space(12.0);
        ui.separator();
        ui.label("View controls");
        ui.label("Drag in the preview to rotate the box.");
        ui.add(egui::Slider::new(&mut self.zoom, 0.5..=2.0).text("Zoom"));
    }

    fn show_preview(&mut self, ui: &mut egui::Ui) {
        ui.heading("3D preview");
        ui.label("A shaded box preview. Regenerate solid to verify the model with OCCT.");
        ui.add_space(8.0);

        let size = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(size, Sense::drag());
        if response.dragged() {
            let motion = response.drag_motion();
            self.yaw += motion.x * 0.012;
            self.elevation = (self.elevation + motion.y * 0.010).clamp(-0.45, 1.1);
        }
        draw_box_preview(
            ui.painter(),
            rect,
            [self.width_mm, self.depth_mm, self.height_mm],
            self.yaw,
            self.elevation,
            self.zoom,
        );
    }
}

fn dimension_field(ui: &mut egui::Ui, name: &str, value: &mut f64) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(name);
        changed = ui.add(egui::DragValue::new(value).speed(0.1)).changed();
        ui.label("mm");
    });
    changed
}

#[derive(Clone, Copy)]
struct ProjectedPoint {
    position: Pos2,
    depth: f32,
}

fn draw_box_preview(
    painter: &egui::Painter,
    rect: egui::Rect,
    dimensions: [f64; 3],
    yaw: f32,
    elevation: f32,
    zoom: f32,
) {
    let background = Color32::from_rgb(28, 31, 38);
    painter.rect_filled(rect, egui::CornerRadius::ZERO, background);
    painter.rect_stroke(
        rect,
        egui::CornerRadius::ZERO,
        Stroke::new(1.0, Color32::from_rgb(62, 68, 78)),
        egui::StrokeKind::Inside,
    );

    if dimensions
        .iter()
        .any(|dimension| !dimension.is_finite() || *dimension <= 0.0)
    {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "Enter positive, finite dimensions to preview the box",
            egui::FontId::proportional(18.0),
            Color32::from_rgb(230, 170, 115),
        );
        return;
    }

    let max_dimension = dimensions.into_iter().fold(0.0_f64, f64::max);
    let half = dimensions.map(|dimension| (dimension / max_dimension * 0.5) as f32);
    let vertices = [
        [-half[0], -half[1], -half[2]],
        [half[0], -half[1], -half[2]],
        [half[0], half[1], -half[2]],
        [-half[0], half[1], -half[2]],
        [-half[0], -half[1], half[2]],
        [half[0], -half[1], half[2]],
        [half[0], half[1], half[2]],
        [-half[0], half[1], half[2]],
    ];
    let cos_yaw = yaw.cos();
    let sin_yaw = yaw.sin();
    let cos_elevation = elevation.cos();
    let sin_elevation = elevation.sin();
    let right = [cos_yaw, sin_yaw, 0.0];
    let up = [
        -sin_yaw * sin_elevation,
        cos_yaw * sin_elevation,
        cos_elevation,
    ];
    let camera = [
        sin_yaw * cos_elevation,
        -cos_yaw * cos_elevation,
        sin_elevation,
    ];
    let scale = rect.width().min(rect.height()) * 0.65 * zoom;
    let center = rect.center();
    let projected: Vec<ProjectedPoint> = vertices
        .into_iter()
        .map(|point| {
            let x = dot(point, right);
            let y = dot(point, up);
            ProjectedPoint {
                position: Pos2::new(center.x + x * scale, center.y - y * scale),
                depth: dot(point, camera),
            }
        })
        .collect();

    let faces: [([usize; 4], [f32; 3]); 6] = [
        ([0, 1, 2, 3], [0.0, 0.0, -1.0]),
        ([0, 1, 5, 4], [0.0, -1.0, 0.0]),
        ([1, 2, 6, 5], [1.0, 0.0, 0.0]),
        ([2, 3, 7, 6], [0.0, 1.0, 0.0]),
        ([3, 0, 4, 7], [-1.0, 0.0, 0.0]),
        ([4, 5, 6, 7], [0.0, 0.0, 1.0]),
    ];
    let light = normalize([0.45, -0.6, 0.8]);
    let mut visible_faces: Vec<_> = faces
        .iter()
        .filter_map(|(indices, normal)| {
            let facing = dot(*normal, camera);
            (facing > 0.001).then(|| {
                let depth = indices
                    .iter()
                    .map(|index| projected[*index].depth)
                    .sum::<f32>()
                    / 4.0;
                (depth, *indices, *normal)
            })
        })
        .collect();
    visible_faces.sort_by(|a, b| a.0.total_cmp(&b.0));

    for (_, indices, normal) in visible_faces {
        let points = indices
            .iter()
            .map(|index| projected[*index].position)
            .collect();
        let brightness = 0.50 + 0.50 * dot(normalize(normal), light).max(0.0);
        painter.add(Shape::convex_polygon(
            points,
            shade([211, 155, 76], brightness),
            Stroke::new(1.6, Color32::from_rgb(50, 39, 28)),
        ));
    }

    painter.text(
        Pos2::new(rect.left() + 18.0, rect.bottom() - 20.0),
        egui::Align2::LEFT_CENTER,
        "Millimetres · drag to rotate",
        egui::FontId::proportional(13.0),
        Color32::from_rgb(168, 176, 188),
    );
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn normalize(vector: [f32; 3]) -> [f32; 3] {
    let length = dot(vector, vector).sqrt().max(f32::EPSILON);
    [vector[0] / length, vector[1] / length, vector[2] / length]
}

fn shade(base: [u8; 3], brightness: f32) -> Color32 {
    let channel = |value: u8| ((f32::from(value) * brightness).round() as u8).min(255);
    Color32::from_rgb(channel(base[0]), channel(base[1]), channel(base[2]))
}

#[allow(dead_code)]
fn _keep_vec2_type_import_used(_: Vec2) {}
