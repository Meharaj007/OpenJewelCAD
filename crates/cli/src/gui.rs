use super::{load_model, save_model, PrimitiveKind, PrimitiveModel};
use eframe::egui::{self, Color32, Pos2, Sense, Shape, Stroke, Vec2};
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
    kind: PrimitiveKind,
    parameters_mm: [f64; 3],
    project_path: String,
    yaw: f32,
    elevation: f32,
    zoom: f32,
    status: String,
    status_is_error: bool,
}

impl CadApp {
    fn new() -> Self {
        let (status, status_is_error) = match openjewelcad_cad_ffi::Kernel::from_environment() {
            Ok(_) => (
                "OCCT kernel ready. Choose a solid and regenerate.".into(),
                false,
            ),
            Err(error) => (format!("CAD kernel unavailable: {error}"), true),
        };
        Self {
            kind: PrimitiveKind::Box,
            parameters_mm: PrimitiveKind::Box.defaults(),
            project_path: String::new(),
            yaw: 0.65,
            elevation: 0.42,
            zoom: 1.0,
            status,
            status_is_error,
        }
    }

    fn new_project(&mut self) {
        self.kind = PrimitiveKind::Box;
        self.parameters_mm = self.kind.defaults();
        self.project_path.clear();
        self.set_status("New solid. Choose a tool, then set dimensions.", false);
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
                self.kind = model.kind;
                self.parameters_mm = model.values();
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
        match PrimitiveModel::new(self.kind, self.parameters_mm)
            .and_then(|model| save_model(&path, model).map_err(Into::into))
        {
            Ok(()) => self.set_status(format!("Saved project to {}", path.display()), false),
            Err(error) => self.set_status(format!("Could not save project: {error}"), true),
        }
    }

    fn regenerate(&mut self) {
        let result: Result<(f64, bool), Box<dyn Error>> = (|| {
            let model = PrimitiveModel::new(self.kind, self.parameters_mm)?;
            let kernel = openjewelcad_cad_ffi::Kernel::from_environment()?;
            let shape = model.create_shape(&kernel)?;
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
        ui.heading("Solid primitive");
        ui.label("Dimensions are in millimetres.");
        ui.add_space(12.0);

        let previous_kind = self.kind;
        egui::ComboBox::from_label("CAD tool")
            .selected_text(self.kind.display_name())
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.kind, PrimitiveKind::Box, "Box");
                ui.selectable_value(&mut self.kind, PrimitiveKind::Cylinder, "Cylinder");
                ui.selectable_value(&mut self.kind, PrimitiveKind::Sphere, "Sphere");
                ui.selectable_value(&mut self.kind, PrimitiveKind::Torus, "Torus");
            });
        if self.kind != previous_kind {
            self.parameters_mm = self.kind.defaults();
        }

        let mut dimensions_changed = self.kind != previous_kind;
        for index in 0..self.kind.parameter_names().len() {
            dimensions_changed |= dimension_field(
                ui,
                self.kind.parameter_labels()[index],
                &mut self.parameters_mm[index],
            );
        }
        if self.kind == PrimitiveKind::Torus {
            ui.label("Major radius must be larger than tube radius.");
        }
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
        ui.label("Drag to rotate. Regenerate solid to verify the model with OCCT.");
        ui.add_space(8.0);

