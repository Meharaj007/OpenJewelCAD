use super::{
    export_model, load_model, save_model, ArcFeature, BooleanFeature, BooleanOperation,
    LineFeature, MeshExportFormat, OpenPolylineFeature, PatternFeature, PatternMode,
    PolygonProfile, PrimitiveKind, PrimitiveModel, TransformParams,
};
use eframe::egui::{self, Color32, Pos2, Sense, Shape, Stroke, Vec2};
use openjewelcad_cad_ffi::TriangleMesh;
use rfd::FileDialog;
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const MAX_RECENT_PROJECTS: usize = 12;
const SURFACE: Color32 = Color32::from_rgb(24, 29, 37);
const CARD: Color32 = Color32::from_rgb(30, 36, 46);
const EDGE: Color32 = Color32::from_rgb(54, 64, 78);
const ACCENT: Color32 = Color32::from_rgb(71, 190, 170);

pub(super) fn launch() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([1000.0, 680.0]),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    eframe::run_native(
        "Open Jewel CAD",
        options,
        Box::new(|creation_context| {
            let mut visuals = egui::Visuals::dark();
            visuals.panel_fill = Color32::from_rgb(17, 21, 28);
            visuals.window_fill = Color32::from_rgb(25, 30, 39);
            visuals.extreme_bg_color = Color32::from_rgb(14, 18, 24);
            visuals.selection.bg_fill = ACCENT;
            visuals.selection.stroke = Stroke::new(1.0, Color32::from_rgb(166, 245, 224));
            visuals.hyperlink_color = ACCENT;
            creation_context.egui_ctx.set_visuals(visuals);
            Ok(Box::new(CadApp::new()))
        }),
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AppPage {
    ProjectManager,
    Editor,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ViewPreset {
    Isometric,
    Top,
    Front,
    Right,
    Custom,
}

struct CadApp {
    page: AppPage,
    kind: PrimitiveKind,
    parameters_mm: [f64; 3],
    profile: PolygonProfile,
    profile_rectangle_mm: [f64; 2],
    profile_polygon_sides: usize,
    profile_polygon_radius_mm: f64,
    revolve_inner_radius_mm: f64,
    revolve_outer_radius_mm: f64,
    revolve_height_mm: f64,
    arc: ArcFeature,
    line: LineFeature,
    open_polyline: OpenPolylineFeature,
    extrusion_direction: [f64; 3],
    revolution_angle_deg: f64,
    transform: TransformParams,
    material_density_g_cm3: f64,
    analysis: Option<ShapeAnalysis>,
    project_path: String,
    recent_projects: Vec<PathBuf>,
    project_folder: Option<PathBuf>,
    folder_projects: Vec<PathBuf>,
    folder_filter: String,
    yaw: f32,
    elevation: f32,
    zoom: f32,
    pan: Vec2,
    view_preset: ViewPreset,
    wireframe: bool,
    command_palette_open: bool,
    palette_focus_pending: bool,
    tool_search: String,
    boolean_enabled: bool,
    boolean_operation: BooleanOperation,
    tool_kind: PrimitiveKind,
    tool_parameters_mm: [f64; 3],
    tool_transform: TransformParams,
    array_enabled: bool,
    array_mode: PatternMode,
    array_copies: u32,
    array_step_mm: [f64; 3],
    array_angle_step_deg: f64,
    preview_mesh: Option<PreviewMesh>,
    status: String,
    status_is_error: bool,
}

#[derive(Clone, Copy)]
struct ShapeAnalysis {
    length_mm: f64,
    volume_mm3: f64,
    surface_area_mm2: f64,
    center_of_mass_mm: [f64; 3],
    valid: bool,
    bounds_mm: [f64; 6],
}

impl CadApp {
    fn new() -> Self {
        let (status, status_is_error) = match openjewelcad_cad_ffi::Kernel::from_environment() {
            Ok(_) => (
                "OCCT kernel ready. Create or open a project to begin.".into(),
                false,
            ),
            Err(error) => (format!("CAD kernel unavailable: {error}"), true),
        };
        let recent_projects = load_recent_projects();
        let project_folder = recent_projects
            .first()
            .and_then(|path| path.parent().map(Path::to_path_buf));
        let folder_projects = project_folder
            .as_deref()
            .and_then(|folder| scan_project_folder(folder).ok())
            .unwrap_or_default();
        Self {
            page: AppPage::ProjectManager,
            kind: PrimitiveKind::Box,
            parameters_mm: PrimitiveKind::Box.defaults(),
            profile: PolygonProfile::default(),
            profile_rectangle_mm: [10.0, 10.0],
            profile_polygon_sides: 6,
            profile_polygon_radius_mm: 5.0,
            revolve_inner_radius_mm: 3.0,
            revolve_outer_radius_mm: 5.0,
            revolve_height_mm: 5.0,
            arc: ArcFeature::default(),
            line: LineFeature::default(),
            open_polyline: OpenPolylineFeature::default(),
            extrusion_direction: [0.0, 0.0, 1.0],
            revolution_angle_deg: 360.0,
            transform: TransformParams::IDENTITY,
            material_density_g_cm3: 0.0,
            analysis: None,
            project_path: String::new(),
            recent_projects,
            project_folder,
            folder_projects,
            folder_filter: String::new(),
            yaw: 0.65,
            elevation: 0.42,
            zoom: 1.0,
            pan: Vec2::ZERO,
            view_preset: ViewPreset::Isometric,
            wireframe: false,
            command_palette_open: false,
            palette_focus_pending: false,
            tool_search: String::new(),
            boolean_enabled: false,
            boolean_operation: BooleanOperation::Difference,
            tool_kind: PrimitiveKind::Sphere,
            tool_parameters_mm: PrimitiveKind::Sphere.defaults(),
            tool_transform: TransformParams {
                translation_mm: [5.0, 5.0, 1.0],
                ..TransformParams::IDENTITY
            },
            array_enabled: false,
            array_mode: PatternMode::Linear,
            array_copies: 4,
            array_step_mm: [8.0, 0.0, 0.0],
            array_angle_step_deg: 45.0,
            preview_mesh: None,
            status,
            status_is_error,
        }
    }

    fn new_project(&mut self) {
        self.kind = PrimitiveKind::Box;
        self.parameters_mm = self.kind.defaults();
        self.profile = PolygonProfile::default();
        self.arc = ArcFeature::default();
        self.line = LineFeature::default();
        self.open_polyline = OpenPolylineFeature::default();
        self.extrusion_direction = [0.0, 0.0, 1.0];
        self.revolution_angle_deg = 360.0;
        self.transform = TransformParams::IDENTITY;
        self.material_density_g_cm3 = 0.0;
        self.analysis = None;
        self.preview_mesh = None;
        self.reset_view();
        self.boolean_enabled = false;
        self.array_enabled = false;
        self.project_path.clear();
        self.page = AppPage::Editor;
        self.set_status("New solid. Choose a tool, then set dimensions.", false);
    }

    fn open_project(&mut self) {
        let Some(path) = FileDialog::new()
            .add_filter("Open Jewel CAD project", &["ojcad"])
            .pick_file()
        else {
            return;
        };
        self.open_project_path(path);
    }

    fn open_project_path(&mut self, path: PathBuf) {
        match load_model(&path) {
            Ok(model) => {
                self.kind = model.kind;
                self.parameters_mm = model.values();
                self.profile = model.profile;
                self.arc = model.arc.unwrap_or_default();
                self.line = model.line.unwrap_or_default();
                self.open_polyline = model.open_polyline.unwrap_or_default();
                self.extrusion_direction = model.extrusion_direction;
                self.revolution_angle_deg = model.revolution_angle_deg;
                self.transform = model.transform;
                self.material_density_g_cm3 = model.material_density_g_cm3.unwrap_or(0.0);
                self.boolean_enabled = model.boolean.is_some();
                if let Some(boolean) = model.boolean {
                    self.boolean_operation = boolean.operation;
                    self.tool_kind = boolean.tool_kind;
                    self.tool_parameters_mm = boolean.values_mm();
                    self.tool_transform = boolean.tool_transform;
                }
                self.array_enabled = model.pattern.is_some();
                if let Some(pattern) = model.pattern {
                    self.array_mode = pattern.mode;
                    self.array_copies = pattern.copies;
                    self.array_step_mm = pattern.step_mm;
                    self.array_angle_step_deg = pattern.angle_step_deg;
                }
                self.analysis = None;
                self.preview_mesh = None;
                self.reset_view();
                self.project_path = path.to_string_lossy().into_owned();
                self.page = AppPage::Editor;
                match self.record_recent_project(path) {
                    Ok(()) => {
                        self.set_status("Project loaded. Regenerate to check it with OCCT.", false)
                    }
                    Err(error) => self.set_status(
                        format!("Project loaded; recent list could not be saved: {error}"),
                        true,
                    ),
                }
            }
            Err(error) => self.set_status(format!("Could not open project: {error}"), true),
        }
    }

    fn create_project_dialog(&mut self) {
        let Some(mut path) = FileDialog::new()
            .add_filter("Open Jewel CAD project", &["ojcad"])
            .set_file_name("jewelry_design.ojcad")
            .save_file()
        else {
            return;
        };
        if path.extension().is_none() {
            path.set_extension("ojcad");
        }
        let model = PrimitiveModel::new(PrimitiveKind::Box, PrimitiveKind::Box.defaults());
        match model.and_then(|model| save_model(&path, model).map_err(Into::into)) {
            Ok(()) => {
                self.kind = PrimitiveKind::Box;
                self.parameters_mm = self.kind.defaults();
                self.extrusion_direction = [0.0, 0.0, 1.0];
                self.revolution_angle_deg = 360.0;
                self.transform = TransformParams::IDENTITY;
                self.material_density_g_cm3 = 0.0;
                self.analysis = None;
                self.preview_mesh = None;
                self.reset_view();
                self.boolean_enabled = false;
                self.project_path = path.to_string_lossy().into_owned();
                self.page = AppPage::Editor;
                match self.record_recent_project(path) {
                    Ok(()) => self
                        .set_status("Project created. Your first solid is ready to edit.", false),
                    Err(error) => self.set_status(
                        format!("Project created; recent list could not be saved: {error}"),
                        true,
                    ),
                }
            }
            Err(error) => self.set_status(format!("Could not create project: {error}"), true),
        }
    }

    fn record_recent_project(&mut self, path: PathBuf) -> io::Result<()> {
        self.recent_projects
            .retain(|recent| !same_path(recent, &path));
        self.recent_projects.insert(0, path);
        self.recent_projects.truncate(MAX_RECENT_PROJECTS);
        store_recent_projects(&self.recent_projects)
    }

    fn remove_recent_project(&mut self, path: &Path) {
        self.recent_projects
            .retain(|recent| !same_path(recent, path));
        if let Err(error) = store_recent_projects(&self.recent_projects) {
            self.set_status(format!("Could not update recent projects: {error}"), true);
        }
    }

    fn choose_project_folder(&mut self) {
        let mut dialog = FileDialog::new();
        if let Some(folder) = &self.project_folder {
            dialog = dialog.set_directory(folder);
        }
        if let Some(folder) = dialog.pick_folder() {
            self.set_project_folder(folder);
        }
    }

    fn set_project_folder(&mut self, folder: PathBuf) {
        match scan_project_folder(&folder) {
            Ok(projects) => {
                self.project_folder = Some(folder);
                self.folder_projects = projects;
                self.set_status("Folder loaded. Choose a project to open it.", false);
            }
            Err(error) => self.set_status(format!("Could not read project folder: {error}"), true),
        }
    }

    fn refresh_project_folder(&mut self) {
        if let Some(folder) = self.project_folder.clone() {
            self.set_project_folder(folder);
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

    fn export_current_mesh(&mut self, format: MeshExportFormat) {
        let (label, extension, filter) = match format {
            MeshExportFormat::BinaryStl => ("STL", "stl", "STL mesh"),
            MeshExportFormat::Obj => ("OBJ", "obj", "OBJ mesh"),
        };
        let stem = if self.project_path.is_empty() {
            "open_jewel_model".to_owned()
        } else {
            Path::new(&self.project_path)
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        };
        let Some(mut path) = FileDialog::new()
            .add_filter(filter, &[extension])
            .set_file_name(format!("{stem}.{extension}"))
            .save_file()
        else {
            return;
        };
        if path.extension().is_none() {
            path.set_extension(extension);
        }
        let result = self
            .current_model()
            .and_then(|model| export_model(model, &path, format));
        match result {
            Ok(summary) => self.set_status(
                format!(
                    "Exported {label} · {} triangles · millimetres · 0.05 mm linear deflection",
                    summary.triangles
                ),
                false,
            ),
            Err(error) => self.set_status(format!("Could not export mesh: {error}"), true),
        }
    }

    fn write_project(&mut self, path: PathBuf) {
        match self
            .current_model()
            .and_then(|model| save_model(&path, model))
        {
            Ok(()) => {
                self.project_path = path.to_string_lossy().into_owned();
                match self.record_recent_project(path.clone()) {
                    Ok(()) => {
                        self.set_status(format!("Saved project to {}", path.display()), false)
                    }
                    Err(error) => self.set_status(
                        format!("Saved project; recent list could not be saved: {error}"),
                        true,
                    ),
                }
            }
            Err(error) => self.set_status(format!("Could not save project: {error}"), true),
        }
    }

    fn regenerate(&mut self) {
        let result: Result<(ShapeAnalysis, Option<PreviewMesh>), Box<dyn Error>> = (|| {
            let model = self.current_model()?;
            let kernel = openjewelcad_cad_ffi::Kernel::from_environment()?;
            let shape = model.create_shape(&kernel)?;
            let valid = shape.is_valid()?;
            let bounds_mm = shape.bounds_mm()?;
            let (length_mm, volume_mm3, surface_area_mm2, center_of_mass_mm) =
                if model.kind.is_curve() {
                    (shape.length_mm()?, 0.0, 0.0, [0.0; 3])
                } else {
                    let properties = shape.mass_properties()?;
                    (
                        0.0,
                        if model.kind.is_surface() {
                            0.0
                        } else {
                            properties[0]
                        },
                        properties[1],
                        [properties[2], properties[3], properties[4]],
                    )
                };
            let analysis = ShapeAnalysis {
                length_mm,
                volume_mm3,
                surface_area_mm2,
                center_of_mass_mm,
                valid,
                bounds_mm,
            };
            let mesh = if model.kind.is_curve() {
                None
            } else {
                Some(PreviewMesh::from_occt(shape.tessellate(0.05, 0.35)?)?)
            };
            Ok((analysis, mesh))
        })();

        match result {
            Ok((analysis, mesh)) => {
                self.analysis = Some(analysis);
                self.preview_mesh = mesh;
                let dimensions = analysis.bounds_mm;
                let feature_summary = match self.kind {
                    PrimitiveKind::LineCurve => {
                        format!("Open line · length {:.3} mm", analysis.length_mm)
                    }
                    PrimitiveKind::OpenPolylineCurve => {
                        format!("Open polyline · length {:.3} mm", analysis.length_mm)
                    }
                    PrimitiveKind::ArcCurve => {
                        format!("Open arc · length {:.3} mm", analysis.length_mm)
                    }
                    kind if kind.is_curve() => {
                        format!("Closed curve · length {:.3} mm", analysis.length_mm)
                    }
                    kind if kind.is_surface() => {
                        format!("Surface · area {:.3} mm²", analysis.surface_area_mm2)
                    }
                    _ => format!("Solid · volume {:.3} mm³", analysis.volume_mm3),
                };
                self.set_status(
                    format!(
                        "{} · {} · bounds {:.3} × {:.3} × {:.3} mm",
                        if analysis.valid { "Valid" } else { "Invalid" },
                        feature_summary,
                        dimensions[3] - dimensions[0],
                        dimensions[4] - dimensions[1],
                        dimensions[5] - dimensions[2],
                    ),
                    !analysis.valid,
                );
            }
            Err(error) => self.set_status(format!("Could not regenerate shape: {error}"), true),
        }
    }

    fn current_model(&self) -> Result<PrimitiveModel, Box<dyn Error>> {
        let mut model = PrimitiveModel::new(self.kind, self.parameters_mm)?
            .with_transform(self.transform)?
            .with_profile(self.profile)?
            .with_arc(self.kind.uses_arc().then_some(self.arc))?
            .with_line(self.kind.uses_line().then_some(self.line))?
            .with_open_polyline(self.kind.uses_open_polyline().then_some(self.open_polyline))?
            .with_extrusion_direction_if_supported(self.extrusion_direction)?
            .with_revolution_angle_if_supported(self.revolution_angle_deg)?
            .with_material_density(
                (self.material_density_g_cm3 > 0.0).then_some(self.material_density_g_cm3),
            )?;
        if self.boolean_enabled {
            let boolean = BooleanFeature::new(
                self.boolean_operation,
                self.tool_kind,
                self.tool_parameters_mm,
                self.tool_transform,
            )?;
            model = model.with_boolean(Some(boolean));
        }
        if self.array_enabled {
            let pattern = match self.array_mode {
                PatternMode::Linear => {
                    PatternFeature::linear(self.array_copies, self.array_step_mm)?
                }
                PatternMode::PolarZ => {
                    PatternFeature::polar_z(self.array_copies, self.array_angle_step_deg)?
                }
            };
            model = model.with_pattern(Some(pattern));
        }
        Ok(model)
    }

    fn current_array(&self) -> Option<PatternFeature> {
        if !self.array_enabled {
            return None;
        }
        match self.array_mode {
            PatternMode::Linear => PatternFeature::linear(self.array_copies, self.array_step_mm),
            PatternMode::PolarZ => {
                PatternFeature::polar_z(self.array_copies, self.array_angle_step_deg)
            }
        }
        .ok()
    }

    fn set_status(&mut self, message: impl Into<String>, is_error: bool) {
        self.status = message.into();
        self.status_is_error = is_error;
    }

    fn reset_view(&mut self) {
        self.yaw = 0.65;
        self.elevation = 0.42;
        self.zoom = 1.0;
        self.pan = Vec2::ZERO;
        self.view_preset = ViewPreset::Isometric;
    }
}

impl eframe::App for CadApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.page == AppPage::Editor
            && ui.input(|input| input.modifiers.command && input.key_pressed(egui::Key::K))
        {
            self.command_palette_open = true;
            self.palette_focus_pending = true;
            self.tool_search.clear();
        }
        ui.painter().rect_filled(
            ui.max_rect(),
            egui::CornerRadius::ZERO,
            Color32::from_rgb(17, 21, 28),
        );
        ui.vertical(|ui| {
            if self.page == AppPage::ProjectManager {
                self.show_project_manager(ui);
            } else {
                self.show_editor(ui);
            }
        });
        self.show_command_palette(ui.ctx());
    }
}

impl CadApp {
    fn show_project_manager(&mut self, ui: &mut egui::Ui) {
        egui::Frame::new()
            .fill(SURFACE)
            .inner_margin(egui::Margin::same(20))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("OJ").size(20.0).strong().color(ACCENT));
                    ui.vertical(|ui| {
                        ui.heading("Open Jewel CAD");
                        ui.label(egui::RichText::new("Project workspace").color(Color32::GRAY));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Open project…").clicked() {
                            self.open_project();
                        }
                        if ui.button("＋  New project").clicked() {
                            self.create_project_dialog();
                        }
                    });
                });
            });
        ui.add_space(14.0);

        let body_height = (ui.available_height() - 42.0).max(240.0);
        let content_width = ui.available_width();
        let left_width = ((content_width - 12.0) * 0.59)
            .max(300.0)
            .min(content_width - 312.0);
        let right_width = (content_width - left_width - 12.0).max(280.0);
        ui.allocate_ui_with_layout(
            Vec2::new(content_width, body_height),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.horizontal_top(|ui| {
                    ui.allocate_ui_with_layout(
                        Vec2::new(left_width, body_height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            egui::Frame::new()
                                .fill(CARD)
                                .stroke(Stroke::new(1.0, EDGE))
                                .corner_radius(egui::CornerRadius::same(12))
                                .inner_margin(16)
                                .show(ui, |ui| {
                                    ui.set_min_size(Vec2::new(
                                        (left_width - 32.0).max(100.0),
                                        (body_height - 32.0).max(100.0),
                                    ));
                                    ui.heading("Recent projects");
                                    ui.label("Pick up where you left off.");
                                    ui.add_space(8.0);
                                    if self.recent_projects.is_empty() {
                                        ui.add_space(20.0);
                                        ui.label("No recent projects yet.");
                                        ui.label("Create a project or open an existing .ojcad file.");
                                    } else {
                                        let recent = self.recent_projects.clone();
                                        egui::ScrollArea::vertical()
                                            .id_salt("recent-project-list")
                                            .auto_shrink([false, false])
                                            .max_height((body_height - 120.0).max(150.0))
                                            .show(ui, |ui| {
                                                for path in recent {
                                                    ui.add_space(4.0);
                                                    egui::Frame::new()
                                                        .fill(SURFACE)
                                                        .corner_radius(egui::CornerRadius::same(8))
                                                        .inner_margin(10)
                                                        .show(ui, |ui| {
                                                            ui.horizontal(|ui| {
                                                                ui.vertical(|ui| {
                                                                    ui.label(egui::RichText::new(file_name(&path)).strong());
                                                                    ui.label(egui::RichText::new(path.parent().unwrap_or(Path::new("")).display().to_string()).small().color(Color32::GRAY));
                                                                });
                                                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                                    if ui.small_button("Remove").clicked() {
                                                                        self.remove_recent_project(&path);
                                                                    }
                                                                    if ui.button("Open").clicked() {
                                                                        self.open_project_path(path.clone());
                                                                    }
                                                                });
                                                            });
                                                        });
                                                }
                                            });
                                    }
                                });
                        },
                    );

                    ui.add_space(12.0);
                    ui.allocate_ui_with_layout(
                        Vec2::new(right_width, body_height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            egui::Frame::new()
                                .fill(CARD)
                                .stroke(Stroke::new(1.0, EDGE))
                                .corner_radius(egui::CornerRadius::same(12))
                                .inner_margin(16)
                                .show(ui, |ui| {
                                    ui.set_min_size(Vec2::new(
                                        (right_width - 32.0).max(100.0),
                                        (body_height - 32.0).max(100.0),
                                    ));
                                    self.show_file_browser(ui, body_height);
                                });
                        },
                    );
                });
            },
        );
        self.show_status_bar(ui);
    }

    fn show_file_browser(&mut self, ui: &mut egui::Ui, body_height: f32) {
        ui.horizontal(|ui| {
            ui.heading("Project files");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_enabled(self.project_folder.is_some(), egui::Button::new("Refresh"))
                    .clicked()
                {
                    self.refresh_project_folder();
                }
                if ui.small_button("Choose folder…").clicked() {
                    self.choose_project_folder();
                }
            });
        });
        if let Some(folder) = &self.project_folder {
            ui.label(
                egui::RichText::new(folder.display().to_string())
                    .small()
                    .color(Color32::GRAY),
            );
            ui.add_space(6.0);
            ui.add(
                egui::TextEdit::singleline(&mut self.folder_filter).hint_text("Filter this folder"),
            );
            let filter = self.folder_filter.to_lowercase();
            let projects: Vec<_> = self
                .folder_projects
                .iter()
                .filter(|path| file_name(path).to_lowercase().contains(&filter))
                .cloned()
                .collect();
            egui::ScrollArea::vertical()
                .id_salt("folder-project-list")
                .auto_shrink([false, false])
                .max_height((body_height - 140.0).max(160.0))
                .show(ui, |ui| {
                    if projects.is_empty() {
                        ui.add_space(12.0);
                        ui.label(if self.folder_projects.is_empty() {
                            "No .ojcad projects in this folder."
                        } else {
                            "No projects match this filter."
                        });
                    }
                    for path in projects {
                        let name = file_name(&path);
                        if ui.selectable_label(false, format!("◇  {name}")).clicked() {
                            self.open_project_path(path);
                        }
                    }
                });
        } else {
            ui.add_space(12.0);
            ui.label("Choose a folder to browse its .ojcad projects.");
            ui.label("Your files stay where you saved them.");
        }
    }

    fn show_editor(&mut self, ui: &mut egui::Ui) {
        egui::Frame::new()
            .fill(SURFACE)
            .inner_margin(egui::Margin::symmetric(12, 8))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("← Projects").clicked() {
                        self.page = AppPage::ProjectManager;
                    }
                    ui.separator();
                    ui.vertical(|ui| {
                        let title = if self.project_path.is_empty() {
                            "Untitled project".to_owned()
                        } else {
                            file_name(Path::new(&self.project_path))
                        };
                        ui.label(egui::RichText::new(title).strong());
                        ui.label(
                            egui::RichText::new(&self.project_path)
                                .small()
                                .color(Color32::GRAY),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.menu_button("Export", |ui| {
                            if ui.button("Binary STL…").clicked() {
                                ui.close();
                                self.export_current_mesh(MeshExportFormat::BinaryStl);
                            }
                            if ui.button("Wavefront OBJ…").clicked() {
                                ui.close();
                                self.export_current_mesh(MeshExportFormat::Obj);
                            }
                        });
                        if ui.button("Regenerate").clicked() {
                            self.regenerate();
                        }
                        if ui.button("Find tool  Ctrl+K").clicked() {
                            self.command_palette_open = true;
                            self.palette_focus_pending = true;
                            self.tool_search.clear();
                        }
                        if ui.button("Save as…").clicked() {
                            self.save_project_as();
                        }
                        if ui.button("Save").clicked() {
                            self.save_project();
                        }
                        if ui.button("Open…").clicked() {
                            self.open_project();
                        }
                        if ui.button("New").clicked() {
                            self.new_project();
                        }
                    });
                });
            });

        let body_height = (ui.available_height() - 40.0).max(280.0);
        let full_width = ui.available_width();
        let toolbox_width = 172.0;
        let properties_width = 330.0;
        let preview_width = (full_width - toolbox_width - properties_width - 24.0).max(320.0);
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(toolbox_width, body_height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| self.show_toolbox(ui, toolbox_width, body_height),
            );
            ui.add_space(8.0);
            ui.allocate_ui_with_layout(
                Vec2::new(preview_width, body_height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| self.show_preview(ui),
            );
            ui.add_space(8.0);
            ui.allocate_ui_with_layout(
                Vec2::new(properties_width, body_height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| self.show_properties(ui, properties_width, body_height),
            );
        });
        self.show_status_bar(ui);
    }

    fn show_toolbox(&mut self, ui: &mut egui::Ui, panel_width: f32, panel_height: f32) {
        egui::Frame::new()
            .fill(CARD)
            .stroke(Stroke::new(1.0, EDGE))
            .corner_radius(egui::CornerRadius::same(10))
            .inner_margin(12)
            .show(ui, |ui| {
                ui.set_min_size(Vec2::new(
                    (panel_width - 24.0).max(100.0),
                    (panel_height - 24.0).max(100.0),
                ));
                ui.heading("Create");
                ui.label(
                    egui::RichText::new("Curve → surface → solid")
                        .small()
                        .color(Color32::GRAY),
                );
                ui.add_space(8.0);
                let previous_kind = self.kind;
                let mut selected_kind = None;
                egui::CollapsingHeader::new("1 · Curves")
                    .default_open(self.kind.is_curve())
                    .show(ui, |ui| {
                        if ui
                            .selectable_label(
                                self.kind == PrimitiveKind::PolylineCurve,
                                "Closed polyline",
                            )
                            .clicked()
                        {
                            selected_kind = Some(PrimitiveKind::PolylineCurve);
                        }
                        if ui
                            .selectable_label(self.kind == PrimitiveKind::LineCurve, "Line")
                            .clicked()
                        {
                            selected_kind = Some(PrimitiveKind::LineCurve);
                        }
                        if ui
                            .selectable_label(
                                self.kind == PrimitiveKind::OpenPolylineCurve,
                                "Open polyline",
                            )
                            .clicked()
                        {
                            selected_kind = Some(PrimitiveKind::OpenPolylineCurve);
                        }
                        if ui
                            .selectable_label(
                                self.kind == PrimitiveKind::BezierCurve,
                                "Cubic Bezier",
                            )
                            .clicked()
                        {
                            selected_kind = Some(PrimitiveKind::BezierCurve);
                        }
                        if ui
                            .selectable_label(self.kind == PrimitiveKind::ArcCurve, "Arc")
                            .clicked()
                        {
                            selected_kind = Some(PrimitiveKind::ArcCurve);
                        }
                        if ui
                            .selectable_label(self.kind == PrimitiveKind::CircleCurve, "Circle")
                            .clicked()
                        {
                            selected_kind = Some(PrimitiveKind::CircleCurve);
                        }
                        if ui
                            .selectable_label(self.kind == PrimitiveKind::EllipseCurve, "Ellipse")
                            .clicked()
                        {
                            selected_kind = Some(PrimitiveKind::EllipseCurve);
                        }
                    });
                egui::CollapsingHeader::new("2 · Surfaces")
                    .default_open(self.kind.is_surface())
                    .show(ui, |ui| {
                        if ui
                            .selectable_label(
                                self.kind == PrimitiveKind::PlanarSurface,
                                "Planar surface",
                            )
                            .clicked()
                        {
                            selected_kind = Some(PrimitiveKind::PlanarSurface);
                        }
                        if ui
                            .selectable_label(
                                self.kind == PrimitiveKind::PlanarCircleSurface,
                                "Circle disk",
                            )
                            .clicked()
                        {
                            selected_kind = Some(PrimitiveKind::PlanarCircleSurface);
                        }
                        if ui
                            .selectable_label(
                                self.kind == PrimitiveKind::PlanarEllipseSurface,
                                "Ellipse disk",
                            )
                            .clicked()
                        {
                            selected_kind = Some(PrimitiveKind::PlanarEllipseSurface);
                        }
                        for (kind, label) in [
                            (PrimitiveKind::LineSurface, "Line surface"),
                            (PrimitiveKind::OpenPolylineSurface, "Open path surface"),
                            (PrimitiveKind::BezierSurface, "Bezier surface"),
                            (
                                PrimitiveKind::RevolvedBezierSurface,
                                "Revolved Bezier surface",
                            ),
                            (PrimitiveKind::ArcSurface, "Arc surface"),
                            (PrimitiveKind::RevolvedSurface, "Revolved surface"),
                        ] {
                            if ui.selectable_label(self.kind == kind, label).clicked() {
                                selected_kind = Some(kind);
                            }
                        }
                    });
                egui::CollapsingHeader::new("3 · Solids")
                    .default_open(self.kind.is_solid())
                    .show(ui, |ui| {
                        let tools = [
                            (PrimitiveKind::Box, "Box"),
                            (PrimitiveKind::Cylinder, "Cylinder"),
                            (PrimitiveKind::Cone, "Cone"),
                            (PrimitiveKind::TruncatedCone, "Frustum"),
                            (PrimitiveKind::Ellipsoid, "Ellipsoid"),
                            (PrimitiveKind::Pipe, "Pipe"),
                            (PrimitiveKind::Sphere, "Sphere"),
                            (PrimitiveKind::Torus, "Torus"),
                            (PrimitiveKind::RingBand, "Ring band"),
                            (PrimitiveKind::RoundBrilliant, "Round brilliant"),
                            (PrimitiveKind::ExtrudedPolygon, "Extruded profile"),
                            (PrimitiveKind::ExtrudedCircle, "Extruded circle"),
                            (PrimitiveKind::ExtrudedEllipse, "Extruded ellipse"),
                            (PrimitiveKind::RevolvedSolid, "Revolved solid"),
                        ];
                        ui.columns(2, |columns| {
                            for (index, (kind, label)) in tools.into_iter().enumerate() {
                                if columns[index % 2]
                                    .selectable_label(self.kind == kind, label)
                                    .clicked()
                                {
                                    selected_kind = Some(kind);
                                }
                            }
                        });
                    });
                if let Some(kind) = selected_kind.filter(|kind| *kind != previous_kind) {
                    self.select_tool_kind(kind);
                }
                ui.add_space(12.0);
                ui.separator();
                ui.label(egui::RichText::new("View").strong());
                ui.label("Drag in the viewport to orbit.");
                ui.add(egui::Slider::new(&mut self.zoom, 0.4..=1.6).text("Zoom"));
                if ui.button("Fit model").clicked() {
                    self.zoom = 1.0;
                    self.pan = Vec2::ZERO;
                }
            });
    }

    fn show_properties(&mut self, ui: &mut egui::Ui, panel_width: f32, panel_height: f32) {
        egui::Frame::new()
            .fill(CARD)
            .stroke(Stroke::new(1.0, EDGE))
            .corner_radius(egui::CornerRadius::same(10))
            .inner_margin(12)
            .show(ui, |ui| {
                ui.set_min_size(Vec2::new(
                    (panel_width - 24.0).max(100.0),
                    (panel_height - 24.0).max(100.0),
                ));
                egui::ScrollArea::vertical()
                    .id_salt("editor-properties-panel")
                    .auto_shrink([false, false])
                    .max_height((panel_height - 24.0).max(120.0))
                    .show(ui, |ui| {
                ui.heading("Properties");
                ui.label(egui::RichText::new(self.kind.display_name()).color(ACCENT));
                if !self.kind.parameter_names().is_empty() {
                    ui.label(
                        egui::RichText::new("Dimensions · mm")
                            .small()
                            .color(Color32::GRAY),
                    );
                }
                ui.add_space(6.0);
                let mut changed = false;
                let mut next_profile_stage = None;
                let mut closed_profile_from_path = None;
                if matches!(
                    self.kind,
                    PrimitiveKind::RevolvedSurface | PrimitiveKind::RevolvedBezierSurface
                ) {
                    ui.label(egui::RichText::new("Surface of revolution").strong());
                    changed |= angle_field(
                        ui,
                        "Turn angle",
                        &mut self.revolution_angle_deg,
                        1.0..=360.0,
                    );
                    ui.label(
                        egui::RichText::new("Use 360° for a full turn.")
                            .small()
                            .color(Color32::GRAY),
                    );
                }
                for index in 0..self.kind.parameter_names().len() {
                    changed |= dimension_field(
                        ui,
                        self.kind.parameter_labels()[index],
                        &mut self.parameters_mm[index],
                    );
                }
                if self.kind.uses_vector_extrusion() {
                    ui.add_space(8.0);
                    ui.separator();
                    ui.label(egui::RichText::new("Extrusion direction · XYZ").strong());
                    ui.label(
                        egui::RichText::new("The height is the travel distance along this direction.")
                            .small()
                            .color(Color32::GRAY),
                    );
                    for (axis, component) in ["X", "Y", "Z"]
                        .into_iter()
                        .zip(self.extrusion_direction.iter_mut())
                    {
                        changed |= ui
                            .add(
                                egui::DragValue::new(component)
                                    .speed(0.05)
                                    .range(-10_000_000.0..=10_000_000.0)
                                    .prefix(format!("{axis} ")),
                            )
                            .changed();
                    }
                }
                let mut arc_changed = false;
                if self.kind.uses_arc() {
                    ui.add_space(8.0);
                    ui.separator();
                    ui.label(
                        egui::RichText::new(if self.kind == PrimitiveKind::ArcCurve {
                            "Open arc · XY mm"
                        } else {
                            "Arc surface source · XY mm"
                        })
                        .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Set the counter-clockwise start and sweep angles.")
                            .small()
                            .color(Color32::GRAY),
                    );
                    arc_changed |= angle_field(
                        ui,
                        "Start angle",
                        &mut self.arc.start_angle_deg,
                        -360.0..=360.0,
                    );
                    arc_changed |= angle_field(
                        ui,
                        "Sweep angle",
                        &mut self.arc.sweep_angle_deg,
                        0.1..=359.9,
                    );
                    if self.kind == PrimitiveKind::ArcCurve
                        && ui.button("Extrude curve to surface →").clicked()
                    {
                        next_profile_stage = Some(PrimitiveKind::ArcSurface);
                    }
                }
                changed |= arc_changed;
                let mut line_changed = false;
                if self.kind.uses_line() {
                    ui.add_space(8.0);
                    ui.separator();
                    ui.label(egui::RichText::new("Line endpoints · XY mm").strong());
                    for (label, point) in [
                        ("Start", &mut self.line.start_mm),
                        ("End", &mut self.line.end_mm),
                    ] {
                        for (axis, coordinate) in ["X", "Y"].into_iter().zip(point.iter_mut()) {
                            line_changed |= dimension_field(
                                ui,
                                &format!("{label} {axis}"),
                                coordinate,
                            );
                        }
                    }
                    if self.kind == PrimitiveKind::LineCurve
                        && ui.button("Reverse direction").clicked()
                    {
                        std::mem::swap(&mut self.line.start_mm, &mut self.line.end_mm);
                        line_changed = true;
                    }
                    if self.kind == PrimitiveKind::LineCurve
                        && ui.button("Extrude curve to surface →").clicked()
                    {
                        next_profile_stage = Some(PrimitiveKind::LineSurface);
                    }
                }
                changed |= line_changed;
                let mut open_polyline_changed = false;
                if self.kind.uses_open_polyline() {
                    let is_bezier = matches!(
                        self.kind,
                        PrimitiveKind::BezierCurve
                            | PrimitiveKind::BezierSurface
                            | PrimitiveKind::RevolvedBezierSurface
                    );
                    ui.add_space(8.0);
                    ui.separator();
                    ui.label(egui::RichText::new(if is_bezier { "Bezier control points · XY mm" } else { "Open path · XY mm" }).strong());
                    ui.label(
                        egui::RichText::new(if is_bezier {
                            "P1 and P4 are endpoints; P2 and P3 shape the curve."
                        } else {
                            "Edit the path in order; the last point stays open."
                        })
                            .small()
                            .color(Color32::GRAY),
                    );
                    if self.kind == PrimitiveKind::OpenPolylineCurve
                        && ui.button("Reverse direction").clicked()
                    {
                        self.open_polyline.reverse();
                        open_polyline_changed = true;
                    }
                    if self.kind == PrimitiveKind::OpenPolylineCurve
                        && ui.button("Close path →").clicked()
                    {
                        match super::closed_profile_from_open(self.open_polyline) {
                            Ok(profile) => {
                                closed_profile_from_path = Some(profile);
                                next_profile_stage = Some(PrimitiveKind::PolylineCurve);
                            }
                            Err(error) => self.set_status(error.to_string(), true),
                        }
                    }
                    let mut remove_vertex = None;
                    for index in 0..usize::from(self.open_polyline.point_count) {
                        let mut point = self.open_polyline.points_mm[index];
                        ui.horizontal(|ui| {
                            ui.label(format!("P{}", index + 1));
                            for (axis, coordinate) in ["X", "Y"].into_iter().zip(point.iter_mut()) {
                                ui.label(axis);
                                open_polyline_changed |= ui
                                    .add(egui::DragValue::new(coordinate).speed(0.1).suffix(" mm"))
                                    .changed();
                            }
                            if !is_bezier && ui.small_button("×").clicked() {
                                remove_vertex = Some(index);
                            }
                        });
                        if point != self.open_polyline.points_mm[index] {
                            self.open_polyline.set_point_unchecked(index, point);
                        }
                    }
                    if let Some(index) = remove_vertex {
                        match self.open_polyline.remove_point(index) {
                            Ok(()) => open_polyline_changed = true,
                            Err(error) => self.set_status(error.to_string(), true),
                        }
                    }
                    if !is_bezier && ui.button("Add point to path").clicked() {
                        match self.open_polyline.add_point() {
                            Ok(()) => open_polyline_changed = true,
                            Err(error) => self.set_status(error.to_string(), true),
                        }
                    }
                    if self.kind == PrimitiveKind::OpenPolylineCurve
                        && ui.button("Extrude curve to surface →").clicked()
                    {
                        next_profile_stage = Some(PrimitiveKind::OpenPolylineSurface);
                    }
                    if self.kind == PrimitiveKind::BezierCurve
                        && ui.button("Extrude curve to surface →").clicked()
                    {
                        next_profile_stage = Some(PrimitiveKind::BezierSurface);
                    }
                    if self.kind == PrimitiveKind::BezierCurve {
                        let positive_radius_profile = self
                            .open_polyline
                            .points()
                            .iter()
                            .all(|point| point[0] > 1e-9);
                        if ui
                            .add_enabled(
                                positive_radius_profile,
                                egui::Button::new("Revolve curve into surface →"),
                            )
                            .on_hover_text(
                                "Every Bezier control point must stay at positive X radius from the Y axis.",
                            )
                            .clicked()
                        {
                            next_profile_stage = Some(PrimitiveKind::RevolvedBezierSurface);
                        }
                    }
                    if self.kind == PrimitiveKind::OpenPolylineCurve {
                        let positive_radius_profile = self
                            .open_polyline
                            .points()
                            .iter()
                            .all(|point| point[0] > 1e-9);
                        if ui
                            .add_enabled(
                                positive_radius_profile,
                                egui::Button::new("Revolve curve into surface →"),
                            )
                            .on_hover_text("All profile points must be at positive X radius from the Y axis.")
                            .clicked()
                        {
                            next_profile_stage = Some(PrimitiveKind::RevolvedSurface);
                        }
                    }
                    if matches!(
                        self.kind,
                        PrimitiveKind::RevolvedSurface | PrimitiveKind::RevolvedBezierSurface
                    ) {
                        ui.label(
                            egui::RichText::new(
                                "Every profile point must stay at positive X radius from the Y axis.",
                            )
                            .small()
                            .color(Color32::GRAY),
                        );
                    }
                }
                changed |= open_polyline_changed;
                if self.kind == PrimitiveKind::Torus {
                    ui.label(
                        egui::RichText::new("Major radius must exceed tube radius.")
                            .small()
                            .color(Color32::GRAY),
                    );
                } else if self.kind == PrimitiveKind::TruncatedCone {
                    ui.label(
                        egui::RichText::new("Top and base radii must differ.")
                            .small()
                            .color(Color32::GRAY),
                    );
                } else if self.kind == PrimitiveKind::Pipe {
                    ui.label(
                        egui::RichText::new("Inner radius must be smaller than outer radius.")
                            .small()
                            .color(Color32::GRAY),
                    );
                } else if self.kind == PrimitiveKind::RoundBrilliant {
                    ui.label(
                        egui::RichText::new("Table diameter must be smaller than the girdle.")
                            .small()
                            .color(Color32::GRAY),
                    );
                }
                let mut profile_changed = false;
                if self.kind.uses_profile() || self.kind.is_circle_stage() {
                    ui.add_space(8.0);
                    ui.separator();
                    let (profile_title, profile_description) = match self.kind {
                        PrimitiveKind::PolylineCurve => (
                            "Closed curve · XY mm",
                            "Draw the outline with editable straight segments.",
                        ),
                        PrimitiveKind::PlanarSurface => (
                            "Surface boundary · XY mm",
                            "The OCCT planar face follows this closed curve.",
                        ),
                        PrimitiveKind::RevolvedSolid => (
                            "Revolve section · XY mm",
                            "X is the radius; the profile turns 360° around the Y axis.",
                        ),
                        PrimitiveKind::CircleCurve => (
                            "Circle · XY mm",
                            "A true analytic circle that can become a disk or cylinder.",
                        ),
                        PrimitiveKind::PlanarCircleSurface => (
                            "Circle surface · XY mm",
                            "The OCCT planar face follows this circular boundary.",
                        ),
                        PrimitiveKind::EllipseCurve => (
                            "Ellipse · XY mm",
                            "An exact analytic ellipse that can become a face or solid.",
                        ),
                        PrimitiveKind::PlanarEllipseSurface => (
                            "Ellipse surface · XY mm",
                            "The OCCT planar face follows this elliptical boundary.",
                        ),
                        PrimitiveKind::ExtrudedCircle => (
                            "Cylinder · XY mm",
                            "The cylinder is extruded from this exact circular profile.",
                        ),
                        PrimitiveKind::ExtrudedEllipse => (
                            "Elliptical prism · XY mm",
                            "Extruded along +Z from the exact saved ellipse radii.",
                        ),
                        _ => (
                            "Solid profile · XY mm",
                            "Extrude the closed profile along +Z.",
                        ),
                    };
                    ui.label(egui::RichText::new(profile_title).strong());
                    ui.label(
                        egui::RichText::new(profile_description)
                            .small()
                            .color(Color32::GRAY),
                    );
                    if self.kind == PrimitiveKind::PolylineCurve
                        && ui.button("Reverse direction").clicked()
                    {
                        let reversed = self.profile.points().iter().rev().copied().collect::<Vec<_>>();
                        match PolygonProfile::from_points(&reversed) {
                            Ok(profile) => {
                                self.profile = profile;
                                profile_changed = true;
                            }
                            Err(error) => self.set_status(error.to_string(), true),
                        }
                    }
                    let revolve_profile_valid = self
                        .profile
                        .points()
                        .iter()
                        .all(|point| point[0] > 1e-9);
                    if self.kind == PrimitiveKind::PolylineCurve {
                        ui.horizontal(|ui| {
                            if ui.button("Create planar surface →").clicked() {
                                next_profile_stage = Some(PrimitiveKind::PlanarSurface);
                            }
                            if ui
                                .add_enabled(
                                    revolve_profile_valid,
                                    egui::Button::new("Revolve to solid →"),
                                )
                                .on_hover_text("Build a revolve section that stays outside the Y axis first.")
                                .clicked()
                            {
                                next_profile_stage = Some(PrimitiveKind::RevolvedSolid);
                            }
                        });
                    } else if self.kind == PrimitiveKind::CircleCurve
                        && ui.button("Create planar disk →").clicked()
                    {
                        next_profile_stage = Some(PrimitiveKind::PlanarCircleSurface);
                    } else if self.kind == PrimitiveKind::EllipseCurve
                        && ui.button("Create planar ellipse →").clicked()
                    {
                        next_profile_stage = Some(PrimitiveKind::PlanarEllipseSurface);
                    } else if self.kind == PrimitiveKind::PlanarSurface {
                        ui.horizontal(|ui| {
                            if ui.button("Extrude to solid →").clicked() {
                                next_profile_stage = Some(PrimitiveKind::ExtrudedPolygon);
                            }
                            if ui
                                .add_enabled(
                                    revolve_profile_valid,
                                    egui::Button::new("Revolve to solid →"),
                                )
                                .on_hover_text("Build a revolve section that stays outside the Y axis first.")
                                .clicked()
                            {
                                next_profile_stage = Some(PrimitiveKind::RevolvedSolid);
                            }
                        });
                    } else if self.kind == PrimitiveKind::PlanarCircleSurface
                        && ui.button("Extrude to cylinder →").clicked()
                    {
                        next_profile_stage = Some(PrimitiveKind::ExtrudedCircle);
                    } else if self.kind == PrimitiveKind::PlanarEllipseSurface
                        && ui.button("Extrude to solid →").clicked()
                    {
                        next_profile_stage = Some(PrimitiveKind::ExtrudedEllipse);
                    }
                    if self.kind.uses_profile() {
                    if self.kind != PrimitiveKind::RevolvedSolid {
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("Profile constructors").strong());
                    ui.horizontal(|ui| {
                        ui.label("Rectangle");
                        for (axis, value) in ["W", "H"].into_iter().zip(self.profile_rectangle_mm.iter_mut()) {
                            ui.add(egui::DragValue::new(value).speed(0.1).range(0.1..=100_000.0).prefix(format!("{axis} ")).suffix(" mm"));
                        }
                    });
                    if ui.button("Create rectangle profile").clicked() {
                        match super::rectangle_profile(self.profile_rectangle_mm[0], self.profile_rectangle_mm[1]) {
                            Ok(profile) => {
                                self.profile = profile;
                                profile_changed = true;
                            }
                            Err(error) => self.set_status(error.to_string(), true),
                        }
                    }
                    ui.horizontal(|ui| {
                        ui.label("Regular polygon");
                        ui.add(egui::DragValue::new(&mut self.profile_polygon_sides).range(3..=64).prefix("Sides "));
                        ui.add(egui::DragValue::new(&mut self.profile_polygon_radius_mm).speed(0.1).range(0.1..=10_000.0).prefix("Radius ").suffix(" mm"));
                    });
                    if ui.button("Create regular polygon profile").clicked() {
                        match super::regular_polygon_profile(self.profile_polygon_sides, self.profile_polygon_radius_mm) {
                            Ok(profile) => {
                                self.profile = profile;
                                profile_changed = true;
                            }
                            Err(error) => self.set_status(error.to_string(), true),
                        }
                    }
                    }
                    ui.horizontal(|ui| {
                        ui.label("Lathe section");
                        for (label, value) in [
                            ("Inner", &mut self.revolve_inner_radius_mm),
                            ("Outer", &mut self.revolve_outer_radius_mm),
                            ("Height", &mut self.revolve_height_mm),
                        ] {
                            ui.add(
                                egui::DragValue::new(value)
                                    .speed(0.1)
                                    .range(0.1..=10_000.0)
                                    .prefix(format!("{label} "))
                                    .suffix(" mm"),
                            );
                        }
                    });
                    if ui.button("Build revolve section").clicked() {
                        match super::revolved_profile(
                            self.revolve_inner_radius_mm,
                            self.revolve_outer_radius_mm,
                            self.revolve_height_mm,
                        ) {
                            Ok(profile) => {
                                self.profile = profile;
                                profile_changed = true;
                            }
                            Err(error) => self.set_status(error.to_string(), true),
                        }
                    }
                        let mut remove_vertex = None;
                        for index in 0..usize::from(self.profile.point_count) {
                            let mut point = self.profile.points_mm[index];
                            ui.horizontal(|ui| {
                                ui.label(format!("P{}", index + 1));
                                let x_changed = ui
                                    .add(egui::DragValue::new(&mut point[0]).speed(0.1).suffix(" mm"))
                                    .changed();
                                let y_changed = ui
                                    .add(egui::DragValue::new(&mut point[1]).speed(0.1).suffix(" mm"))
                                    .changed();
                                if x_changed || y_changed {
                                    self.profile.set_point_unchecked(index, point);
                                    profile_changed = true;
                                }
                                if ui.small_button("×").clicked() {
                                    remove_vertex = Some(index);
                                }
                            });
                        }
                        if let Some(index) = remove_vertex {
                            match self.profile.remove_vertex(index) {
                                Ok(()) => profile_changed = true,
                                Err(error) => self.set_status(error.to_string(), true),
                            }
                        }
                        if ui
                            .button("Add vertex at longest edge")
                            .on_hover_text("Adds a new profile point halfway along the longest edge.")
                            .clicked()
                        {
                            match self.profile.add_vertex() {
                                Ok(()) => profile_changed = true,
                                Err(error) => self.set_status(error.to_string(), true),
                            }
                        }
                    }
                }
                changed |= profile_changed;
                if changed {
                    self.analysis = None;
                    self.preview_mesh = None;
                    self.set_status(if profile_changed {
                        "Profile changed. Regenerate to validate the curve, surface, or solid."
                    } else if arc_changed {
                        "Arc angles changed. Regenerate to validate the curve or surface."
                    } else if line_changed {
                        "Line endpoints changed. Regenerate to validate the curve."
                    } else if open_polyline_changed {
                        "Open path changed. Regenerate to validate the curve or surface."
                    } else {
                        "Dimensions changed. Regenerate to validate the feature."
                    }, false);
                }
                if let Some(next_kind) = next_profile_stage {
                    let circle_radius = self.parameters_mm[0];
                    let ellipse_radii = [self.parameters_mm[0], self.parameters_mm[1]];
                    let arc_radius = self.parameters_mm[0];
                    self.kind = next_kind;
                    self.parameters_mm = next_kind.defaults();
                    if next_kind == PrimitiveKind::PolylineCurve {
                        if let Some(profile) = closed_profile_from_path {
                            self.profile = profile;
                        }
                    }
                    if next_kind.is_circle_stage() {
                        self.parameters_mm[0] = circle_radius;
                    }
                    if next_kind.is_ellipse_stage() {
                        self.parameters_mm[0] = ellipse_radii[0];
                        self.parameters_mm[1] = ellipse_radii[1];
                    }
                    if next_kind == PrimitiveKind::ArcSurface {
                        self.parameters_mm[0] = arc_radius;
                    }
                    self.analysis = None;
                    self.preview_mesh = None;
                    self.set_status(
                        format!("{} created from the saved profile. Regenerate to validate it.", next_kind.display_name()),
                        false,
                    );
                }
                let mut transform_changed = false;
                egui::CollapsingHeader::new("Transform")
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new("Placement in millimetres and degrees")
                                .small()
                                .color(Color32::GRAY),
                        );
                        for (index, axis) in ["X", "Y", "Z"].into_iter().enumerate() {
                            transform_changed |= transform_field(
                                ui,
                                &format!("Move {axis}"),
                                &mut self.transform.translation_mm[index],
                                "mm",
                                0.1,
                            );
                        }
                        for (index, axis) in ["X", "Y", "Z"].into_iter().enumerate() {
                            transform_changed |= transform_field(
                                ui,
                                &format!("Rotate {axis}"),
                                &mut self.transform.rotation_deg[index],
                                "°",
                                1.0,
                            );
                        }
                        transform_changed |= transform_field(
                            ui,
                            "Uniform scale",
                            &mut self.transform.uniform_scale,
                            "×",
                            0.01,
                        );
                        ui.label(
                            egui::RichText::new("Mirror at world origin")
                                .small()
                                .color(Color32::GRAY),
                        );
                        ui.horizontal_wrapped(|ui| {
                            for (index, plane) in ["YZ · X=0", "XZ · Y=0", "XY · Z=0"]
                                .into_iter()
                                .enumerate()
                            {
                                transform_changed |= ui
                                    .checkbox(&mut self.transform.mirror_axes[index], plane)
                                    .changed();
                            }
                        });
                    });
                if transform_changed {
                    self.analysis = None;
                    self.preview_mesh = None;
                    self.set_status(
                        "Transform changed. Regenerate to validate the solid and update measurements.",
                        false,
                    );
                }
                if self.kind.is_solid() {
                let mut boolean_changed = false;
                egui::CollapsingHeader::new("Boolean tools")
                    .default_open(self.boolean_enabled)
                    .show(ui, |ui| {
                        boolean_changed |= ui
                            .checkbox(&mut self.boolean_enabled, "Combine with a tool solid")
                            .changed();
                        if self.boolean_enabled {
                            let operation_name = match self.boolean_operation {
                                BooleanOperation::Union => "Union",
                                BooleanOperation::Difference => "Difference",
                                BooleanOperation::Intersection => "Intersection",
                            };
                            egui::ComboBox::from_id_salt("boolean-operation")
                                .selected_text(operation_name)
                                .show_ui(ui, |ui| {
                                    for (operation, name) in [
                                        (BooleanOperation::Union, "Union"),
                                        (BooleanOperation::Difference, "Difference"),
                                        (BooleanOperation::Intersection, "Intersection"),
                                    ] {
                                        boolean_changed |= ui
                                            .selectable_value(
                                                &mut self.boolean_operation,
                                                operation,
                                                name,
                                            )
                                            .changed();
                                    }
                                });
                            egui::ComboBox::from_id_salt("boolean-tool-type")
                                .selected_text(self.tool_kind.display_name())
                                .show_ui(ui, |ui| {
                                    for tool_kind in [
                                        PrimitiveKind::Box,
                                        PrimitiveKind::Cylinder,
                                        PrimitiveKind::Cone,
                                        PrimitiveKind::TruncatedCone,
                                        PrimitiveKind::Ellipsoid,
                                        PrimitiveKind::Pipe,
                                        PrimitiveKind::Sphere,
                                        PrimitiveKind::Torus,
                                        PrimitiveKind::RingBand,
                                        PrimitiveKind::RoundBrilliant,
                                    ] {
                                        if ui
                                            .selectable_value(
                                                &mut self.tool_kind,
                                                tool_kind,
                                                tool_kind.display_name(),
                                            )
                                            .changed()
                                        {
                                            self.tool_parameters_mm = tool_kind.defaults();
                                            boolean_changed = true;
                                        }
                                    }
                                });
                            for index in 0..self.tool_kind.parameter_names().len() {
                                boolean_changed |= dimension_field(
                                    ui,
                                    self.tool_kind.parameter_labels()[index],
                                    &mut self.tool_parameters_mm[index],
                                );
                            }
                            ui.label(
                                egui::RichText::new("Tool position · mm")
                                    .small()
                                    .color(Color32::GRAY),
                            );
                            for (index, axis) in ["X", "Y", "Z"].into_iter().enumerate() {
                                boolean_changed |= transform_field(
                                    ui,
                                    &format!("Tool {axis}"),
                                    &mut self.tool_transform.translation_mm[index],
                                    "mm",
                                    0.1,
                                );
                            }
                            ui.label(
                                egui::RichText::new("Mirror tool at world origin")
                                    .small()
                                    .color(Color32::GRAY),
                            );
                            ui.horizontal_wrapped(|ui| {
                                for (index, plane) in ["YZ · X=0", "XZ · Y=0", "XY · Z=0"]
                                    .into_iter()
                                    .enumerate()
                                {
                                    boolean_changed |= ui
                                        .checkbox(
                                            &mut self.tool_transform.mirror_axes[index],
                                            plane,
                                        )
                                        .changed();
                                }
                            });
                        }
                    });
                if boolean_changed {
                    self.analysis = None;
                    self.preview_mesh = None;
                    self.set_status(
                        "Boolean inputs changed. Regenerate to calculate the exact result.",
                        false,
                    );
                }
                let mut array_changed = false;
                egui::CollapsingHeader::new("Array tools")
                    .default_open(self.array_enabled)
                    .show(ui, |ui| {
                        array_changed |= ui
                            .checkbox(&mut self.array_enabled, "Repeat finished shape")
                            .changed();
                        if self.array_enabled {
                            egui::ComboBox::from_id_salt("array-mode")
                                .selected_text(match self.array_mode {
                                    PatternMode::Linear => "Linear",
                                    PatternMode::PolarZ => "Circular · Z axis",
                                })
                                .show_ui(ui, |ui| {
                                    array_changed |= ui
                                        .selectable_value(
                                            &mut self.array_mode,
                                            PatternMode::Linear,
                                            "Linear",
                                        )
                                        .changed();
                                    array_changed |= ui
                                        .selectable_value(
                                            &mut self.array_mode,
                                            PatternMode::PolarZ,
                                            "Circular · Z axis",
                                        )
                                        .changed();
                                });
                            array_changed |= ui
                                .horizontal(|ui| {
                                    ui.label("Copies");
                                    ui.add(egui::DragValue::new(&mut self.array_copies).range(2..=64))
                                        .changed()
                                })
                                .inner;
                            match self.array_mode {
                                PatternMode::Linear => {
                                    ui.label(
                                        egui::RichText::new("Step between copies · mm")
                                            .small()
                                            .color(Color32::GRAY),
                                    );
                                    for (index, axis) in ["X", "Y", "Z"].into_iter().enumerate() {
                                        array_changed |= transform_field(
                                            ui,
                                            &format!("Step {axis}"),
                                            &mut self.array_step_mm[index],
                                            "mm",
                                            0.1,
                                        );
                                    }
                                }
                                PatternMode::PolarZ => {
                                    ui.label(
                                        egui::RichText::new("Copies rotate around world origin")
                                            .small()
                                            .color(Color32::GRAY),
                                    );
                                    array_changed |= ui
                                        .horizontal(|ui| {
                                            ui.label("Angle step");
                                            ui.add(
                                                egui::DragValue::new(
                                                    &mut self.array_angle_step_deg,
                                                )
                                                .speed(1.0)
                                                .suffix("°"),
                                            )
                                            .changed()
                                        })
                                        .inner;
                                }
                            }
                        }
                    });
                if array_changed {
                    self.analysis = None;
                    self.preview_mesh = None;
                    self.set_status(
                        "Array inputs changed. Regenerate to calculate exact copies.",
                        false,
                    );
                }
                }
                ui.add_space(12.0);
                ui.separator();
                egui::CollapsingHeader::new("Analysis")
                    .default_open(true)
                    .show(ui, |ui| {
                        let mut density_changed = false;
                        if self.kind.is_solid() {
                        ui.label(
                            egui::RichText::new("Optional density for metal weight estimates")
                                .small()
                                .color(Color32::GRAY),
                        );
                        density_changed = ui
                            .horizontal(|ui| {
                                ui.label("Density");
                                ui.add(
                                    egui::DragValue::new(&mut self.material_density_g_cm3)
                                        .speed(0.01)
                                        .range(0.0..=100.0)
                                        .suffix(" g/cm³"),
                                )
                                .changed()
                            })
                            .inner;
                        }
                        if density_changed {
                            self.set_status(
                                "Material density updated; the weight estimate is refreshed.",
                                false,
                            );
                        }
                        if let Some(analysis) = self.analysis {
                            let bounds = analysis.bounds_mm;
                            ui.label(
                                egui::RichText::new(if analysis.valid {
                                    "OCCT validity: valid"
                                } else {
                                    "OCCT validity: invalid"
                                })
                                .color(if analysis.valid {
                                    Color32::from_rgb(133, 206, 179)
                                } else {
                                    Color32::from_rgb(234, 129, 111)
                                }),
                            );
                            match self.kind {
                                kind if kind.is_curve() => {
                                    ui.label(format!("Curve length  {:.3} mm", analysis.length_mm));
                                }
                                kind if kind.is_surface() => {
                                    ui.label(format!("Surface area  {:.3} mm²", analysis.surface_area_mm2));
                                }
                                _ => {
                                    ui.label(format!("Volume  {:.3} mm³", analysis.volume_mm3));
                                    ui.label(format!("Surface area  {:.3} mm²", analysis.surface_area_mm2));
                                    ui.label(format!(
                                        "Center  {:.3}, {:.3}, {:.3} mm",
                                        analysis.center_of_mass_mm[0],
                                        analysis.center_of_mass_mm[1],
                                        analysis.center_of_mass_mm[2],
                                    ));
                                }
                            }
                            ui.label(format!(
                                "Size  {:.3} × {:.3} × {:.3} mm",
                                bounds[3] - bounds[0],
                                bounds[4] - bounds[1],
                                bounds[5] - bounds[2],
                            ));
                            ui.label(format!(
                                "Min  {:.3}, {:.3}, {:.3} mm",
                                bounds[0], bounds[1], bounds[2],
                            ));
                            if self.kind.is_solid() && self.material_density_g_cm3 > 0.0 {
                                let weight_g =
                                    analysis.volume_mm3 * self.material_density_g_cm3 / 1000.0;
                                ui.label(format!("Estimated metal weight  {weight_g:.3} g"));
                            } else if self.kind.is_solid() {
                                ui.label("Set density for weight estimate.");
                            }
                        } else {
                            ui.label("Regenerate for measurements.");
                        }
                    });
                ui.add_space(12.0);
                ui.separator();
                ui.label(egui::RichText::new("Document").strong());
                ui.label(egui::RichText::new(file_name(Path::new(&self.project_path))).small());
                if ui.button("Save project").clicked() {
                    self.save_project();
                }
                if ui.button("Save project as…").clicked() {
                    self.save_project_as();
                }
                    });
            });
    }

    fn select_tool_kind(&mut self, kind: PrimitiveKind) {
        let previous_kind = self.kind;
        if previous_kind == kind {
            return;
        }
        let circle_radius = self.parameters_mm[0];
        let ellipse_radii = [self.parameters_mm[0], self.parameters_mm[1]];
        let arc_radius = self.parameters_mm[0];
        let previous_revolution_angle_deg = self.revolution_angle_deg;
        let uses_surface_revolution = |kind| {
            matches!(
                kind,
                PrimitiveKind::RevolvedSurface | PrimitiveKind::RevolvedBezierSurface
            )
        };
        self.kind = kind;
        self.parameters_mm = kind.defaults();
        self.revolution_angle_deg =
            if uses_surface_revolution(previous_kind) && uses_surface_revolution(kind) {
                previous_revolution_angle_deg
            } else {
                360.0
            };
        if kind.uses_arc() && !previous_kind.uses_arc() {
            self.arc = ArcFeature::default();
        }
        if kind.uses_line() && !previous_kind.uses_line() {
            self.line = LineFeature::default();
        }
        let is_bezier = |kind| {
            matches!(
                kind,
                PrimitiveKind::BezierCurve
                    | PrimitiveKind::BezierSurface
                    | PrimitiveKind::RevolvedBezierSurface
            )
        };
        let entering_bezier = is_bezier(kind) && !is_bezier(previous_kind);
        let leaving_bezier = is_bezier(previous_kind) && !is_bezier(kind);
        if (kind.uses_open_polyline() && !previous_kind.uses_open_polyline())
            || entering_bezier
            || leaving_bezier
        {
            self.open_polyline = if kind == PrimitiveKind::RevolvedBezierSurface && entering_bezier
            {
                OpenPolylineFeature::revolved_bezier_default()
            } else if entering_bezier {
                OpenPolylineFeature::bezier_default()
            } else if kind == PrimitiveKind::RevolvedSurface {
                OpenPolylineFeature::revolved_surface_default()
            } else {
                OpenPolylineFeature::default()
            };
        }
        if previous_kind.is_circle_stage() && kind.is_circle_stage() {
            self.parameters_mm[0] = circle_radius;
        }
        if previous_kind.is_ellipse_stage() && kind.is_ellipse_stage() {
            self.parameters_mm[0] = ellipse_radii[0];
            self.parameters_mm[1] = ellipse_radii[1];
        }
        if previous_kind.uses_arc() && kind.uses_arc() {
            self.parameters_mm[0] = arc_radius;
        }
        if !(previous_kind.uses_profile() && kind.uses_profile()) {
            self.profile = if kind == PrimitiveKind::RevolvedSolid {
                super::default_revolved_profile()
            } else {
                PolygonProfile::default()
            };
        }
        if !kind.is_solid() {
            self.boolean_enabled = false;
            self.array_enabled = false;
        }
        self.analysis = None;
        self.preview_mesh = None;
        self.set_status(
            "Tool changed. Profile points carry from curve to surface to solid.",
            false,
        );
    }

    fn show_command_palette(&mut self, context: &egui::Context) {
        if !self.command_palette_open {
            return;
        }
        let mut selected_kind = None;
        let mut open = self.command_palette_open;
        egui::Window::new("Find a modeling tool")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(410.0)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(context, |ui| {
                ui.label("Search curves, surfaces, solids, and jewelry tools");
                let search = ui.add(
                    egui::TextEdit::singleline(&mut self.tool_search)
                        .hint_text("Try “circle”, “revolve”, or “ring”")
                        .desired_width(f32::INFINITY),
                );
                if self.palette_focus_pending {
                    search.request_focus();
                    self.palette_focus_pending = false;
                }
                ui.add_space(6.0);
                egui::ScrollArea::vertical()
                    .max_height(360.0)
                    .show(ui, |ui| {
                        let query = self.tool_search.to_lowercase();
                        let mut section = "";
                        for (group, kind) in available_tools() {
                            let label = kind.display_name();
                            if !query.is_empty()
                                && !label.to_lowercase().contains(&query)
                                && !group.to_lowercase().contains(&query)
                            {
                                continue;
                            }
                            if section != *group {
                                section = *group;
                                ui.add_space(4.0);
                                ui.label(egui::RichText::new(*group).small().color(ACCENT));
                            }
                            if ui
                                .selectable_label(self.kind == *kind, format!("  {label}"))
                                .clicked()
                            {
                                selected_kind = Some(*kind);
                            }
                        }
                        if !available_tools().iter().any(|(group, kind)| {
                            let label = kind.display_name().to_lowercase();
                            query.is_empty()
                                || label.contains(&query)
                                || group.to_lowercase().contains(&query)
                        }) {
                            ui.label("No matching tools");
                        }
                    });
                if ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                    let query = self.tool_search.to_lowercase();
                    selected_kind = available_tools()
                        .iter()
                        .find(|(group, kind)| {
                            let label = kind.display_name().to_lowercase();
                            query.is_empty()
                                || label.contains(&query)
                                || group.to_lowercase().contains(&query)
                        })
                        .map(|(_, kind)| *kind);
                }
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("Enter selects first match · Esc closes · Ctrl+K opens")
                        .small()
                        .color(Color32::GRAY),
                );
            });
        self.command_palette_open = open;
        if let Some(kind) = selected_kind {
            self.select_tool_kind(kind);
            self.command_palette_open = false;
        }
        if context.input(|input| input.key_pressed(egui::Key::Escape)) {
            self.command_palette_open = false;
        }
    }

    fn show_preview(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Viewport");
            ui.label(
                egui::RichText::new(if self.wireframe { "Edges" } else { "Shaded" })
                    .small()
                    .color(Color32::GRAY),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.selectable_label(self.wireframe, "Edges").clicked() {
                    self.wireframe = true;
                }
                if ui.selectable_label(!self.wireframe, "Shaded").clicked() {
                    self.wireframe = false;
                }
                ui.separator();
                for (preset, label, yaw, elevation) in [
                    (ViewPreset::Right, "Right", std::f32::consts::FRAC_PI_2, 0.0),
                    (ViewPreset::Front, "Front", 0.0, 0.0),
                    (ViewPreset::Top, "Top", 0.0, 1.55),
                    (ViewPreset::Isometric, "Iso", 0.65, 0.42),
                ] {
                    if ui
                        .selectable_label(self.view_preset == preset, label)
                        .clicked()
                    {
                        self.view_preset = preset;
                        self.yaw = yaw;
                        self.elevation = elevation;
                    }
                }
                if ui
                    .button("Fit")
                    .on_hover_text("Fit the whole model in the viewport")
                    .clicked()
                {
                    self.zoom = 1.0;
                    self.pan = Vec2::ZERO;
                }
            });
        });
        ui.add_space(6.0);
        let size = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(size, Sense::drag());
        if response.dragged() {
            let motion = response.drag_motion();
            if response.dragged_by(egui::PointerButton::Middle) {
                self.pan += motion;
                self.pan.x = self.pan.x.clamp(-rect.width() * 0.4, rect.width() * 0.4);
                self.pan.y = self.pan.y.clamp(-rect.height() * 0.4, rect.height() * 0.4);
            } else {
                self.yaw += motion.x * 0.012;
                self.elevation = (self.elevation + motion.y * 0.010).clamp(-1.55, 1.55);
                self.view_preset = ViewPreset::Custom;
            }
        }
        if response.hovered() {
            let scroll = ui.input(|input| input.smooth_scroll_delta.y);
            if scroll.abs() > f32::EPSILON {
                self.zoom = (self.zoom * (scroll * 0.0015).exp()).clamp(0.4, 1.6);
            }
        }
        draw_primitive_preview(
            &ui.painter().with_clip_rect(rect),
            rect,
            self.kind,
            self.parameters_mm,
            self.profile,
            self.arc,
            self.line,
            self.open_polyline,
            self.extrusion_direction,
            self.revolution_angle_deg,
            self.transform,
            self.yaw,
            self.elevation,
            self.zoom,
            self.pan,
            self.wireframe,
            self.preview_mesh.as_ref(),
            self.current_array(),
        );
    }

    fn show_status_bar(&self, ui: &mut egui::Ui) {
        let color = if self.status_is_error {
            Color32::from_rgb(234, 129, 111)
        } else {
            Color32::from_rgb(133, 206, 179)
        };
        egui::Frame::new()
            .fill(SURFACE)
            .inner_margin(egui::Margin::symmetric(12, 6))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("●").color(color));
                    ui.label(egui::RichText::new(&self.status).color(color));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new("Millimetres")
                                .small()
                                .color(Color32::GRAY),
                        );
                    });
                });
            });
    }
}

