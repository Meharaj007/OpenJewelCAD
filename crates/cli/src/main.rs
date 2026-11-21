mod gui;

use openjewelcad_cad_ffi::{Kernel, Shape};
use openjewelcad_geometry::LengthMm;
use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn MoveFileExW(existing: *const u16, replacement: *const u16, flags: u32) -> i32;
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum PrimitiveKind {
    Box,
    Cylinder,
    Sphere,
    Torus,
}

impl PrimitiveKind {
    fn name(self) -> &'static str {
        match self {
            Self::Box => "box",
            Self::Cylinder => "cylinder",
            Self::Sphere => "sphere",
            Self::Torus => "torus",
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        match name {
            "box" => Some(Self::Box),
            "cylinder" => Some(Self::Cylinder),
            "sphere" => Some(Self::Sphere),
            "torus" => Some(Self::Torus),
            _ => None,
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            Self::Box => "Box",
            Self::Cylinder => "Cylinder",
            Self::Sphere => "Sphere",
            Self::Torus => "Torus",
        }
    }

    fn parameter_names(self) -> &'static [&'static str] {
        match self {
            Self::Box => &["width_mm", "depth_mm", "height_mm"],
            Self::Cylinder => &["radius_mm", "height_mm"],
            Self::Sphere => &["radius_mm"],
            Self::Torus => &["major_radius_mm", "tube_radius_mm"],
        }
    }

    fn parameter_labels(self) -> &'static [&'static str] {
        match self {
            Self::Box => &["Width", "Depth", "Height"],
            Self::Cylinder => &["Radius", "Height"],
            Self::Sphere => &["Radius"],
            Self::Torus => &["Major radius", "Tube radius"],
        }
    }

    fn defaults(self) -> [f64; 3] {
        match self {
            Self::Box => [10.0, 10.0, 2.0],
            Self::Cylinder => [5.0, 10.0, 0.0],
            Self::Sphere => [5.0, 0.0, 0.0],
            Self::Torus => [8.0, 2.0, 0.0],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PrimitiveModel {
    kind: PrimitiveKind,
    parameters_mm: [LengthMm; 3],
}

impl PrimitiveModel {
    fn new(kind: PrimitiveKind, values: [f64; 3]) -> Result<Self, Box<dyn Error>> {
        let names = kind.parameter_names();
        for (index, name) in names.iter().enumerate() {
            if !values[index].is_finite() || values[index] <= 0.0 {
                return Err(
                    format!("{name} must be a finite positive number of millimetres").into(),
                );
            }
        }
        if kind == PrimitiveKind::Torus && values[0] <= values[1] {
            return Err("torus major_radius_mm must be greater than tube_radius_mm".into());
        }

        let mut parameters_mm = [LengthMm::zero(); 3];
        for (index, _) in names.iter().enumerate() {
            parameters_mm[index] = LengthMm::new(values[index])?;
        }
        Ok(Self {
            kind,
            parameters_mm,
        })
    }

    fn values(self) -> [f64; 3] {
        self.parameters_mm.map(LengthMm::as_mm)
    }

    fn serialize(self) -> String {
        let mut content = format!(
            "OPENJEWELCAD_PROJECT=1\nfeature_type={}\n",
            self.kind.name()
        );
        for (index, name) in self.kind.parameter_names().iter().enumerate() {
            content.push_str(&format!(
                "{name}={:.17}\n",
                self.parameters_mm[index].as_mm()
            ));
        }
        content
    }

    fn parse(content: &str) -> Result<Self, Box<dyn Error>> {
        let mut fields = BTreeMap::new();
        for line in content.lines() {
            let (key, value) = line.split_once('=').ok_or("project has a malformed line")?;
            if fields.insert(key, value).is_some() {
                return Err(format!("duplicate project field: {key}").into());
            }
        }
        if fields.remove("OPENJEWELCAD_PROJECT") != Some("1") {
            return Err("unsupported or missing project version".into());
        }
        let kind_name = fields
            .remove("feature_type")
            .ok_or("project is missing feature_type")?;
        let kind = PrimitiveKind::from_name(kind_name)
            .ok_or_else(|| format!("unsupported feature type: {kind_name}"))?;
        if let Some(key) = fields
            .keys()
            .find(|key| !kind.parameter_names().contains(key))
        {
            return Err(format!("unknown project field for {}: {key}", kind.name()).into());
        }

        let mut values = [0.0; 3];
        for (index, name) in kind.parameter_names().iter().enumerate() {
            let value = fields
                .remove(name)
                .ok_or_else(|| format!("project is missing {name}"))?;
            values[index] = value
                .parse::<f64>()
                .map_err(|_| format!("invalid {name}"))?;
        }
        Self::new(kind, values)
    }

    fn create_shape(self, kernel: &Kernel) -> Result<Shape, openjewelcad_cad_ffi::KernelError> {
        let values = self.values();
        match self.kind {
            PrimitiveKind::Box => kernel.box_mm(values[0], values[1], values[2]),
            PrimitiveKind::Cylinder => kernel.cylinder_mm(values[0], values[1]),
            PrimitiveKind::Sphere => kernel.sphere_mm(values[0]),
            PrimitiveKind::Torus => kernel.torus_mm(values[0], values[1]),
        }
    }
}

fn save_model(path: &Path, model: PrimitiveModel) -> Result<(), Box<dyn Error>> {
    let temporary = temporary_path(path);
    fs::write(&temporary, model.serialize())?;
    if let Err(error) = replace_file(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(error.into());
    }
    Ok(())
}

fn temporary_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(format!(".tmp.{}", std::process::id()));
    PathBuf::from(name)
}

fn replace_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
        let destination: Vec<u16> = destination
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        // REPLACE_EXISTING | WRITE_THROUGH keeps saves atomic on the same volume.
        if unsafe { MoveFileExW(source.as_ptr(), destination.as_ptr(), 0x1 | 0x8) } == 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        fs::rename(source, destination)
    }
}