        let size = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(size, Sense::drag());
        if response.dragged() {
            let motion = response.drag_motion();
            self.yaw += motion.x * 0.012;
            self.elevation = (self.elevation + motion.y * 0.010).clamp(-0.45, 1.1);
        }
        draw_primitive_preview(
            ui.painter(),
            rect,
            self.kind,
            self.parameters_mm,
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

struct PreviewFace {
    indices: Vec<usize>,
    normal: [f32; 3],
}

struct PreviewMesh {
    vertices: Vec<[f32; 3]>,
    faces: Vec<PreviewFace>,
}

fn draw_primitive_preview(
    painter: &egui::Painter,
    rect: egui::Rect,
    kind: PrimitiveKind,
    parameters: [f64; 3],
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

    let parameter_count = kind.parameter_names().len();
    let invalid = parameters[..parameter_count]
        .iter()
        .any(|dimension| !dimension.is_finite() || *dimension <= 0.0)
        || (kind == PrimitiveKind::Torus && parameters[0] <= parameters[1]);
    if invalid {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "Enter valid positive dimensions to preview this solid",
            egui::FontId::proportional(18.0),
            Color32::from_rgb(230, 170, 115),
        );
        return;
    }

    let mesh = make_preview_mesh(kind, parameters);
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
    let projected: Vec<ProjectedPoint> = mesh
        .vertices
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

    let light = normalize([0.45, -0.6, 0.8]);
    let mut visible_faces: Vec<_> = mesh
        .faces
        .iter()
        .filter(|face| dot(face.normal, camera) > 0.001)
        .map(|face| {
            let depth = face
                .indices
                .iter()
                .map(|index| projected[*index].depth)
                .sum::<f32>()
                / face.indices.len() as f32;
            (depth, face)
        })
        .collect();
    visible_faces.sort_by(|a, b| a.0.total_cmp(&b.0));

    for (_, face) in visible_faces {
        let points = face
            .indices
            .iter()
            .map(|index| projected[*index].position)
            .collect();
        let brightness = 0.50 + 0.50 * dot(face.normal, light).max(0.0);
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

fn make_preview_mesh(kind: PrimitiveKind, parameters: [f64; 3]) -> PreviewMesh {
    match kind {
        PrimitiveKind::Box => box_preview_mesh(parameters),
        PrimitiveKind::Cylinder => cylinder_preview_mesh(parameters[0], parameters[1]),
        PrimitiveKind::Sphere => sphere_preview_mesh(parameters[0]),
        PrimitiveKind::Torus => torus_preview_mesh(parameters[0], parameters[1]),
    }
}

fn scaled_point(point: [f64; 3], extent: f64) -> [f32; 3] {
    [
        (point[0] / extent) as f32,
        (point[1] / extent) as f32,
        (point[2] / extent) as f32,
    ]
}

fn box_preview_mesh(dimensions: [f64; 3]) -> PreviewMesh {
    let extent = dimensions.into_iter().fold(0.0_f64, f64::max);
    let half = dimensions.map(|dimension| dimension / extent * 0.5);
    let vertices = [
        [-half[0], -half[1], -half[2]],
        [half[0], -half[1], -half[2]],
        [half[0], half[1], -half[2]],
        [-half[0], half[1], -half[2]],
        [-half[0], -half[1], half[2]],
        [half[0], -half[1], half[2]],
        [half[0], half[1], half[2]],
        [-half[0], half[1], half[2]],
    ]
    .into_iter()
    .map(|point| point.map(|coordinate| coordinate as f32))
    .collect();
    let faces = vec![
        preview_face(&[0, 1, 2, 3], [0.0, 0.0, -1.0]),
        preview_face(&[0, 1, 5, 4], [0.0, -1.0, 0.0]),
        preview_face(&[1, 2, 6, 5], [1.0, 0.0, 0.0]),
        preview_face(&[2, 3, 7, 6], [0.0, 1.0, 0.0]),
        preview_face(&[3, 0, 4, 7], [-1.0, 0.0, 0.0]),
        preview_face(&[4, 5, 6, 7], [0.0, 0.0, 1.0]),
    ];
    PreviewMesh { vertices, faces }
}

fn cylinder_preview_mesh(radius: f64, height: f64) -> PreviewMesh {
    let segments = 40;
    let extent = radius.max(height * 0.5);
    let mut vertices = Vec::with_capacity(segments * 2);
    for z in [-height * 0.5, height * 0.5] {
        for segment in 0..segments {
            let angle = std::f64::consts::TAU * segment as f64 / segments as f64;
            vertices.push(scaled_point(
                [radius * angle.cos(), radius * angle.sin(), z],
                extent,
            ));
        }
    }
    let mut faces = Vec::with_capacity(segments + 2);
    for segment in 0..segments {
        let next = (segment + 1) % segments;
        let angle = std::f64::consts::TAU * (segment as f64 + 0.5) / segments as f64;
        faces.push(preview_face(
            &[segment, next, segments + next, segments + segment],
            [angle.cos() as f32, angle.sin() as f32, 0.0],
        ));
    }
    faces.push(preview_face(
        &(0..segments).rev().collect::<Vec<_>>(),
        [0.0, 0.0, -1.0],
    ));
    faces.push(preview_face(
        &(segments..segments * 2).collect::<Vec<_>>(),
        [0.0, 0.0, 1.0],
    ));
    PreviewMesh { vertices, faces }
}

fn sphere_preview_mesh(radius: f64) -> PreviewMesh {
    let latitude_steps = 14;
    let longitude_steps = 28;
    let mut vertices = vec![
        scaled_point([0.0, 0.0, radius], radius),
        scaled_point([0.0, 0.0, -radius], radius),
    ];
    for latitude in 1..latitude_steps {
        let angle = -std::f64::consts::FRAC_PI_2
            + std::f64::consts::PI * latitude as f64 / latitude_steps as f64;
        for longitude in 0..longitude_steps {
            let around = std::f64::consts::TAU * longitude as f64 / longitude_steps as f64;
            let cos_lat = angle.cos();
            vertices.push(scaled_point(
                [
                    radius * cos_lat * around.cos(),
                    radius * cos_lat * around.sin(),
                    radius * angle.sin(),
                ],
                radius,
            ));
        }
    }

    let ring = |latitude: usize, longitude: usize| {
        2 + latitude * longitude_steps + longitude % longitude_steps
    };
    let mut faces = Vec::new();
    for longitude in 0..longitude_steps {
        let next = (longitude + 1) % longitude_steps;
        let normal = normalize([
            (vertices[ring(0, longitude)][0] + vertices[ring(0, next)][0]) * 0.5,
            (vertices[ring(0, longitude)][1] + vertices[ring(0, next)][1]) * 0.5,
            (vertices[ring(0, longitude)][2] + vertices[ring(0, next)][2] + 1.0) * 0.5,
        ]);
        faces.push(preview_face(
            &[0, ring(0, longitude), ring(0, next)],
            normal,
        ));
        for latitude in 0..latitude_steps - 2 {
            let indices = [
                ring(latitude, longitude),
                ring(latitude + 1, longitude),
                ring(latitude + 1, next),
                ring(latitude, next),
            ];
            faces.push(preview_face(
                &indices,
                face_radial_normal(&vertices, &indices),
            ));
        }
        let last = latitude_steps - 2;
        let normal = normalize([
            (vertices[ring(last, longitude)][0] + vertices[ring(last, next)][0]) * 0.5,
            (vertices[ring(last, longitude)][1] + vertices[ring(last, next)][1]) * 0.5,
            (vertices[ring(last, longitude)][2] + vertices[ring(last, next)][2] - 1.0) * 0.5,
        ]);
        faces.push(preview_face(
            &[1, ring(last, next), ring(last, longitude)],
            normal,
        ));
    }
    PreviewMesh { vertices, faces }
}

fn torus_preview_mesh(major_radius: f64, tube_radius: f64) -> PreviewMesh {
    let major_steps = 40;
    let tube_steps = 16;
    let extent = major_radius + tube_radius;
    let mut vertices = Vec::with_capacity(major_steps * tube_steps);
    for major in 0..major_steps {
        let around = std::f64::consts::TAU * major as f64 / major_steps as f64;
        for tube in 0..tube_steps {
            let cross = std::f64::consts::TAU * tube as f64 / tube_steps as f64;
            let radial = major_radius + tube_radius * cross.cos();
            vertices.push(scaled_point(
                [
                    radial * around.cos(),
                    radial * around.sin(),
                    tube_radius * cross.sin(),
                ],
                extent,
            ));
        }
    }
    let mut faces = Vec::with_capacity(major_steps * tube_steps);
    for major in 0..major_steps {
        let next_major = (major + 1) % major_steps;
        let around = std::f64::consts::TAU * (major as f64 + 0.5) / major_steps as f64;
        for tube in 0..tube_steps {
            let next_tube = (tube + 1) % tube_steps;
            let cross = std::f64::consts::TAU * (tube as f64 + 0.5) / tube_steps as f64;
            let normal = [
                (cross.cos() * around.cos()) as f32,
                (cross.cos() * around.sin()) as f32,
                cross.sin() as f32,
            ];
            faces.push(preview_face(
                &[
                    major * tube_steps + tube,
                    next_major * tube_steps + tube,
                    next_major * tube_steps + next_tube,
                    major * tube_steps + next_tube,
                ],
                normal,
            ));
        }
    }
    PreviewMesh { vertices, faces }
}

fn preview_face(indices: &[usize], normal: [f32; 3]) -> PreviewFace {
    PreviewFace {
        indices: indices.to_vec(),
        normal: normalize(normal),
    }
}

fn face_radial_normal(vertices: &[[f32; 3]], indices: &[usize]) -> [f32; 3] {
    let mut center = [0.0; 3];
    for index in indices {
        for (axis, coordinate) in center.iter_mut().enumerate() {
            *coordinate += vertices[*index][axis] / indices.len() as f32;
        }
    }
    normalize(center)
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