fn load_recent_projects() -> Vec<PathBuf> {
    let Some(file) = recent_projects_file() else {
        return Vec::new();
    };
    let Ok(contents) = fs::read_to_string(file) else {
        return Vec::new();
    };
    let mut projects: Vec<PathBuf> = Vec::new();
    for path in contents.lines().map(PathBuf::from) {
        if path.is_file() && !projects.iter().any(|existing| same_path(existing, &path)) {
            projects.push(path);
        }
        if projects.len() == MAX_RECENT_PROJECTS {
            break;
        }
    }
    projects
}

fn available_tools() -> &'static [(&'static str, PrimitiveKind)] {
    use PrimitiveKind as K;
    &[
        ("Curves", K::PolylineCurve),
        ("Curves", K::LineCurve),
        ("Curves", K::OpenPolylineCurve),
        ("Curves", K::BezierCurve),
        ("Curves", K::ArcCurve),
        ("Curves", K::CircleCurve),
        ("Curves", K::EllipseCurve),
        ("Surfaces", K::PlanarSurface),
        ("Surfaces", K::PlanarCircleSurface),
        ("Surfaces", K::PlanarEllipseSurface),
        ("Surfaces", K::LineSurface),
        ("Surfaces", K::OpenPolylineSurface),
        ("Surfaces", K::BezierSurface),
        ("Surfaces", K::RevolvedBezierSurface),
        ("Surfaces", K::ArcSurface),
        ("Surfaces", K::RevolvedSurface),
        ("Solids", K::Box),
        ("Solids", K::Cylinder),
        ("Solids", K::Cone),
        ("Solids", K::TruncatedCone),
        ("Solids", K::Ellipsoid),
        ("Solids", K::Pipe),
        ("Solids", K::Sphere),
        ("Solids", K::Torus),
        ("Solids", K::RingBand),
        ("Jewelry", K::RoundBrilliant),
        ("Solids", K::ExtrudedPolygon),
        ("Solids", K::ExtrudedCircle),
        ("Solids", K::ExtrudedEllipse),
        ("Solids", K::RevolvedSolid),
    ]
}