fn load_model(path: &Path) -> Result<PrimitiveModel, Box<dyn Error>> {
    PrimitiveModel::parse(&fs::read_to_string(path)?)
}

fn create_or_edit(
    kind: PrimitiveKind,
    action: &str,
    file: &str,
    values: [f64; 3],
) -> Result<(), Box<dyn Error>> {
    let path = Path::new(file);
    if action == "edit" {
        let existing = load_model(path)?;
        if existing.kind != kind {
            return Err(format!(
                "cannot edit a {} project with the {} command",
                existing.kind.name(),
                kind.name()
            )
            .into());
        }
    }
    save_model(path, PrimitiveModel::new(kind, values)?)?;
    inspect(path)
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.as_slice() {
        [] => launch_gui(),
        [command] if command == "gui" => launch_gui(),
        [command] if command == "--help" || command == "help" => {
            print_help();
            Ok(())
        }
        [command, action, file, a, b, c]
            if command == "box" && matches!(action.as_str(), "new" | "edit") =>
        {
            create_or_edit(
                PrimitiveKind::Box,
                action,
                file,
                [a.parse()?, b.parse()?, c.parse()?],
            )
        }
        [command, action, file, a, b]
            if matches!(command.as_str(), "cylinder" | "torus")
                && matches!(action.as_str(), "new" | "edit") =>
        {
            let kind = PrimitiveKind::from_name(command).expect("matched a known primitive");
            create_or_edit(kind, action, file, [a.parse()?, b.parse()?, 0.0])
        }
        [command, action, file, a]
            if command == "sphere" && matches!(action.as_str(), "new" | "edit") =>
        {
            create_or_edit(PrimitiveKind::Sphere, action, file, [a.parse()?, 0.0, 0.0])
        }
        [command, file] if command == "inspect" => inspect(Path::new(file)),
        _ => {
            print_help();
            Err("invalid command; run with --help to see available commands".into())
        }
    }
}

fn launch_gui() -> Result<(), Box<dyn Error>> {
    gui::launch().map_err(|error| -> Box<dyn Error> {
        format!("could not start Open Jewel CAD: {error}").into()
    })
}

fn print_help() {
    println!("Open Jewel CAD — parametric primitive editor\n\nCommands:\n  openjewelcad                 Open the desktop editor\n  openjewelcad gui             Open the desktop editor\n  openjewelcad box new <file> <width_mm> <depth_mm> <height_mm>\n  openjewelcad cylinder new <file> <radius_mm> <height_mm>\n  openjewelcad sphere new <file> <radius_mm>\n  openjewelcad torus new <file> <major_radius_mm> <tube_radius_mm>\n  Use edit instead of new to update an existing project of the same type.\n  openjewelcad inspect <file>\n\nThe desktop editor needs OPENJEWELCAD_KERNEL_DLL set to the built kernel DLL and its folder on PATH.");
}

fn inspect(path: &Path) -> Result<(), Box<dyn Error>> {
    let model = load_model(path)?;
    let kernel = Kernel::from_environment()?;
    let shape = model.create_shape(&kernel)?;
    println!("Project: {}", path.display());
    println!("Feature: {}", model.kind.display_name());
    for (index, label) in model.kind.parameter_labels().iter().enumerate() {
        println!("{label}: {:.3} mm", model.parameters_mm[index].as_mm());
    }
    println!("Volume: {:.3} mm³", shape.volume_mm3()?);
    println!(
        "OCCT validity: {}",
        if shape.is_valid()? {
            "valid"
        } else {
            "invalid"
        }
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_project_round_trips_parameters() {
        for (kind, values) in [
            (PrimitiveKind::Box, [2.5, 3.0, 4.25]),
            (PrimitiveKind::Cylinder, [2.5, 4.25, 0.0]),
            (PrimitiveKind::Sphere, [3.0, 0.0, 0.0]),
            (PrimitiveKind::Torus, [5.0, 1.25, 0.0]),
        ] {
            let model = PrimitiveModel::new(kind, values).unwrap();
            assert_eq!(PrimitiveModel::parse(&model.serialize()).unwrap(), model);
        }
    }

    #[test]
    fn rejects_invalid_dimensions_and_project_version() {
        assert!(PrimitiveModel::new(PrimitiveKind::Box, [0.0, 1.0, 1.0]).is_err());
        assert!(PrimitiveModel::new(PrimitiveKind::Torus, [2.0, 2.0, 0.0]).is_err());
        assert!(PrimitiveModel::parse("OPENJEWELCAD_PROJECT=9\n").is_err());
        assert!(PrimitiveModel::parse(
            "OPENJEWELCAD_PROJECT=1\nfeature_type=sphere\nradius_mm=2\nextra=1\n"
        )
        .is_err());
    }
}
