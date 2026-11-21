mod gui;

use openjewelcad_cad_ffi::Kernel;
use openjewelcad_geometry::LengthMm;
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
struct BoxModel {
    width_mm: LengthMm,
    depth_mm: LengthMm,
    height_mm: LengthMm,
}

impl BoxModel {
    fn new(width: f64, depth: f64, height: f64) -> Result<Self, Box<dyn Error>> {
        for (label, value) in [("width", width), ("depth", depth), ("height", height)] {
            if !value.is_finite() || value <= 0.0 {
                return Err(
                    format!("{label} must be a finite positive number of millimetres").into(),
                );
            }
        }
        Ok(Self {
            width_mm: LengthMm::new(width)?,
            depth_mm: LengthMm::new(depth)?,
            height_mm: LengthMm::new(height)?,
        })
    }

    fn serialize(self) -> String {
        format!(
            "OPENJEWELCAD_PROJECT=1\nfeature_type=box\nwidth_mm={:.17}\ndepth_mm={:.17}\nheight_mm={:.17}\n",
            self.width_mm.as_mm(), self.depth_mm.as_mm(), self.height_mm.as_mm()
        )
    }

    fn parse(content: &str) -> Result<Self, Box<dyn Error>> {
        let mut version = None;
        let mut feature_type = None;
        let mut width = None;
        let mut depth = None;
        let mut height = None;
        for line in content.lines() {
            let (key, value) = line.split_once('=').ok_or("project has a malformed line")?;
            let target = match key {
                "OPENJEWELCAD_PROJECT" => &mut version,
                "feature_type" => &mut feature_type,
                "width_mm" => &mut width,
                "depth_mm" => &mut depth,
                "height_mm" => &mut height,
                _ => return Err(format!("unknown project field: {key}").into()),
            };
            if target.replace(value).is_some() {
                return Err(format!("duplicate project field: {key}").into());
            }
        }
        if version != Some("1") {
            return Err("unsupported or missing project version".into());
        }
        if feature_type != Some("box") {
            return Err("unsupported or missing feature type".into());
        }
        let number = |value: Option<&str>, label: &str| -> Result<f64, Box<dyn Error>> {
            value
                .ok_or_else(|| -> Box<dyn Error> { format!("missing {label}").into() })?
                .parse::<f64>()
                .map_err(|_| format!("invalid {label}").into())
        };
        Self::new(
            number(width, "width_mm")?,
            number(depth, "depth_mm")?,
            number(height, "height_mm")?,
        )
    }
}

fn save_model(path: &Path, model: BoxModel) -> Result<(), Box<dyn Error>> {
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

fn load_model(path: &Path) -> Result<BoxModel, Box<dyn Error>> {
    BoxModel::parse(&fs::read_to_string(path)?)
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
        [command, action, file, width, depth, height]
            if command == "box" && (action == "new" || action == "edit") =>
        {
            let path = Path::new(file);
            if action == "edit" {
                load_model(path)?;
            }
            let updated = BoxModel::new(width.parse()?, depth.parse()?, height.parse()?)?;
            save_model(path, updated)?;
            inspect(path)
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
    println!("Open Jewel CAD — simple parametric box editor\n\nCommands:\n  openjewelcad                 Open the desktop editor\n  openjewelcad gui             Open the desktop editor\n  openjewelcad box new <file> <width_mm> <depth_mm> <height_mm>\n  openjewelcad box edit <file> <width_mm> <depth_mm> <height_mm>\n  openjewelcad inspect <file>\n\nThe desktop editor needs OPENJEWELCAD_KERNEL_DLL set to the built kernel DLL and its folder on PATH.");
}

fn inspect(path: &Path) -> Result<(), Box<dyn Error>> {
    let model = load_model(path)?;
    let kernel = Kernel::from_environment()?;
    let shape = kernel.box_mm(
        model.width_mm.as_mm(),
        model.depth_mm.as_mm(),
        model.height_mm.as_mm(),
    )?;
    println!("Project: {}", path.display());
    println!("Feature: Box");
    println!(
        "Size: {:.3} × {:.3} × {:.3} mm",
        model.width_mm.as_mm(),
        model.depth_mm.as_mm(),
        model.height_mm.as_mm()
    );
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
        let model = BoxModel::new(2.5, 3.0, 4.25).unwrap();
        assert_eq!(BoxModel::parse(&model.serialize()).unwrap(), model);
    }

    #[test]
    fn rejects_invalid_dimensions_and_project_version() {
        assert!(BoxModel::new(0.0, 1.0, 1.0).is_err());
        assert!(BoxModel::parse("OPENJEWELCAD_PROJECT=9\n").is_err());
    }
}