fn store_recent_projects(projects: &[PathBuf]) -> io::Result<()> {
    let Some(file) = recent_projects_file() else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "application data folder is unavailable",
        ));
    };
    let folder = file.parent().expect("recent-project path has a parent");
    fs::create_dir_all(folder)?;
    let contents = projects
        .iter()
        .take(MAX_RECENT_PROJECTS)
        .map(|path| path.to_string_lossy())
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(file, contents)
}

fn recent_projects_file() -> Option<PathBuf> {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .map(|folder| folder.join("OpenJewelCAD").join("recent_projects.txt"))
}

fn same_path(left: &Path, right: &Path) -> bool {
    #[cfg(windows)]
    {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    }
    #[cfg(not(windows))]
    {
        left == right
    }
}

fn scan_project_folder(folder: &Path) -> io::Result<Vec<PathBuf>> {
    let mut projects = fs::read_dir(folder)?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let is_project = path.is_file()
                && path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("ojcad"));
            is_project.then_some(path)
        })
        .collect::<Vec<_>>();
    projects.sort_by_key(|path| file_name(path).to_lowercase());
    Ok(projects)
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned()
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

fn angle_field(
    ui: &mut egui::Ui,
    name: &str,
    value: &mut f64,
    range: std::ops::RangeInclusive<f64>,
) -> bool {
    ui.horizontal(|ui| {
        ui.label(name);
        ui.add(
            egui::DragValue::new(value)
                .speed(1.0)
                .range(range)
                .suffix("°"),
        )
        .changed()
    })
    .inner
}

fn transform_field(ui: &mut egui::Ui, name: &str, value: &mut f64, unit: &str, speed: f64) -> bool {
    ui.horizontal(|ui| {
        ui.label(name);
        let value = if unit == "×" {
            ui.add(
                egui::DragValue::new(value)
                    .speed(speed)
                    .range(0.000001..=1_000_000.0),
            )
        } else {
            ui.add(egui::DragValue::new(value).speed(speed))
        };
        ui.label(unit);
        value.changed()
    })
    .inner
}

#[derive(Clone, Copy)]
struct ProjectedPoint {
    position: Pos2,
    depth: f32,
}

#[derive(Clone)]
struct PreviewFace {
    indices: Vec<usize>,
    normal: [f32; 3],
}

#[derive(Clone)]
struct PreviewMesh {
    vertices: Vec<[f32; 3]>,
    faces: Vec<PreviewFace>,
}

impl PreviewMesh {
    fn from_occt(mesh: TriangleMesh) -> Result<Self, Box<dyn Error>> {
        if mesh.vertices_mm.is_empty() || mesh.triangles.is_empty() {
            return Err("OCCT returned an empty preview mesh".into());
        }
        let mut min = [f64::INFINITY; 3];
        let mut max = [f64::NEG_INFINITY; 3];
        for vertex in &mesh.vertices_mm {
            for axis in 0..3 {
                min[axis] = min[axis].min(vertex[axis]);
                max[axis] = max[axis].max(vertex[axis]);
            }
        }
        let extent = (0..3)
            .map(|axis| max[axis] - min[axis])
            .fold(0.0_f64, f64::max)
            .max(f64::EPSILON);
        let vertices: Vec<[f32; 3]> = mesh
            .vertices_mm
            .iter()
            .map(|point| point.map(|coordinate| (coordinate / extent) as f32))
            .collect();
        let mut faces = Vec::with_capacity(mesh.triangles.len());
        for triangle in mesh.triangles {
            let indices = triangle.map(|index| index as usize);
            if indices.iter().any(|index| *index >= vertices.len()) {
                return Err("OCCT mesh contains an invalid triangle index".into());
            }
            let a = vertices[indices[0]];
            let b = vertices[indices[1]];
            let c = vertices[indices[2]];
            let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
            let normal = [
                ab[1] * ac[2] - ab[2] * ac[1],
                ab[2] * ac[0] - ab[0] * ac[2],
                ab[0] * ac[1] - ab[1] * ac[0],
            ];
            faces.push(PreviewFace {
                indices: indices.to_vec(),
                normal: normalize(normal),
            });
        }
        Ok(Self { vertices, faces })
    }
}

fn draw_primitive_preview(
    painter: &egui::Painter,
    rect: egui::Rect,
    kind: PrimitiveKind,
    parameters: [f64; 3],
    profile: PolygonProfile,
    arc: ArcFeature,
    line: LineFeature,
    open_polyline: OpenPolylineFeature,
    extrusion_direction: [f64; 3],
    revolution_angle_deg: f64,
    transform: TransformParams,
    yaw: f32,
    elevation: f32,
    zoom: f32,
    pan: Vec2,
    wireframe: bool,
    cached_mesh: Option<&PreviewMesh>,
    pattern: Option<PatternFeature>,
) {
    let background = Color32::from_rgb(19, 24, 31);
    painter.rect_filled(rect, egui::CornerRadius::ZERO, background);
    let grid_stroke = Stroke::new(1.0, Color32::from_rgb(31, 39, 49));
    let grid_spacing = 40.0;
    let mut x = (rect.left() / grid_spacing).floor() * grid_spacing;
    while x <= rect.right() {
        painter.line_segment(
            [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
            grid_stroke,
        );
        x += grid_spacing;
    }
    let mut y = (rect.top() / grid_spacing).floor() * grid_spacing;
    while y <= rect.bottom() {
        painter.line_segment(
            [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
            grid_stroke,
        );
        y += grid_spacing;
    }
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
        || (kind == PrimitiveKind::Torus && parameters[0] <= parameters[1])
        || (kind == PrimitiveKind::TruncatedCone && parameters[0] == parameters[1])
        || (kind == PrimitiveKind::Pipe && parameters[1] >= parameters[0])
        || (kind == PrimitiveKind::RoundBrilliant && parameters[2] >= parameters[0])
        || (kind.uses_line() && LineFeature::new(line.start_mm, line.end_mm).is_err())
        || (kind.uses_open_polyline()
            && OpenPolylineFeature::from_points(open_polyline.points()).is_err())
        || (kind == PrimitiveKind::RevolvedSurface
            && open_polyline.points().iter().any(|point| point[0] <= 1e-9))
        || (kind == PrimitiveKind::RevolvedBezierSurface
            && open_polyline.points().iter().any(|point| point[0] <= 1e-9))
        || (matches!(
            kind,
            PrimitiveKind::RevolvedSurface | PrimitiveKind::RevolvedBezierSurface
        ) && (!revolution_angle_deg.is_finite()
            || revolution_angle_deg <= 0.0
            || revolution_angle_deg > 360.0))
        || (kind == PrimitiveKind::RevolvedSolid
            && profile.points().iter().any(|point| point[0] <= 1e-9))
        || (kind.uses_profile() && super::validate_polygon_profile(profile.points()).is_err())
        || (kind.uses_vector_extrusion()
            && super::validate_extrusion_direction(extrusion_direction).is_err())
        || (matches!(
            kind,
            PrimitiveKind::ExtrudedPolygon
                | PrimitiveKind::ExtrudedCircle
                | PrimitiveKind::ExtrudedEllipse
        ) && extrusion_direction[2].abs()
            <= extrusion_direction[0]
                .hypot(extrusion_direction[1])
                .hypot(extrusion_direction[2])
                * 1e-12);
    if cached_mesh.is_none() && invalid {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            if kind.is_curve() {
                "Enter valid curve values to preview the curve"
            } else if kind.is_surface() {
                "Enter valid dimensions to preview the surface"
            } else {
                "Enter valid positive dimensions to preview this solid"
            },
            egui::FontId::proportional(18.0),
            Color32::from_rgb(230, 170, 115),
        );
        return;
    }

    let mesh = if let Some(mesh) = cached_mesh {
        mesh.clone()
    } else {
        let mut mesh = make_preview_mesh(
            kind,
            parameters,
            profile,
            arc,
            line,
            open_polyline,
            extrusion_direction,
            revolution_angle_deg,
        );
        transform_preview_mesh(
            &mut mesh,
            kind,
            parameters,
            profile,
            line,
            open_polyline,
            extrusion_direction,
            transform,
        );
        if let Some(pattern) = pattern {
            mesh = array_preview_mesh(
                &mesh,
                pattern,
                preview_model_extent(
                    kind,
                    parameters,
                    profile,
                    line,
                    open_polyline,
                    extrusion_direction,
                ),
            );
        }
        mesh
    };
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
    let camera_points: Vec<[f32; 3]> = mesh
        .vertices
        .iter()
        .map(|point| [dot(*point, right), dot(*point, up), dot(*point, camera)])
        .collect();
    let (min_x, max_x, min_y, max_y) = camera_points.iter().fold(
        (
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::INFINITY,
            f32::NEG_INFINITY,
        ),
        |(min_x, max_x, min_y, max_y), point| {
            (
                min_x.min(point[0]),
                max_x.max(point[0]),
                min_y.min(point[1]),
                max_y.max(point[1]),
            )
        },
    );
    let padding = 24.0_f32.min(rect.width() * 0.1).min(rect.height() * 0.1);
    let available_width = (rect.width() - padding * 2.0).max(1.0);
    let available_height = (rect.height() - padding * 2.0).max(1.0);
    let model_width = (max_x - min_x).max(f32::EPSILON);
    let model_height = (max_y - min_y).max(f32::EPSILON);
    let scale =
        (available_width / model_width).min(available_height / model_height) * zoom.clamp(0.4, 1.6);
    let model_center_x = (min_x + max_x) * 0.5;
    let model_center_y = (min_y + max_y) * 0.5;
    let center = rect.center()
        + Vec2::new(
            pan.x.clamp(-rect.width() * 0.4, rect.width() * 0.4),
            pan.y.clamp(-rect.height() * 0.4, rect.height() * 0.4),
        );
    let projected: Vec<ProjectedPoint> = camera_points
        .into_iter()
        .map(|point| ProjectedPoint {
            position: Pos2::new(
                center.x + (point[0] - model_center_x) * scale,
                center.y - (point[1] - model_center_y) * scale,
            ),
            depth: point[2],
        })
        .collect();

    if kind.is_curve() {
        let stroke = Stroke::new(2.0, Color32::from_rgb(99, 210, 193));
        let count = projected.len();
        let segment_count = if matches!(
            kind,
            PrimitiveKind::ArcCurve
                | PrimitiveKind::LineCurve
                | PrimitiveKind::OpenPolylineCurve
                | PrimitiveKind::BezierCurve
        ) {
            count.saturating_sub(1)
        } else {
            count
        };
        for index in 0..segment_count {
            let next = (index + 1) % count;
            painter.line_segment(
                [projected[index].position, projected[next].position],
                stroke,
            );
        }
        if kind == PrimitiveKind::PolylineCurve {
            for point in &projected {
                painter.circle_filled(point.position, 3.5, Color32::WHITE);
            }
        } else if kind == PrimitiveKind::LineCurve {
            for point in [projected.first(), projected.last()].into_iter().flatten() {
                painter.circle_filled(point.position, 3.5, Color32::WHITE);
            }
        } else if kind == PrimitiveKind::BezierCurve {
            for point in [projected.first(), projected.last()].into_iter().flatten() {
                painter.circle_filled(point.position, 3.5, Color32::WHITE);
            }
        } else if kind == PrimitiveKind::OpenPolylineCurve {
            for point in &projected {
                painter.circle_filled(point.position, 3.5, Color32::WHITE);
            }
        }
        painter.text(
            Pos2::new(rect.left() + 18.0, rect.bottom() - 20.0),
            egui::Align2::LEFT_CENTER,
            format!(
                "{} · mm · drag orbit · wheel zoom · middle-drag pan",
                kind.display_name()
            ),
            egui::FontId::proportional(13.0),
            Color32::from_rgb(168, 176, 188),
        );
        return;
    }

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
            .collect::<Vec<_>>();
        if wireframe {
            let stroke = Stroke::new(1.2, Color32::from_rgb(226, 183, 112));
            for index in 0..points.len() {
                painter.line_segment([points[index], points[(index + 1) % points.len()]], stroke);
            }
        } else {
            let brightness = 0.50 + 0.50 * dot(face.normal, light).max(0.0);
            let base_color = if kind == PrimitiveKind::RoundBrilliant {
                [91, 177, 204]
            } else {
                [211, 155, 76]
            };
            painter.add(Shape::convex_polygon(
                points,
                shade(base_color, brightness),
                Stroke::new(1.6, Color32::from_rgb(50, 39, 28)),
            ));
        }
    }

    painter.text(
        Pos2::new(rect.left() + 18.0, rect.bottom() - 20.0),
        egui::Align2::LEFT_CENTER,
        "Millimetres · drag orbit · wheel zoom · middle-drag pan",
        egui::FontId::proportional(13.0),
        Color32::from_rgb(168, 176, 188),
    );
}

fn array_preview_mesh(base: &PreviewMesh, pattern: PatternFeature, extent: f64) -> PreviewMesh {
    let vertices_per_copy = base.vertices.len();
    let mut vertices = Vec::with_capacity(vertices_per_copy * pattern.copies as usize);
    let mut faces = Vec::with_capacity(base.faces.len() * pattern.copies as usize);
    for copy in 0..pattern.copies {
        let offset = vertices.len();
        let angle = (f64::from(copy) * pattern.angle_step_deg).to_radians();
        let (sin_angle, cos_angle) = angle.sin_cos();
        for point in &base.vertices {
            let mut point = point.map(f64::from);
            match pattern.mode {
                PatternMode::Linear => {
                    for axis in 0..3 {
                        point[axis] +=
                            pattern.step_mm[axis] * f64::from(copy) / extent.max(f64::EPSILON);
                    }
                }
                PatternMode::PolarZ => {
                    let x = point[0] * cos_angle - point[1] * sin_angle;
                    let y = point[0] * sin_angle + point[1] * cos_angle;
                    point[0] = x;
                    point[1] = y;
                }
            }
            vertices.push(point.map(|coordinate| coordinate as f32));
        }
        for face in &base.faces {
            let mut normal = face.normal;
            if pattern.mode == PatternMode::PolarZ {
                let x = f64::from(normal[0]) * cos_angle - f64::from(normal[1]) * sin_angle;
                let y = f64::from(normal[0]) * sin_angle + f64::from(normal[1]) * cos_angle;
                normal[0] = x as f32;
                normal[1] = y as f32;
            }
            faces.push(PreviewFace {
                indices: face.indices.iter().map(|index| index + offset).collect(),
                normal,
            });
        }
    }
    PreviewMesh { vertices, faces }
}

fn make_preview_mesh(
    kind: PrimitiveKind,
    parameters: [f64; 3],
    profile: PolygonProfile,
    arc: ArcFeature,
    line: LineFeature,
    open_polyline: OpenPolylineFeature,
    extrusion_direction: [f64; 3],
    revolution_angle_deg: f64,
) -> PreviewMesh {
    match kind {
        PrimitiveKind::Box => box_preview_mesh(parameters),
        PrimitiveKind::Cylinder => cylinder_preview_mesh(parameters[0], parameters[1]),
        PrimitiveKind::Cone => cone_preview_mesh(parameters[0], parameters[1]),
        PrimitiveKind::TruncatedCone => {
            truncated_cone_preview_mesh(parameters[0], parameters[1], parameters[2])
        }
        PrimitiveKind::Ellipsoid => {
            ellipsoid_preview_mesh(parameters[0], parameters[1], parameters[2])
        }
        PrimitiveKind::Pipe => pipe_preview_mesh(parameters[0], parameters[1], parameters[2]),
        PrimitiveKind::Sphere => sphere_preview_mesh(parameters[0]),
        PrimitiveKind::Torus => torus_preview_mesh(parameters[0], parameters[1]),
        PrimitiveKind::RingBand => pipe_preview_mesh(
            parameters[0] * 0.5 + parameters[1],
            parameters[0] * 0.5,
            parameters[2],
        ),
        PrimitiveKind::RoundBrilliant => round_brilliant_preview_mesh(parameters),
        PrimitiveKind::LineCurve => {
            let extent = preview_extent(kind, parameters, profile, line, open_polyline);
            PreviewMesh {
                vertices: vec![
                    scaled_point([line.start_mm[0], line.start_mm[1], 0.0], extent),
                    scaled_point([line.end_mm[0], line.end_mm[1], 0.0], extent),
                ],
                faces: Vec::new(),
            }
        }
        PrimitiveKind::LineSurface => open_curve_surface_preview_mesh(
            &[line.start_mm, line.end_mm],
            parameters[0],
            extrusion_direction,
            preview_extent(kind, parameters, profile, line, open_polyline),
        ),
        PrimitiveKind::OpenPolylineCurve => {
            let extent = preview_extent(kind, parameters, profile, line, open_polyline);
            PreviewMesh {
                vertices: open_polyline
                    .points()
                    .iter()
                    .map(|point| scaled_point([point[0], point[1], 0.0], extent))
                    .collect(),
                faces: Vec::new(),
            }
        }
        PrimitiveKind::BezierCurve => {
            let extent = preview_extent(kind, parameters, profile, line, open_polyline);
            let points = sampled_bezier_points(open_polyline.points(), 48);
            PreviewMesh {
                vertices: points
                    .iter()
                    .map(|point| scaled_point([point[0], point[1], 0.0], extent))
                    .collect(),
                faces: Vec::new(),
            }
        }
        PrimitiveKind::OpenPolylineSurface => open_curve_surface_preview_mesh(
            open_polyline.points(),
            parameters[0],
            extrusion_direction,
            preview_extent(kind, parameters, profile, line, open_polyline),
        ),
        PrimitiveKind::BezierSurface => {
            let extent = preview_model_extent(
                kind,
                parameters,
                profile,
                line,
                open_polyline,
                extrusion_direction,
            );
            let points = sampled_bezier_points(open_polyline.points(), 48);
            open_curve_surface_preview_mesh(&points, parameters[0], extrusion_direction, extent)
        }
        PrimitiveKind::RevolvedSurface => revolved_open_polyline_surface_preview_mesh(
            open_polyline.points(),
            preview_extent(kind, parameters, profile, line, open_polyline),
            revolution_angle_deg,
        ),
        PrimitiveKind::RevolvedBezierSurface => {
            let points = sampled_bezier_points(open_polyline.points(), 48);
            revolved_open_polyline_surface_preview_mesh(
                &points,
                preview_extent(kind, parameters, profile, line, open_polyline),
                revolution_angle_deg,
            )
        }
        PrimitiveKind::PolylineCurve => polyline_curve_preview_mesh(profile),
        PrimitiveKind::ArcCurve => arc_curve_preview_mesh(
            parameters[0],
            arc,
            preview_extent(kind, parameters, profile, line, open_polyline),
        ),
        PrimitiveKind::ArcSurface => {
            let extent = preview_extent(kind, parameters, profile, line, open_polyline);
            let points = sampled_arc_points(parameters[0], arc);
            open_curve_surface_preview_mesh(&points, parameters[1], extrusion_direction, extent)
        }
        PrimitiveKind::CircleCurve => polyline_curve_preview_mesh(circle_profile(parameters[0])),
        PrimitiveKind::EllipseCurve => {
            polyline_curve_preview_mesh(ellipse_profile(parameters[0], parameters[1]))
        }
        PrimitiveKind::PlanarSurface => planar_surface_preview_mesh(profile),
        PrimitiveKind::PlanarCircleSurface => {
            planar_surface_preview_mesh(circle_profile(parameters[0]))
        }
        PrimitiveKind::PlanarEllipseSurface => {
            planar_surface_preview_mesh(ellipse_profile(parameters[0], parameters[1]))
        }
        PrimitiveKind::ExtrudedPolygon => {
            polygon_prism_preview_mesh(profile, parameters[0], extrusion_direction)
        }
        PrimitiveKind::ExtrudedCircle => polygon_prism_preview_mesh(
            circle_profile(parameters[0]),
            parameters[1],
            extrusion_direction,
        ),
        PrimitiveKind::ExtrudedEllipse => polygon_prism_preview_mesh(
            ellipse_profile(parameters[0], parameters[1]),
            parameters[2],
            extrusion_direction,
        ),
        PrimitiveKind::RevolvedSolid => revolved_profile_preview_mesh(profile),
    }
}

fn sampled_bezier_points(control_points: &[[f64; 2]], segments: usize) -> Vec<[f64; 2]> {
    if control_points.len() != 4 || segments == 0 {
        return Vec::new();
    }
    (0..=segments)
        .map(|step| {
            let t = step as f64 / segments as f64;
            let one_minus_t = 1.0 - t;
            let weights = [
                one_minus_t.powi(3),
                3.0 * one_minus_t.powi(2) * t,
                3.0 * one_minus_t * t.powi(2),
                t.powi(3),
            ];
            let mut point = [0.0; 2];
            for (control, weight) in control_points.iter().zip(weights) {
                point[0] += control[0] * weight;
                point[1] += control[1] * weight;
            }
            point
        })
        .collect()
}

fn open_curve_surface_preview_mesh(
    points: &[[f64; 2]],
    height_mm: f64,
    direction: [f64; 3],
    extent: f64,
) -> PreviewMesh {
    let length = direction[0].hypot(direction[1]).hypot(direction[2]);
    if !length.is_finite() || length <= 1e-12 || !height_mm.is_finite() {
        return PreviewMesh {
            vertices: Vec::new(),
            faces: Vec::new(),
        };
    }
    let offset = direction.map(|component| component / length * height_mm);
    let mut vertices = Vec::with_capacity(points.len().saturating_sub(1) * 4);
    let mut faces = Vec::with_capacity(points.len().saturating_sub(1));
    for segment in points.windows(2) {
        let base = vertices.len();
        vertices.extend([
            scaled_point([segment[0][0], segment[0][1], 0.0], extent),
            scaled_point([segment[1][0], segment[1][1], 0.0], extent),
            scaled_point(
                [
                    segment[1][0] + offset[0],
                    segment[1][1] + offset[1],
                    offset[2],
                ],
                extent,
            ),
            scaled_point(
                [
                    segment[0][0] + offset[0],
                    segment[0][1] + offset[1],
                    offset[2],
                ],
                extent,
            ),
        ]);
        let first = vertices[base];
        let second = vertices[base + 1];
        let third = vertices[base + 2];
        let ab = [
            second[0] - first[0],
            second[1] - first[1],
            second[2] - first[2],
        ];
        let ac = [
            third[0] - first[0],
            third[1] - first[1],
            third[2] - first[2],
        ];
        let normal = normalize([
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ]);
        faces.push(PreviewFace {
            indices: vec![base, base + 1, base + 2, base + 3],
            normal,
        });
    }
    PreviewMesh { vertices, faces }
}

fn revolved_open_polyline_surface_preview_mesh(
    points: &[[f64; 2]],
    extent: f64,
    angle_deg: f64,
) -> PreviewMesh {
    const SEGMENTS: usize = 48;
    let mut vertices = Vec::with_capacity(points.len().saturating_sub(1) * SEGMENTS * 4);
    let mut faces = Vec::with_capacity(points.len().saturating_sub(1) * SEGMENTS);
    let sweep = angle_deg.to_radians();
    for segment in points.windows(2) {
        for step in 0..SEGMENTS {
            let angle_a = sweep * step as f64 / SEGMENTS as f64;
            let angle_b = sweep * (step + 1) as f64 / SEGMENTS as f64;
            let point = |radius: f64, height: f64, angle: f64| {
                scaled_point(
                    [radius * angle.cos(), height, -radius * angle.sin()],
                    extent,
                )
            };
            let base = vertices.len();
            vertices.extend([
                point(segment[0][0], segment[0][1], angle_a),
                point(segment[1][0], segment[1][1], angle_a),
                point(segment[1][0], segment[1][1], angle_b),
                point(segment[0][0], segment[0][1], angle_b),
            ]);
            let first = vertices[base];
            let second = vertices[base + 1];
            let third = vertices[base + 2];
            let ab = [
                second[0] - first[0],
                second[1] - first[1],
                second[2] - first[2],
            ];
            let ac = [
                third[0] - first[0],
                third[1] - first[1],
                third[2] - first[2],
            ];
            faces.push(PreviewFace {
                indices: vec![base, base + 1, base + 2, base + 3],
                normal: normalize([
                    ab[1] * ac[2] - ab[2] * ac[1],
                    ab[2] * ac[0] - ab[0] * ac[2],
                    ab[0] * ac[1] - ab[1] * ac[0],
                ]),
            });
        }
    }
    PreviewMesh { vertices, faces }
}

fn revolved_profile_preview_mesh(profile: PolygonProfile) -> PreviewMesh {
    const SEGMENTS: usize = 48;
    let mut vertices = Vec::with_capacity(profile.points().len() * SEGMENTS * 4);
    let mut faces = Vec::with_capacity(profile.points().len() * SEGMENTS);
    for edge in 0..profile.points().len() {
        let a = profile.points()[edge];
        let b = profile.points()[(edge + 1) % profile.points().len()];
        for segment in 0..SEGMENTS {
            let angle_a = std::f64::consts::TAU * segment as f64 / SEGMENTS as f64;
            let angle_b = std::f64::consts::TAU * (segment + 1) as f64 / SEGMENTS as f64;
            let point = |radius: f64, height: f64, angle: f64| {
                [
                    (radius * angle.cos()) as f32,
                    height as f32,
                    (radius * angle.sin()) as f32,
                ]
            };
            let base = vertices.len();
            vertices.extend([
                point(a[0], a[1], angle_a),
                point(b[0], b[1], angle_a),
                point(b[0], b[1], angle_b),
                point(a[0], a[1], angle_b),
            ]);
            let first = vertices[base];
            let second = vertices[base + 1];
            let third = vertices[base + 2];
            let ab = [
                second[0] - first[0],
                second[1] - first[1],
                second[2] - first[2],
            ];
            let ac = [
                third[0] - first[0],
                third[1] - first[1],
                third[2] - first[2],
            ];
            let normal = normalize([
                ab[1] * ac[2] - ab[2] * ac[1],
                ab[2] * ac[0] - ab[0] * ac[2],
                ab[0] * ac[1] - ab[1] * ac[0],
            ]);
            faces.push(PreviewFace {
                indices: vec![base, base + 1, base + 2, base + 3],
                normal,
            });
        }
    }
    PreviewMesh { vertices, faces }
}

fn sampled_arc_points(radius: f64, arc: ArcFeature) -> Vec<[f64; 2]> {
    let segments = ((arc.sweep_angle_deg.abs() / 4.0).ceil() as usize).clamp(2, 90);
    let start = arc.start_angle_deg.to_radians();
    let sweep = arc.sweep_angle_deg.to_radians();
    (0..=segments)
        .map(|index| {
            let angle = start + sweep * index as f64 / segments as f64;
            [radius * angle.cos(), radius * angle.sin()]
        })
        .collect()
}

fn arc_curve_preview_mesh(radius: f64, arc: ArcFeature, extent: f64) -> PreviewMesh {
    let vertices = sampled_arc_points(radius, arc)
        .into_iter()
        .map(|point| scaled_point([point[0], point[1], 0.0], extent))
        .collect();
    PreviewMesh {
        vertices,
        faces: Vec::new(),
    }
}

fn transform_preview_mesh(
    mesh: &mut PreviewMesh,
    kind: PrimitiveKind,
    parameters: [f64; 3],
    profile: PolygonProfile,
    line: LineFeature,
    open_polyline: OpenPolylineFeature,
    extrusion_direction: [f64; 3],
    transform: TransformParams,
) {
    let extent = preview_model_extent(
        kind,
        parameters,
        profile,
        line,
        open_polyline,
        extrusion_direction,
    );
    let angles = transform.rotation_deg.map(f64::to_radians);
    let (sin_x, cos_x) = angles[0].sin_cos();
    let (sin_y, cos_y) = angles[1].sin_cos();
    let (sin_z, cos_z) = angles[2].sin_cos();
    for vertex in &mut mesh.vertices {
        let mut point = vertex.map(|coordinate| coordinate as f64 * transform.uniform_scale);

        let y = point[1] * cos_x - point[2] * sin_x;
        let z = point[1] * sin_x + point[2] * cos_x;
        point[1] = y;
        point[2] = z;

        let x = point[0] * cos_y + point[2] * sin_y;
        let z = -point[0] * sin_y + point[2] * cos_y;
        point[0] = x;
        point[2] = z;

        let x = point[0] * cos_z - point[1] * sin_z;
        let y = point[0] * sin_z + point[1] * cos_z;
        point[0] = x + transform.translation_mm[0] / extent;
        point[1] = y + transform.translation_mm[1] / extent;
        point[2] += transform.translation_mm[2] / extent;
        for (axis, mirrored) in transform.mirror_axes.into_iter().enumerate() {
            if mirrored {
                point[axis] = -point[axis];
            }
        }
        *vertex = point.map(|coordinate| coordinate as f32);
    }
    let reflection_parity = transform.mirror_axes.into_iter().filter(|on| *on).count() % 2;
    for face in &mut mesh.faces {
        let mut normal = face.normal.map(f64::from);
        let y = normal[1] * cos_x - normal[2] * sin_x;
        let z = normal[1] * sin_x + normal[2] * cos_x;
        normal[1] = y;
        normal[2] = z;
        let x = normal[0] * cos_y + normal[2] * sin_y;
        let z = -normal[0] * sin_y + normal[2] * cos_y;
        normal[0] = x;
        normal[2] = z;
        let x = normal[0] * cos_z - normal[1] * sin_z;
        let y = normal[0] * sin_z + normal[1] * cos_z;
        normal[0] = x;
        normal[1] = y;
        for (axis, mirrored) in transform.mirror_axes.into_iter().enumerate() {
            if mirrored {
                normal[axis] = -normal[axis];
            }
        }
        if reflection_parity == 1 {
            face.indices.reverse();
        }
        face.normal = normalize(normal.map(|coordinate| coordinate as f32));
    }
}

fn preview_extent(
    kind: PrimitiveKind,
    parameters: [f64; 3],
    profile: PolygonProfile,
    line: LineFeature,
    open_polyline: OpenPolylineFeature,
) -> f64 {
    match kind {
        PrimitiveKind::Box => parameters.into_iter().fold(0.0_f64, f64::max),
        PrimitiveKind::Cylinder | PrimitiveKind::Cone => parameters[0].max(parameters[1] * 0.5),
        PrimitiveKind::TruncatedCone => parameters[0].max(parameters[1]).max(parameters[2] * 0.5),
        PrimitiveKind::Ellipsoid => parameters.into_iter().fold(0.0_f64, f64::max),
        PrimitiveKind::Pipe => parameters[0].max(parameters[2] * 0.5),
        PrimitiveKind::Sphere => parameters[0],
        PrimitiveKind::Torus => parameters[0] + parameters[1],
        PrimitiveKind::RingBand => (parameters[0] * 0.5 + parameters[1]).max(parameters[2] * 0.5),
        PrimitiveKind::RoundBrilliant => parameters[0].max(parameters[1]),
        PrimitiveKind::LineCurve | PrimitiveKind::LineSurface => line
            .start_mm
            .into_iter()
            .chain(line.end_mm)
            .map(f64::abs)
            .fold(parameters[0].max(1.0), f64::max),
        PrimitiveKind::OpenPolylineCurve
        | PrimitiveKind::OpenPolylineSurface
        | PrimitiveKind::BezierCurve
        | PrimitiveKind::BezierSurface
        | PrimitiveKind::RevolvedBezierSurface
        | PrimitiveKind::RevolvedSurface => open_polyline
            .points()
            .iter()
            .flatten()
            .map(|value| value.abs())
            .fold(parameters[0].max(1.0), f64::max),
        PrimitiveKind::ArcCurve
        | PrimitiveKind::CircleCurve
        | PrimitiveKind::PlanarCircleSurface => parameters[0].max(1.0),
        PrimitiveKind::ArcSurface => parameters[0].max(parameters[1]).max(1.0),
        PrimitiveKind::EllipseCurve | PrimitiveKind::PlanarEllipseSurface => {
            parameters[0].max(parameters[1]).max(1.0)
        }
        PrimitiveKind::ExtrudedCircle => parameters[0].max(parameters[1] * 0.5).max(1.0),
        PrimitiveKind::ExtrudedEllipse => parameters[0]
            .max(parameters[1])
            .max(parameters[2] * 0.5)
            .max(1.0),
        PrimitiveKind::PolylineCurve | PrimitiveKind::PlanarSurface => profile
            .points()
            .iter()
            .flatten()
            .map(|value| value.abs())
            .fold(1.0, f64::max),
        PrimitiveKind::ExtrudedPolygon => profile
            .points()
            .iter()
            .flatten()
            .map(|value| value.abs())
            .fold(parameters[0], f64::max),
        PrimitiveKind::RevolvedSolid => profile
            .points()
            .iter()
            .flatten()
            .map(|value| value.abs())
            .fold(1.0, f64::max),
    }
}

fn preview_model_extent(
    kind: PrimitiveKind,
    parameters: [f64; 3],
    profile: PolygonProfile,
    line: LineFeature,
    open_polyline: OpenPolylineFeature,
    extrusion_direction: [f64; 3],
) -> f64 {
    let mut extent = preview_extent(kind, parameters, profile, line, open_polyline);
    let (profile, height) = match kind {
        PrimitiveKind::ExtrudedPolygon => (Some(profile), parameters[0]),
        PrimitiveKind::ExtrudedCircle => (Some(circle_profile(parameters[0])), parameters[1]),
        PrimitiveKind::ExtrudedEllipse => (
            Some(ellipse_profile(parameters[0], parameters[1])),
            parameters[2],
        ),
        PrimitiveKind::BezierSurface => (None, parameters[0]),
        _ => (None, 0.0),
    };
    if let Some(profile) = profile {
        let length = extrusion_direction[0]
            .hypot(extrusion_direction[1])
            .hypot(extrusion_direction[2]);
        if length.is_finite() && length > 1e-12 && height.is_finite() {
            let offset = extrusion_direction.map(|component| component / length * height);
            for point in profile.points() {
                extent = extent
                    .max(point[0].abs())
                    .max(point[1].abs())
                    .max((point[0] + offset[0]).abs())
                    .max((point[1] + offset[1]).abs())
                    .max(offset[2].abs());
            }
        }
    }
    if kind == PrimitiveKind::BezierSurface {
        let length = extrusion_direction[0]
            .hypot(extrusion_direction[1])
            .hypot(extrusion_direction[2]);
        if length.is_finite() && length > 1e-12 && parameters[0].is_finite() {
            let offset = extrusion_direction.map(|component| component / length * parameters[0]);
            for point in open_polyline.points() {
                extent = extent
                    .max(point[0].abs())
                    .max(point[1].abs())
                    .max((point[0] + offset[0]).abs())
                    .max((point[1] + offset[1]).abs())
                    .max(offset[2].abs());
            }
        }
    }
    extent.max(1e-6)
}

fn round_brilliant_preview_mesh(parameters: [f64; 3]) -> PreviewMesh {
    const SEGMENTS: usize = 32;
    let diameter = parameters[0];
    let height = parameters[1];
    let extent = diameter.max(height);
    let radius = diameter * 0.5;
    let table_radius = parameters[2] * 0.5;
    let ring_data = [
        (radius * 0.06, 0.0),
        (radius * 0.90, height * 0.43),
        (radius, height * 0.49),
        ((radius + table_radius) * 0.5, height * 0.77),
        (table_radius, height),
    ];
    let mut vertices = Vec::with_capacity(ring_data.len() * SEGMENTS);
    for (ring_radius, ring_height) in ring_data {
        for segment in 0..SEGMENTS {
            let angle = std::f64::consts::TAU * segment as f64 / SEGMENTS as f64;
            vertices.push(scaled_point(
                [
                    ring_radius * angle.cos(),
                    ring_radius * angle.sin(),
                    ring_height - height * 0.5,
                ],
                extent,
            ));
        }
    }

    let mut faces = Vec::with_capacity(SEGMENTS * 8);
    faces.push(preview_face(
        &(0..SEGMENTS).rev().collect::<Vec<_>>(),
        [0.0, 0.0, -1.0],
    ));
    for ring in 0..ring_data.len() - 1 {
        let lower = ring * SEGMENTS;
        let upper = (ring + 1) * SEGMENTS;
        for segment in 0..SEGMENTS {
            let next = (segment + 1) % SEGMENTS;
            faces.push(preview_triangle(
                &vertices,
                [lower + segment, lower + next, upper + next],
            ));
            faces.push(preview_triangle(
                &vertices,
                [lower + segment, upper + next, upper + segment],
            ));
        }
    }
    faces.push(preview_face(
        &(4 * SEGMENTS..5 * SEGMENTS).collect::<Vec<_>>(),
        [0.0, 0.0, 1.0],
    ));
    PreviewMesh { vertices, faces }
}

fn preview_triangle(vertices: &[[f32; 3]], indices: [usize; 3]) -> PreviewFace {
    let a = vertices[indices[0]];
    let b = vertices[indices[1]];
    let c = vertices[indices[2]];
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    preview_face(
        &indices,
        [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ],
    )
}

fn polygon_prism_preview_mesh(
    profile: PolygonProfile,
    height_mm: f64,
    direction: [f64; 3],
) -> PreviewMesh {
    let points = profile.points();
    let count = points.len();
    let length = direction[0].hypot(direction[1]).hypot(direction[2]);
    if !length.is_finite() || length <= 1e-12 || !height_mm.is_finite() {
        return PreviewMesh {
            vertices: Vec::new(),
            faces: Vec::new(),
        };
    }
    let offset = direction.map(|component| component / length * height_mm);
    let extent = vector_extrusion_preview_extent(profile, height_mm, direction);
    let mut vertices = Vec::with_capacity(count * 2);
    for point in points {
        vertices.push(scaled_point([point[0], point[1], 0.0], extent));
    }
    for point in points {
        vertices.push(scaled_point(
            [point[0] + offset[0], point[1] + offset[1], offset[2]],
            extent,
        ));
    }

    let mut faces = Vec::with_capacity(count * 4);
    let orientation = profile.signed_area().signum();
    for index in 0..count {
        let next = (index + 1) % count;
        let dx = points[next][0] - points[index][0];
        let dy = points[next][1] - points[index][1];
        let normal = if orientation > 0.0 {
            [
                dy * offset[2],
                -dx * offset[2],
                dx * offset[1] - dy * offset[0],
            ]
        } else {
            [
                -dy * offset[2],
                dx * offset[2],
                dy * offset[0] - dx * offset[1],
            ]
        };
        faces.push(preview_face(
            &[index, next, count + next, count + index],
            normal.map(|v| v as f32),
        ));
    }

    if let Some(triangles) = triangulate_polygon(points) {
        let top_matches_profile = orientation * offset[2] > 0.0;
        for triangle in triangles {
            let base = [triangle[0], triangle[1], triangle[2]];
            let top = [
                count + triangle[0],
                count + triangle[1],
                count + triangle[2],
            ];
            faces.push(preview_triangle(
                &vertices,
                if top_matches_profile {
                    [base[0], base[2], base[1]]
                } else {
                    base
                },
            ));
            faces.push(preview_triangle(
                &vertices,
                if top_matches_profile {
                    top
                } else {
                    [top[0], top[2], top[1]]
                },
            ));
        }
    }
    PreviewMesh { vertices, faces }
}

fn vector_extrusion_preview_extent(
    profile: PolygonProfile,
    height_mm: f64,
    direction: [f64; 3],
) -> f64 {
    let length = direction[0].hypot(direction[1]).hypot(direction[2]);
    if !length.is_finite() || length <= 1e-12 || !height_mm.is_finite() {
        return 1.0;
    }
    let offset = direction.map(|component| component / length * height_mm);
    profile.points().iter().fold(1e-6_f64, |extent, point| {
        extent
            .max(point[0].abs())
            .max(point[1].abs())
            .max((point[0] + offset[0]).abs())
            .max((point[1] + offset[1]).abs())
            .max(offset[2].abs())
    })
}

fn polyline_curve_preview_mesh(profile: PolygonProfile) -> PreviewMesh {
    let extent = preview_extent(
        PrimitiveKind::PolylineCurve,
        [0.0; 3],
        profile,
        LineFeature::default(),
        OpenPolylineFeature::default(),
    );
    let vertices = profile
        .points()
        .iter()
        .map(|point| scaled_point([point[0], point[1], 0.0], extent))
        .collect();
    PreviewMesh {
        vertices,
        faces: Vec::new(),
    }
}

fn circle_profile(radius_mm: f64) -> PolygonProfile {
    ellipse_profile(radius_mm, radius_mm)
}

fn ellipse_profile(radius_x_mm: f64, radius_y_mm: f64) -> PolygonProfile {
    let mut profile = PolygonProfile {
        point_count: 64,
        points_mm: [[0.0; 2]; super::MAX_PROFILE_POINTS],
    };
    for index in 0..64 {
        let angle = std::f64::consts::TAU * index as f64 / 64.0;
        profile.points_mm[index] = [radius_x_mm * angle.cos(), radius_y_mm * angle.sin()];
    }
    profile
}

fn planar_surface_preview_mesh(profile: PolygonProfile) -> PreviewMesh {
    let points = profile.points();
    let extent = preview_extent(
        PrimitiveKind::PlanarSurface,
        [0.0; 3],
        profile,
        LineFeature::default(),
        OpenPolylineFeature::default(),
    );
    let vertices = points
        .iter()
        .map(|point| scaled_point([point[0], point[1], 0.0], extent))
        .collect::<Vec<_>>();
    let faces = triangulate_polygon(points)
        .unwrap_or_default()
        .into_iter()
        .map(|triangle| preview_triangle(&vertices, triangle))
        .collect();
    PreviewMesh { vertices, faces }
}

fn triangulate_polygon(points: &[[f64; 2]]) -> Option<Vec<[usize; 3]>> {
    const EPSILON: f64 = 1e-10;
    let orientation = super::polygon_signed_area(points).signum();
    if orientation == 0.0 {
        return None;
    }
    let mut remaining = (0..points.len()).collect::<Vec<_>>();
    let mut triangles = Vec::with_capacity(points.len().saturating_sub(2));
    while remaining.len() > 3 {
        let mut removed = false;
        for position in 0..remaining.len() {
            let previous = remaining[(position + remaining.len() - 1) % remaining.len()];
            let current = remaining[position];
            let next = remaining[(position + 1) % remaining.len()];
            let cross = super::polygon_cross(points[previous], points[current], points[next]);
            if cross.abs() <= EPSILON {
                remaining.remove(position);
                removed = true;
                break;
            }
            if cross * orientation <= EPSILON {
                continue;
            }
            let contains_vertex = remaining.iter().copied().any(|candidate| {
                candidate != previous
                    && candidate != current
                    && candidate != next
                    && point_in_triangle(
                        points[candidate],
                        points[previous],
                        points[current],
                        points[next],
                        orientation,
                    )
            });
            if contains_vertex {
                continue;
            }
            if orientation > 0.0 {
                triangles.push([previous, current, next]);
            } else {
                triangles.push([next, current, previous]);
            }
            remaining.remove(position);
            removed = true;
            break;
        }
        if !removed {
            return None;
        }
    }
    let triangle = [remaining[0], remaining[1], remaining[2]];
    let cross = super::polygon_cross(
        points[triangle[0]],
        points[triangle[1]],
        points[triangle[2]],
    );
    if cross.abs() <= EPSILON {
        return None;
    }
    triangles.push(if orientation > 0.0 {
        triangle
    } else {
        [triangle[2], triangle[1], triangle[0]]
    });
    Some(triangles)
}

fn point_in_triangle(
    point: [f64; 2],
    a: [f64; 2],
    b: [f64; 2],
    c: [f64; 2],
    orientation: f64,
) -> bool {
    const EPSILON: f64 = 1e-10;
    super::polygon_cross(a, b, point) * orientation >= -EPSILON
        && super::polygon_cross(b, c, point) * orientation >= -EPSILON
        && super::polygon_cross(c, a, point) * orientation >= -EPSILON
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

fn cone_preview_mesh(radius: f64, height: f64) -> PreviewMesh {
    let segments = 40;
    let extent = radius.max(height * 0.5);
    let mut vertices = Vec::with_capacity(segments + 1);
    for segment in 0..segments {
        let angle = std::f64::consts::TAU * segment as f64 / segments as f64;
        vertices.push(scaled_point(
            [radius * angle.cos(), radius * angle.sin(), -height * 0.5],
            extent,
        ));
    }
    let apex = vertices.len();
    vertices.push(scaled_point([0.0, 0.0, height * 0.5], extent));

    let mut faces = Vec::with_capacity(segments + 1);
    for segment in 0..segments {
        let next = (segment + 1) % segments;
        let angle = std::f64::consts::TAU * (segment as f64 + 0.5) / segments as f64;
        faces.push(preview_face(
            &[segment, next, apex],
            [
                angle.cos() as f32,
                angle.sin() as f32,
                (radius / height) as f32,
            ],
        ));
    }
    faces.push(preview_face(
        &(0..segments).rev().collect::<Vec<_>>(),
        [0.0, 0.0, -1.0],
    ));
    PreviewMesh { vertices, faces }
}

fn truncated_cone_preview_mesh(base_radius: f64, top_radius: f64, height: f64) -> PreviewMesh {
    let segments = 40;
    let extent = base_radius.max(top_radius).max(height * 0.5);
    let mut vertices = Vec::with_capacity(segments * 2);
    for (radius, z) in [(base_radius, -height * 0.5), (top_radius, height * 0.5)] {
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
            [
                angle.cos() as f32,
                angle.sin() as f32,
                ((base_radius - top_radius) / height) as f32,
            ],
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

fn ellipsoid_preview_mesh(radius_x: f64, radius_y: f64, radius_z: f64) -> PreviewMesh {
    let extent = radius_x.max(radius_y).max(radius_z);
    let mut mesh = sphere_preview_mesh(1.0);
    for vertex in &mut mesh.vertices {
        vertex[0] *= (radius_x / extent) as f32;
        vertex[1] *= (radius_y / extent) as f32;
        vertex[2] *= (radius_z / extent) as f32;
    }
    for face in &mut mesh.faces {
        face.normal = normalize([
            face.normal[0] * (extent / radius_x) as f32,
            face.normal[1] * (extent / radius_y) as f32,
            face.normal[2] * (extent / radius_z) as f32,
        ]);
    }
    mesh
}

fn pipe_preview_mesh(outer_radius: f64, inner_radius: f64, height: f64) -> PreviewMesh {
    let segments = 40;
    let extent = outer_radius.max(height * 0.5);
    let mut vertices = Vec::with_capacity(segments * 4);
    for (radius, z) in [
        (outer_radius, -height * 0.5),
        (outer_radius, height * 0.5),
        (inner_radius, -height * 0.5),
        (inner_radius, height * 0.5),
    ] {
        for segment in 0..segments {
            let angle = std::f64::consts::TAU * segment as f64 / segments as f64;
            vertices.push(scaled_point(
                [radius * angle.cos(), radius * angle.sin(), z],
                extent,
            ));
        }
    }

    let mut faces = Vec::with_capacity(segments * 4);
    for segment in 0..segments {
        let next = (segment + 1) % segments;
        let angle = std::f64::consts::TAU * (segment as f64 + 0.5) / segments as f64;
        let outward = [angle.cos() as f32, angle.sin() as f32, 0.0];
        faces.push(preview_face(
            &[segment, next, segments + next, segments + segment],
            outward,
        ));
        faces.push(preview_face(
            &[
                2 * segments + next,
                2 * segments + segment,
                3 * segments + segment,
                3 * segments + next,
            ],
            [-outward[0], -outward[1], 0.0],
        ));
        faces.push(preview_face(
            &[
                segments + segment,
                segments + next,
                3 * segments + next,
                3 * segments + segment,
            ],
            [0.0, 0.0, 1.0],
        ));
        faces.push(preview_face(
            &[next, segment, 2 * segments + segment, 2 * segments + next],
            [0.0, 0.0, -1.0],
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

#[cfg(test)]
mod preview_tests {
    use super::*;

    #[test]
    fn angled_closed_profile_preview_tracks_direction_and_stays_fitted() {
        let profile = circle_profile(2.0);
        let direction = [0.0, 3.0, 4.0];
        let mesh = polygon_prism_preview_mesh(profile, 5.0, direction);
        assert_eq!(mesh.vertices.len(), 128);
        assert!(mesh.faces.len() > 64);
        for vertex in &mesh.vertices {
            assert!(vertex.iter().all(|coordinate| coordinate.is_finite()));
            assert!(vertex
                .iter()
                .all(|coordinate| coordinate.abs() <= 1.0 + 1e-6));
        }

        let extent = vector_extrusion_preview_extent(profile, 5.0, direction);
        let first_top_vertex = mesh.vertices[64];
        for (actual, expected) in first_top_vertex.into_iter().zip([0.4, 0.6, 0.8]) {
            assert!((f64::from(actual) - expected).abs() < 1e-6);
        }
        assert!((extent - 5.0).abs() < 1e-9);
        assert!(mesh
            .faces
            .iter()
            .flat_map(|face| &face.indices)
            .all(|index| *index < mesh.vertices.len()));
    }

    #[test]
    fn bezier_preview_is_smooth_and_extruded_surface_stays_inside_viewport() {
        let controls = OpenPolylineFeature::bezier_default();
        let samples = sampled_bezier_points(controls.points(), 48);
        assert_eq!(samples.len(), 49);
        assert_eq!(samples[0], controls.points()[0]);
        assert_eq!(samples[48], controls.points()[3]);
        assert!(samples[12][1] > 1.0);
        assert!(samples[36][1] < -1.0);

        let direction = [0.0, 3.0, 4.0];
        let extent = preview_model_extent(
            PrimitiveKind::BezierSurface,
            [5.0, 0.0, 0.0],
            PolygonProfile::default(),
            LineFeature::default(),
            controls,
            direction,
        );
        assert!((extent - 9.0).abs() < 1e-9);
        let mesh = make_preview_mesh(
            PrimitiveKind::BezierSurface,
            [5.0, 0.0, 0.0],
            PolygonProfile::default(),
            ArcFeature::default(),
            LineFeature::default(),
            controls,
            direction,
            360.0,
        );
        assert_eq!(mesh.faces.len(), 48);
        assert!(mesh
            .vertices
            .iter()
            .flatten()
            .all(|value| value.is_finite()));
        assert!(mesh
            .vertices
            .iter()
            .flatten()
            .all(|value| value.abs() <= 1.0 + 1e-6));
    }

    #[test]
    fn revolved_bezier_preview_is_fitted_and_has_surface_faces() {
        let controls = OpenPolylineFeature::revolved_bezier_default();
        let extent = preview_extent(
            PrimitiveKind::RevolvedBezierSurface,
            [0.0; 3],
            PolygonProfile::default(),
            LineFeature::default(),
            controls,
        );
        assert!((extent - 6.0).abs() < 1e-9);

        let mesh = make_preview_mesh(
            PrimitiveKind::RevolvedBezierSurface,
            [0.0; 3],
            PolygonProfile::default(),
            ArcFeature::default(),
            LineFeature::default(),
            controls,
            [0.0, 0.0, 1.0],
            90.0,
        );
        assert_eq!(mesh.faces.len(), 48 * 48);
        assert!(mesh
            .vertices
            .iter()
            .flatten()
            .all(|value| value.is_finite() && value.abs() <= 1.0 + 1e-6));
        assert!(
            mesh.vertices
                .iter()
                .map(|point| point[2])
                .fold(0.0_f32, f32::min)
                < -0.8
        );
        assert!(
            mesh.vertices
                .iter()
                .map(|point| point[2])
                .fold(0.0_f32, f32::max)
                .abs()
                < 1e-6
        );
    }
}
