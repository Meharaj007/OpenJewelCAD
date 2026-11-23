//! Safe Rust wrapper around the separately built Open Jewel CAD C ABI library.
//!
//! Set `OPENJEWELCAD_KERNEL_DLL` to the built kernel DLL before use. Shapes
//! retain the DLL for their lifetime and release their native handle on drop.

use std::env;
use std::error::Error;
use std::ffi::{c_char, c_void, CStr};
use std::fmt;
use std::path::Path;
use std::ptr::NonNull;
use std::sync::Arc;

type BoxCreate = unsafe extern "C" fn(f64, f64, f64, *mut *mut c_void, *mut CError) -> i32;
type CreateOne = unsafe extern "C" fn(f64, *mut *mut c_void, *mut CError) -> i32;
type CreateTwo = unsafe extern "C" fn(f64, f64, *mut *mut c_void, *mut CError) -> i32;
type CreateThree = unsafe extern "C" fn(f64, f64, f64, *mut *mut c_void, *mut CError) -> i32;
type CreateFour = unsafe extern "C" fn(f64, f64, f64, f64, *mut *mut c_void, *mut CError) -> i32;
type CreateFive =
    unsafe extern "C" fn(f64, f64, f64, f64, f64, *mut *mut c_void, *mut CError) -> i32;
type CreateSix =
    unsafe extern "C" fn(f64, f64, f64, f64, f64, f64, *mut *mut c_void, *mut CError) -> i32;
type CreateSeven =
    unsafe extern "C" fn(f64, f64, f64, f64, f64, f64, f64, *mut *mut c_void, *mut CError) -> i32;
type AbiVersion = unsafe extern "C" fn() -> u32;
type Volume = unsafe extern "C" fn(*const c_void, *mut f64, *mut CError) -> i32;
type IsValid = unsafe extern "C" fn(*const c_void, *mut i32, *mut CError) -> i32;
type Bounds = unsafe extern "C" fn(*const c_void, *mut f64, *mut CError) -> i32;
type MassProperties = unsafe extern "C" fn(*const c_void, *mut f64, *mut CError) -> i32;
type Length = unsafe extern "C" fn(*const c_void, *mut f64, *mut CError) -> i32;
type Transform = unsafe extern "C" fn(
    *const c_void,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    *mut *mut c_void,
    *mut CError,
) -> i32;
type Mirror = unsafe extern "C" fn(*const c_void, u32, *mut *mut c_void, *mut CError) -> i32;
type Boolean = unsafe extern "C" fn(
    *const c_void,
    *const c_void,
    i32,
    f64,
    *mut *mut c_void,
    *mut CError,
) -> i32;
type LinearArray =
    unsafe extern "C" fn(*const c_void, u32, f64, f64, f64, *mut *mut c_void, *mut CError) -> i32;
type PolarArrayZ =
    unsafe extern "C" fn(*const c_void, u32, f64, *mut *mut c_void, *mut CError) -> i32;
type ExtrudedPolygon =
    unsafe extern "C" fn(*const f64, u32, f64, *mut *mut c_void, *mut CError) -> i32;
type ExtrudedPolylineVector =
    unsafe extern "C" fn(*const f64, u32, f64, f64, f64, f64, *mut *mut c_void, *mut CError) -> i32;
type PolygonFeature = unsafe extern "C" fn(*const f64, u32, *mut *mut c_void, *mut CError) -> i32;
type Tessellate =
    unsafe extern "C" fn(*const c_void, f64, f64, *mut *mut c_void, *mut CError) -> i32;
type MeshCounts = unsafe extern "C" fn(*const c_void, *mut u64, *mut u64, *mut CError) -> i32;
type MeshCopy =
    unsafe extern "C" fn(*const c_void, *mut f64, u64, *mut u32, u64, *mut CError) -> i32;
type MeshDestroy = unsafe extern "C" fn(*mut c_void);
type Destroy = unsafe extern "C" fn(*mut c_void);

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn LoadLibraryExW(name: *const u16, file: *mut c_void, flags: u32) -> *mut c_void;
    fn GetProcAddress(module: *mut c_void, name: *const c_char) -> *mut c_void;
    fn FreeLibrary(module: *mut c_void) -> i32;
    fn GetLastError() -> u32;
}

struct KernelLibrary {
    #[cfg(windows)]
    handle: NonNull<c_void>,
}

impl KernelLibrary {
    fn load(path: &Path) -> Result<Self, KernelError> {
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            let load_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
            let wide: Vec<u16> = load_path.as_os_str().encode_wide().chain(Some(0)).collect();
            const LOAD_WITH_ALTERED_SEARCH_PATH: u32 = 0x0000_0008;
            let handle = unsafe {
                LoadLibraryExW(
                    wide.as_ptr(),
                    std::ptr::null_mut(),
                    LOAD_WITH_ALTERED_SEARCH_PATH,
                )
            };
            return NonNull::new(handle)
                .map(|handle| Self { handle })
                .ok_or_else(|| {
                    KernelError::Load(format!("LoadLibraryW failed with error {}", unsafe {
                        GetLastError()
                    }))
                });
        }
        #[cfg(not(windows))]
        {
            let _ = path;
            Err(KernelError::Load(
                "the CAD kernel wrapper currently supports Windows only".into(),
            ))
        }
    }

    unsafe fn symbol<T: Copy>(&self, name: &'static [u8]) -> Result<T, KernelError> {
        #[cfg(windows)]
        {
            let symbol = unsafe { GetProcAddress(self.handle.as_ptr(), name.as_ptr().cast()) };
            if symbol.is_null() {
                return Err(KernelError::Load(format!(
                    "kernel is missing symbol {}",
                    String::from_utf8_lossy(&name[..name.len().saturating_sub(1)])
                )));
            }
            // Function pointer types are paired with the C declarations in cad_kernel.h.
            Ok(unsafe { std::mem::transmute_copy(&symbol) })
        }
        #[cfg(not(windows))]
        {
            let _ = name;
            Err(KernelError::Load(
                "the CAD kernel wrapper currently supports Windows only".into(),
            ))
        }
    }
}

impl Drop for KernelLibrary {
    fn drop(&mut self) {
        #[cfg(windows)]
        unsafe {
            FreeLibrary(self.handle.as_ptr());
        }
    }
}

#[repr(C)]
struct CError {
    code: i32,
    message: [c_char; 256],
}

impl Default for CError {
    fn default() -> Self {
        Self {
            code: 0,
            message: [0; 256],
        }
    }
}

impl CError {
    fn into_error(self, status: i32) -> KernelError {
        // The C ABI guarantees this fixed-size message is NUL terminated.
        let message = unsafe { CStr::from_ptr(self.message.as_ptr()) }
            .to_string_lossy()
            .into_owned();
        KernelError::Operation { status, message }
    }
}

struct Api {
    _library: Arc<KernelLibrary>,
    box_create: BoxCreate,
    cylinder_create: CreateTwo,
    cone_create: CreateTwo,
    truncated_cone_create: BoxCreate,
    ellipsoid_create: BoxCreate,
    pipe_create: BoxCreate,
    sphere_create: CreateOne,
    torus_create: CreateTwo,
    brilliant_gem_create: BoxCreate,
    extruded_polygon_create: ExtrudedPolygon,
    extruded_polygon_vector_create: ExtrudedPolylineVector,
    revolved_polygon_create: PolygonFeature,
    extruded_open_polyline_surface_create: ExtrudedPolygon,
    extruded_open_polyline_surface_vector_create: ExtrudedPolylineVector,
    bezier_curve_create: PolygonFeature,
    extruded_bezier_surface_vector_create: ExtrudedPolylineVector,
    revolved_open_polyline_surface_create: PolygonFeature,
    closed_polyline_create: PolygonFeature,
    open_polyline_curve_create: PolygonFeature,
    planar_polygon_surface_create: PolygonFeature,
    circle_curve_create: CreateOne,
    planar_circle_surface_create: CreateOne,
    extruded_circle_create: CreateTwo,
    extruded_circle_vector_create: CreateFive,
    ellipse_curve_create: CreateTwo,
    planar_ellipse_surface_create: CreateTwo,
    extruded_ellipse_create: BoxCreate,
    extruded_ellipse_vector_create: CreateSix,
    arc_curve_create: CreateThree,
    extruded_arc_surface_create: CreateFour,
    extruded_arc_surface_vector_create: CreateSeven,
    line_curve_create: CreateFour,
    volume: Volume,
    is_valid: IsValid,
    bounds: Bounds,
    mass_properties: MassProperties,
    length: Length,
    transform: Transform,
    mirror: Mirror,
    boolean: Boolean,
    linear_array: LinearArray,
    polar_array_z: PolarArrayZ,
    tessellate: Tessellate,
    mesh_counts: MeshCounts,
    mesh_copy: MeshCopy,
    mesh_destroy: MeshDestroy,
    destroy: Destroy,
}

/// Loaded geometry kernel. Shapes keep the underlying DLL loaded.
#[derive(Clone)]
pub struct Kernel {
    api: Arc<Api>,
}

impl Kernel {
    /// Load a kernel from an explicit DLL path.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, KernelError> {
        let library = Arc::new(KernelLibrary::load(path.as_ref())?);
        let api = unsafe {
            let abi_version: AbiVersion = library.symbol(b"ojcad_abi_version\0")?;
            let actual = abi_version();
            if actual != 24 {
                return Err(KernelError::IncompatibleAbi {
                    expected: 24,
                    actual,
                });
            }
            Api {
                box_create: library.symbol(b"ojcad_box_create\0")?,
                cylinder_create: library.symbol(b"ojcad_cylinder_create\0")?,
                cone_create: library.symbol(b"ojcad_cone_create\0")?,
                truncated_cone_create: library.symbol(b"ojcad_truncated_cone_create\0")?,
                ellipsoid_create: library.symbol(b"ojcad_ellipsoid_create\0")?,
                pipe_create: library.symbol(b"ojcad_pipe_create\0")?,
                sphere_create: library.symbol(b"ojcad_sphere_create\0")?,
                torus_create: library.symbol(b"ojcad_torus_create\0")?,
                brilliant_gem_create: library.symbol(b"ojcad_brilliant_gem_create\0")?,
                extruded_polygon_create: library.symbol(b"ojcad_extruded_polygon_create\0")?,
                extruded_polygon_vector_create: library
                    .symbol(b"ojcad_extruded_polygon_vector_create\0")?,
                revolved_polygon_create: library.symbol(b"ojcad_revolved_polygon_create\0")?,
                extruded_open_polyline_surface_create: library
                    .symbol(b"ojcad_extruded_open_polyline_surface_create\0")?,
                extruded_open_polyline_surface_vector_create: library
                    .symbol(b"ojcad_extruded_open_polyline_surface_vector_create\0")?,
                bezier_curve_create: library.symbol(b"ojcad_bezier_curve_create\0")?,
                extruded_bezier_surface_vector_create: library
                    .symbol(b"ojcad_extruded_bezier_surface_vector_create\0")?,
                revolved_open_polyline_surface_create: library
                    .symbol(b"ojcad_revolved_open_polyline_surface_create\0")?,
                closed_polyline_create: library.symbol(b"ojcad_closed_polyline_create\0")?,
                open_polyline_curve_create: library
                    .symbol(b"ojcad_open_polyline_curve_create\0")?,
                planar_polygon_surface_create: library
                    .symbol(b"ojcad_planar_polygon_surface_create\0")?,
                circle_curve_create: library.symbol(b"ojcad_circle_curve_create\0")?,
                planar_circle_surface_create: library
                    .symbol(b"ojcad_planar_circle_surface_create\0")?,
                extruded_circle_create: library.symbol(b"ojcad_extruded_circle_create\0")?,
                extruded_circle_vector_create: library
                    .symbol(b"ojcad_extruded_circle_vector_create\0")?,
                ellipse_curve_create: library.symbol(b"ojcad_ellipse_curve_create\0")?,
                planar_ellipse_surface_create: library
                    .symbol(b"ojcad_planar_ellipse_surface_create\0")?,
                extruded_ellipse_create: library.symbol(b"ojcad_extruded_ellipse_create\0")?,
                extruded_ellipse_vector_create: library
                    .symbol(b"ojcad_extruded_ellipse_vector_create\0")?,
                arc_curve_create: library.symbol(b"ojcad_arc_curve_create\0")?,
                extruded_arc_surface_create: library
                    .symbol(b"ojcad_extruded_arc_surface_create\0")?,
                extruded_arc_surface_vector_create: library
                    .symbol(b"ojcad_extruded_arc_surface_vector_create\0")?,
                line_curve_create: library.symbol(b"ojcad_line_curve_create\0")?,
                volume: library.symbol(b"ojcad_shape_volume_mm3\0")?,
                is_valid: library.symbol(b"ojcad_shape_is_valid\0")?,
                bounds: library.symbol(b"ojcad_shape_bounds_mm\0")?,
                mass_properties: library.symbol(b"ojcad_shape_mass_properties\0")?,
                length: library.symbol(b"ojcad_shape_length_mm\0")?,
                transform: library.symbol(b"ojcad_shape_transform_mm_degrees\0")?,
                mirror: library.symbol(b"ojcad_shape_mirror\0")?,
                boolean: library.symbol(b"ojcad_shape_boolean\0")?,
                linear_array: library.symbol(b"ojcad_shape_linear_array\0")?,
                polar_array_z: library.symbol(b"ojcad_shape_polar_array_z\0")?,
                tessellate: library.symbol(b"ojcad_shape_tessellate\0")?,
                mesh_counts: library.symbol(b"ojcad_mesh_counts\0")?,
                mesh_copy: library.symbol(b"ojcad_mesh_copy\0")?,
                mesh_destroy: library.symbol(b"ojcad_mesh_destroy\0")?,
                destroy: library.symbol(b"ojcad_shape_destroy\0")?,
                _library: library,
            }
        };
        Ok(Self { api: Arc::new(api) })
    }

    /// Load the configured kernel DLL, with development and packaged-location fallbacks.
    pub fn from_environment() -> Result<Self, KernelError> {
        if let Some(path) = env::var_os("OPENJEWELCAD_KERNEL_DLL") {
            return Self::load(path);
        }
        let executable_directory = env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(Path::to_path_buf));
        let Some(executable_directory) = executable_directory else {
            return Err(KernelError::MissingKernelPath);
        };
        let candidates = [
            executable_directory.join("openjewelcad_kernel.dll"),
            executable_directory.join("..\\..\\build\\Debug\\openjewelcad_kernel.dll"),
            executable_directory.join("..\\..\\build\\Release\\openjewelcad_kernel.dll"),
        ];
        for candidate in candidates {
            if candidate.is_file() {
                return Self::load(candidate);
            }
        }
        Err(KernelError::MissingKernelPath)
    }

    /// Create an OCCT box with positive dimensions in millimetres.
    pub fn box_mm(&self, x: f64, y: f64, z: f64) -> Result<Shape, KernelError> {
        let mut raw = std::ptr::null_mut();
        let mut error = CError::default();
        let status = unsafe { (self.api.box_create)(x, y, z, &mut raw, &mut error) };
        self.finish_shape_creation(status, raw, error)
    }

    /// Create a cylinder along the Z axis with positive radius and height in millimetres.
    pub fn cylinder_mm(&self, radius: f64, height: f64) -> Result<Shape, KernelError> {
        let mut raw = std::ptr::null_mut();
        let mut error = CError::default();
        let status = unsafe { (self.api.cylinder_create)(radius, height, &mut raw, &mut error) };
        self.finish_shape_creation(status, raw, error)
    }

    /// Create a cone along the Z axis with a positive base radius and height in millimetres.
    pub fn cone_mm(&self, radius: f64, height: f64) -> Result<Shape, KernelError> {
        let mut raw = std::ptr::null_mut();
        let mut error = CError::default();
        let status = unsafe { (self.api.cone_create)(radius, height, &mut raw, &mut error) };
        self.finish_shape_creation(status, raw, error)
    }

    /// Create a conical frustum with distinct positive base/top radii and height in millimetres.
    pub fn truncated_cone_mm(
        &self,
        base_radius: f64,
        top_radius: f64,
        height: f64,
    ) -> Result<Shape, KernelError> {
        let mut raw = std::ptr::null_mut();
        let mut error = CError::default();
        let status = unsafe {
            (self.api.truncated_cone_create)(base_radius, top_radius, height, &mut raw, &mut error)
        };
        self.finish_shape_creation(status, raw, error)
    }

    /// Create an ellipsoid centered at the origin with positive X/Y/Z radii in millimetres.
    pub fn ellipsoid_mm(
        &self,
        radius_x: f64,
        radius_y: f64,
        radius_z: f64,
    ) -> Result<Shape, KernelError> {
        let mut raw = std::ptr::null_mut();
        let mut error = CError::default();
        let status = unsafe {
            (self.api.ellipsoid_create)(radius_x, radius_y, radius_z, &mut raw, &mut error)
        };
        self.finish_shape_creation(status, raw, error)
    }

    /// Create a straight hollow pipe along Z with a smaller positive inner radius.
    pub fn pipe_mm(
        &self,
        outer_radius: f64,
        inner_radius: f64,
        length: f64,
    ) -> Result<Shape, KernelError> {
        let mut raw = std::ptr::null_mut();
        let mut error = CError::default();
        let status = unsafe {
            (self.api.pipe_create)(outer_radius, inner_radius, length, &mut raw, &mut error)
        };
        self.finish_shape_creation(status, raw, error)
    }

    /// Create a sphere centered at the origin with a positive radius in millimetres.
    pub fn sphere_mm(&self, radius: f64) -> Result<Shape, KernelError> {
        let mut raw = std::ptr::null_mut();
        let mut error = CError::default();
        let status = unsafe { (self.api.sphere_create)(radius, &mut raw, &mut error) };
        self.finish_shape_creation(status, raw, error)
    }

    /// Create a ring-shaped torus, with major radius greater than tube radius.
    pub fn torus_mm(&self, major_radius: f64, tube_radius: f64) -> Result<Shape, KernelError> {
        let mut raw = std::ptr::null_mut();
        let mut error = CError::default();
        let status =
            unsafe { (self.api.torus_create)(major_radius, tube_radius, &mut raw, &mut error) };
        self.finish_shape_creation(status, raw, error)
    }

    /// Create a faceted round brilliant gemstone with a finite culet, in millimetres.
    pub fn brilliant_gem_mm(
        &self,
        diameter: f64,
        total_height: f64,
        table_diameter: f64,
    ) -> Result<Shape, KernelError> {
        let mut raw = std::ptr::null_mut();
        let mut error = CError::default();
        let status = unsafe {
            (self.api.brilliant_gem_create)(
                diameter,
                total_height,
                table_diameter,
                &mut raw,
                &mut error,
            )
        };
        self.finish_shape_creation(status, raw, error)
    }

    /// Create a circular OCCT wire in the XY plane, with radius in millimetres.
    pub fn circle_curve_mm(&self, radius_mm: f64) -> Result<Shape, KernelError> {
        self.create_one_parameter_shape(self.api.circle_curve_create, radius_mm)
    }

    /// Create a planar circular OCCT face in the XY plane, with radius in millimetres.
    pub fn planar_circle_surface_mm(&self, radius_mm: f64) -> Result<Shape, KernelError> {
        self.create_one_parameter_shape(self.api.planar_circle_surface_create, radius_mm)
    }

    /// Extrude a circular profile along +Z, with radius and height in millimetres.
    pub fn extruded_circle_mm(&self, radius_mm: f64, height_mm: f64) -> Result<Shape, KernelError> {
        let mut raw = std::ptr::null_mut();
        let mut error = CError::default();
        let status = unsafe {
            (self.api.extruded_circle_create)(radius_mm, height_mm, &mut raw, &mut error)
        };
        self.finish_shape_creation(status, raw, error)
    }

    /// Extrude a circular profile by travel distance along the supplied 3D direction.
    pub fn extruded_circle_vector_mm(
        &self,
        radius_mm: f64,
        height_mm: f64,
        direction: [f64; 3],
    ) -> Result<Shape, KernelError> {
        let mut raw = std::ptr::null_mut();
        let mut error = CError::default();
        let status = unsafe {
            (self.api.extruded_circle_vector_create)(
                radius_mm,
                height_mm,
                direction[0],
                direction[1],
                direction[2],
                &mut raw,
                &mut error,
            )
        };
        self.finish_shape_creation(status, raw, error)
    }

    /// Create an analytic ellipse wire in the XY plane, with X and Y radii in millimetres.
    pub fn ellipse_curve_mm(
        &self,
        radius_x_mm: f64,
        radius_y_mm: f64,
    ) -> Result<Shape, KernelError> {
        self.create_two_parameter_shape(self.api.ellipse_curve_create, radius_x_mm, radius_y_mm)
    }

    /// Create an analytic planar ellipse face in XY, with X and Y radii in millimetres.
    pub fn planar_ellipse_surface_mm(
        &self,
        radius_x_mm: f64,
        radius_y_mm: f64,
    ) -> Result<Shape, KernelError> {
        self.create_two_parameter_shape(
            self.api.planar_ellipse_surface_create,
            radius_x_mm,
            radius_y_mm,
        )
    }

    /// Extrude an ellipse along +Z, with X/Y radii and height in millimetres.
    pub fn extruded_ellipse_mm(
        &self,
        radius_x_mm: f64,
        radius_y_mm: f64,
        height_mm: f64,
    ) -> Result<Shape, KernelError> {
        let mut raw = std::ptr::null_mut();
        let mut error = CError::default();
        let status = unsafe {
            (self.api.extruded_ellipse_create)(
                radius_x_mm,
                radius_y_mm,
                height_mm,
                &mut raw,
                &mut error,
            )
        };
        self.finish_shape_creation(status, raw, error)
    }

    /// Extrude an elliptical profile by travel distance along the supplied 3D direction.
    pub fn extruded_ellipse_vector_mm(
        &self,
        radius_x_mm: f64,
        radius_y_mm: f64,
        height_mm: f64,
        direction: [f64; 3],
    ) -> Result<Shape, KernelError> {
        let mut raw = std::ptr::null_mut();
        let mut error = CError::default();
        let status = unsafe {
            (self.api.extruded_ellipse_vector_create)(
                radius_x_mm,
                radius_y_mm,
                height_mm,
                direction[0],
                direction[1],
                direction[2],
                &mut raw,
                &mut error,
            )
        };
        self.finish_shape_creation(status, raw, error)
    }

    /// Create an open circular arc in XY, with CCW start angle and positive sweep in degrees.
    pub fn arc_curve_mm(
        &self,
        radius_mm: f64,
        start_angle_deg: f64,
        sweep_angle_deg: f64,
    ) -> Result<Shape, KernelError> {
        let mut raw = std::ptr::null_mut();
        let mut error = CError::default();
        let status = unsafe {
            (self.api.arc_curve_create)(
                radius_mm,
                start_angle_deg,
                sweep_angle_deg,
                &mut raw,
                &mut error,
            )
        };
        self.finish_shape_creation(status, raw, error)
    }

    /// Extrude an open circular arc along +Z into an exact OCCT surface shell.
    pub fn extruded_arc_surface_mm(
        &self,
        radius_mm: f64,
        start_angle_deg: f64,
        sweep_angle_deg: f64,
        height_mm: f64,
    ) -> Result<Shape, KernelError> {
        let mut raw = std::ptr::null_mut();
        let mut error = CError::default();
        let status = unsafe {
            (self.api.extruded_arc_surface_create)(
                radius_mm,
                start_angle_deg,
                sweep_angle_deg,
                height_mm,
                &mut raw,
                &mut error,
            )
        };
        self.finish_shape_creation(status, raw, error)
    }

    /// Extrude an open circular arc by a distance along a normalized 3D direction.
    pub fn extruded_arc_surface_vector_mm(
        &self,
        radius_mm: f64,
        start_angle_deg: f64,
        sweep_angle_deg: f64,
        height_mm: f64,
        direction: [f64; 3],
    ) -> Result<Shape, KernelError> {
        let mut raw = std::ptr::null_mut();
        let mut error = CError::default();
        let status = unsafe {
            (self.api.extruded_arc_surface_vector_create)(
                radius_mm,
                start_angle_deg,
                sweep_angle_deg,
                height_mm,
                direction[0],
                direction[1],
                direction[2],
                &mut raw,
                &mut error,
            )
        };
        self.finish_shape_creation(status, raw, error)
    }

    /// Create an open straight wire between two XY points in millimetres.
    pub fn line_curve_mm(
        &self,
        start_mm: [f64; 2],
        end_mm: [f64; 2],
    ) -> Result<Shape, KernelError> {
        let mut raw = std::ptr::null_mut();
        let mut error = CError::default();
        let status = unsafe {
            (self.api.line_curve_create)(
                start_mm[0],
                start_mm[1],
                end_mm[0],
                end_mm[1],
                &mut raw,
                &mut error,
            )
        };
        self.finish_shape_creation(status, raw, error)
    }

    fn create_one_parameter_shape(
        &self,
        create: CreateOne,
        parameter: f64,
    ) -> Result<Shape, KernelError> {
        let mut raw = std::ptr::null_mut();
        let mut error = CError::default();
        let status = unsafe { create(parameter, &mut raw, &mut error) };
        self.finish_shape_creation(status, raw, error)
    }

    fn create_two_parameter_shape(
        &self,
        create: CreateTwo,
        first: f64,
        second: f64,
    ) -> Result<Shape, KernelError> {
        let mut raw = std::ptr::null_mut();
        let mut error = CError::default();
        let status = unsafe { create(first, second, &mut raw, &mut error) };
        self.finish_shape_creation(status, raw, error)
    }

    /// Create an OCCT closed polyline curve from an XY profile in millimetres.
    pub fn closed_polyline_mm(&self, points_xy_mm: &[[f64; 2]]) -> Result<Shape, KernelError> {
        self.create_polygon_feature(points_xy_mm, true)
    }

    /// Create an open OCCT polyline wire from consecutive XY points in millimetres.
    pub fn open_polyline_curve_mm(&self, points_xy_mm: &[[f64; 2]]) -> Result<Shape, KernelError> {
        let point_count =
            u32::try_from(points_xy_mm.len()).map_err(|_| KernelError::Operation {
                status: 1,
                message: "open polyline contains too many points".into(),
            })?;
        let points = if points_xy_mm.is_empty() {
            std::ptr::null()
        } else {
            points_xy_mm.as_ptr().cast::<f64>()
        };
        let mut raw = std::ptr::null_mut();
        let mut error = CError::default();
        let status = unsafe {
            (self.api.open_polyline_curve_create)(points, point_count, &mut raw, &mut error)
        };
        self.finish_shape_creation(status, raw, error)
    }

    /// Create an open cubic Bezier curve in XY from four control points in millimetres.
    pub fn bezier_curve_mm(&self, control_points_xy_mm: &[[f64; 2]]) -> Result<Shape, KernelError> {
        let point_count =
            u32::try_from(control_points_xy_mm.len()).map_err(|_| KernelError::Operation {
                status: 1,
                message: "Bezier curve contains too many control points".into(),
            })?;
        let points = if control_points_xy_mm.is_empty() {
            std::ptr::null()
        } else {
            control_points_xy_mm.as_ptr().cast::<f64>()
        };
        let mut raw = std::ptr::null_mut();
        let mut error = CError::default();
        let status =
            unsafe { (self.api.bezier_curve_create)(points, point_count, &mut raw, &mut error) };
        self.finish_shape_creation(status, raw, error)
    }

    /// Extrude an open cubic Bezier curve into a surface along a supplied 3D direction.
    pub fn extruded_bezier_surface_vector_mm(
        &self,
        control_points_xy_mm: &[[f64; 2]],
        height_mm: f64,
        direction: [f64; 3],
    ) -> Result<Shape, KernelError> {
        let point_count =
            u32::try_from(control_points_xy_mm.len()).map_err(|_| KernelError::Operation {
                status: 1,
                message: "Bezier surface contains too many control points".into(),
            })?;
        let points = if control_points_xy_mm.is_empty() {
            std::ptr::null()
        } else {
            control_points_xy_mm.as_ptr().cast::<f64>()
        };
        let mut raw = std::ptr::null_mut();
        let mut error = CError::default();
        let status = unsafe {
            (self.api.extruded_bezier_surface_vector_create)(
                points,
                point_count,
                height_mm,
                direction[0],
                direction[1],
                direction[2],
                &mut raw,
                &mut error,
            )
        };
        self.finish_shape_creation(status, raw, error)
    }

    /// Create an OCCT planar face from a simple closed XY profile in millimetres.
    pub fn planar_polygon_surface_mm(
        &self,
        points_xy_mm: &[[f64; 2]],
    ) -> Result<Shape, KernelError> {
        self.create_polygon_feature(points_xy_mm, false)
    }

    fn create_polygon_feature(
        &self,
        points_xy_mm: &[[f64; 2]],
        closed_polyline: bool,
    ) -> Result<Shape, KernelError> {
        let point_count =
            u32::try_from(points_xy_mm.len()).map_err(|_| KernelError::Operation {
                status: 1,
                message: "polygon feature contains too many points".into(),
            })?;
        let points = if points_xy_mm.is_empty() {
            std::ptr::null()
        } else {
            points_xy_mm.as_ptr().cast::<f64>()
        };
        let mut raw = std::ptr::null_mut();
        let mut error = CError::default();
        let create = if closed_polyline {
            self.api.closed_polyline_create
        } else {
            self.api.planar_polygon_surface_create
        };
        let status = unsafe { create(points, point_count, &mut raw, &mut error) };
        self.finish_shape_creation(status, raw, error)
    }

    /// Extrude a simple closed XY polygon along +Z. Points and height are in millimetres.
    pub fn extruded_polygon_mm(
        &self,
        points_xy_mm: &[[f64; 2]],
        height_mm: f64,
    ) -> Result<Shape, KernelError> {
        let point_count =
            u32::try_from(points_xy_mm.len()).map_err(|_| KernelError::Operation {
                status: 1,
                message: "polygon profile contains too many points".into(),
            })?;
        let points = if points_xy_mm.is_empty() {
            std::ptr::null()
        } else {
            points_xy_mm.as_ptr().cast::<f64>()
        };
        let mut raw = std::ptr::null_mut();
        let mut error = CError::default();
        let status = unsafe {
            (self.api.extruded_polygon_create)(points, point_count, height_mm, &mut raw, &mut error)
        };
        self.finish_shape_creation(status, raw, error)
    }

    /// Extrude a closed polygon by travel distance along the supplied 3D direction.
    pub fn extruded_polygon_vector_mm(
        &self,
        points_xy_mm: &[[f64; 2]],
        height_mm: f64,
        direction: [f64; 3],
    ) -> Result<Shape, KernelError> {
        let point_count =
            u32::try_from(points_xy_mm.len()).map_err(|_| KernelError::Operation {
                status: 1,
                message: "polygon profile contains too many points".into(),
            })?;
        let points = if points_xy_mm.is_empty() {
            std::ptr::null()
        } else {
            points_xy_mm.as_ptr().cast::<f64>()
        };
        let mut raw = std::ptr::null_mut();
        let mut error = CError::default();
        let status = unsafe {
            (self.api.extruded_polygon_vector_create)(
                points,
                point_count,
                height_mm,
                direction[0],
                direction[1],
                direction[2],
                &mut raw,
                &mut error,
            )
        };
        self.finish_shape_creation(status, raw, error)
    }

    /// Revolve a simple XY profile around the Y axis to form a 360-degree solid.
    /// X is the radius and must stay positive; Y is height along the axis.
    pub fn revolved_polygon_mm(&self, points_xy_mm: &[[f64; 2]]) -> Result<Shape, KernelError> {
        let point_count =
            u32::try_from(points_xy_mm.len()).map_err(|_| KernelError::Operation {
                status: 1,
                message: "revolve profile contains too many points".into(),
            })?;
        let points = if points_xy_mm.is_empty() {
            std::ptr::null()
        } else {
            points_xy_mm.as_ptr().cast::<f64>()
        };
        let mut raw = std::ptr::null_mut();
        let mut error = CError::default();
        let status = unsafe {
            (self.api.revolved_polygon_create)(points, point_count, &mut raw, &mut error)
        };
        self.finish_shape_creation(status, raw, error)
    }

    /// Extrude an open XY line or polyline along +Z to create an exact OCCT surface shell.
    pub fn extruded_open_polyline_surface_mm(
        &self,
        points_xy_mm: &[[f64; 2]],
        height_mm: f64,
    ) -> Result<Shape, KernelError> {
        let point_count =
            u32::try_from(points_xy_mm.len()).map_err(|_| KernelError::Operation {
                status: 1,
                message: "open-curve surface contains too many points".into(),
            })?;
        let points = if points_xy_mm.is_empty() {
            std::ptr::null()
        } else {
            points_xy_mm.as_ptr().cast::<f64>()
        };
        let mut raw = std::ptr::null_mut();
        let mut error = CError::default();
        let status = unsafe {
            (self.api.extruded_open_polyline_surface_create)(
                points,
                point_count,
                height_mm,
                &mut raw,
                &mut error,
            )
        };
        self.finish_shape_creation(status, raw, error)
    }

    /// Extrude an open XY line or polyline by a distance in a normalized 3D direction.
    pub fn extruded_open_polyline_surface_vector_mm(
        &self,
        points_xy_mm: &[[f64; 2]],
        height_mm: f64,
        direction: [f64; 3],
    ) -> Result<Shape, KernelError> {
        let point_count =
            u32::try_from(points_xy_mm.len()).map_err(|_| KernelError::Operation {
                status: 1,
                message: "open-curve surface contains too many points".into(),
            })?;
        let points = if points_xy_mm.is_empty() {
            std::ptr::null()
        } else {
            points_xy_mm.as_ptr().cast::<f64>()
        };
        let mut raw = std::ptr::null_mut();
        let mut error = CError::default();
        let status = unsafe {
            (self.api.extruded_open_polyline_surface_vector_create)(
                points,
                point_count,
                height_mm,
                direction[0],
                direction[1],
                direction[2],
                &mut raw,
                &mut error,
            )
        };
        self.finish_shape_creation(status, raw, error)
    }

    /// Revolve an open XY radius/height profile around +Y into a full OCCT surface shell.
    pub fn revolved_open_polyline_surface_mm(
        &self,
        points_xy_mm: &[[f64; 2]],
    ) -> Result<Shape, KernelError> {
        let point_count =
            u32::try_from(points_xy_mm.len()).map_err(|_| KernelError::Operation {
                status: 1,
                message: "revolved surface contains too many profile points".into(),
            })?;
        let points = if points_xy_mm.is_empty() {
            std::ptr::null()
        } else {
            points_xy_mm.as_ptr().cast::<f64>()
        };
        let mut raw = std::ptr::null_mut();
        let mut error = CError::default();
        let status = unsafe {
            (self.api.revolved_open_polyline_surface_create)(
                points,
                point_count,
                &mut raw,
                &mut error,
            )
        };
        self.finish_shape_creation(status, raw, error)
    }

    fn finish_shape_creation(
        &self,
        status: i32,
        raw: *mut c_void,
        error: CError,
    ) -> Result<Shape, KernelError> {
        if status != 0 {
            return Err(error.into_error(status));
        }
        let handle = NonNull::new(raw).ok_or_else(|| KernelError::Operation {
            status: 3,
            message: "kernel returned success without a shape handle".into(),
        })?;
        Ok(Shape {
            handle,
            api: Arc::clone(&self.api),
        })
    }
}

/// Owned OCCT shape. Drop calls the kernel's matching release function once.
pub struct Shape {
    handle: NonNull<c_void>,
    api: Arc<Api>,
}

impl Shape {
    /// Combine two solids with an OCCT B-rep boolean at the supplied millimetre tolerance.
    pub fn boolean_with(
        &self,
        tool: &Shape,
        operation: BooleanOperation,
        tolerance_mm: f64,
    ) -> Result<Self, KernelError> {
        let mut raw = std::ptr::null_mut();
        let mut error = CError::default();
        let status = unsafe {
            (self.api.boolean)(
                self.handle.as_ptr(),
                tool.handle.as_ptr(),
                operation as i32,
                tolerance_mm,
                &mut raw,
                &mut error,
            )
        };
        if status != 0 {
            return Err(error.into_error(status));
        }
        let handle = NonNull::new(raw).ok_or_else(|| KernelError::Operation {
            status: 3,
            message: "kernel returned success without a boolean result".into(),
        })?;
        Ok(Self {
            handle,
            api: Arc::clone(&self.api),
        })
    }

    /// Create a compound with this shape and linearly translated copies.
    /// The copy count includes the original; at least two copies and a non-zero step are required.
    pub fn linear_array(&self, copies: u32, step_mm: [f64; 3]) -> Result<Self, KernelError> {
        let mut raw = std::ptr::null_mut();
        let mut error = CError::default();
        let status = unsafe {
            (self.api.linear_array)(
                self.handle.as_ptr(),
                copies,
                step_mm[0],
                step_mm[1],
                step_mm[2],
                &mut raw,
                &mut error,
            )
        };
        self.finish_pattern_creation(status, raw, error, "linear array")
    }

    /// Create a compound with this shape rotated around the world Z axis for each copy.
    pub fn polar_array_z(&self, copies: u32, angle_step_deg: f64) -> Result<Self, KernelError> {
        let mut raw = std::ptr::null_mut();
        let mut error = CError::default();
        let status = unsafe {
            (self.api.polar_array_z)(
                self.handle.as_ptr(),
                copies,
                angle_step_deg,
                &mut raw,
                &mut error,
            )
        };
        self.finish_pattern_creation(status, raw, error, "polar array")
    }

    fn finish_pattern_creation(
        &self,
        status: i32,
        raw: *mut c_void,
        error: CError,
        operation: &str,
    ) -> Result<Self, KernelError> {
        if status != 0 {
            return Err(error.into_error(status));
        }
        let handle = NonNull::new(raw).ok_or_else(|| KernelError::Operation {
            status: 3,
            message: format!("kernel returned success without a {operation} result"),
        })?;
        Ok(Self {
            handle,
            api: Arc::clone(&self.api),
        })
    }

    /// Tessellate this exact OCCT shape. Linear deflection is in mm and angular in radians.
    pub fn tessellate(
        &self,
        linear_deflection_mm: f64,
        angular_deflection_radians: f64,
    ) -> Result<TriangleMesh, KernelError> {
        let mut raw = std::ptr::null_mut();
        let mut error = CError::default();
        let status = unsafe {
            (self.api.tessellate)(
                self.handle.as_ptr(),
                linear_deflection_mm,
                angular_deflection_radians,
                &mut raw,
                &mut error,
            )
        };
        if status != 0 {
            return Err(error.into_error(status));
        }
        let handle = NonNull::new(raw).ok_or_else(|| KernelError::Operation {
            status: 3,
            message: "kernel returned success without a mesh handle".into(),
        })?;
        let mesh = MeshGuard {
            handle,
            api: Arc::clone(&self.api),
        };
        let mut vertex_count = 0_u64;
        let mut triangle_count = 0_u64;
        let status = unsafe {
            (self.api.mesh_counts)(
                mesh.handle.as_ptr(),
                &mut vertex_count,
                &mut triangle_count,
                &mut error,
            )
        };
        if status != 0 {
            return Err(error.into_error(status));
        }
        let vertex_values = usize::try_from(vertex_count)
            .ok()
            .and_then(|count| count.checked_mul(3))
            .ok_or_else(|| KernelError::Operation {
                status: 3,
                message: "mesh vertex count exceeds addressable memory".into(),
            })?;
        let triangle_values = usize::try_from(triangle_count)
            .ok()
            .and_then(|count| count.checked_mul(3))
            .ok_or_else(|| KernelError::Operation {
                status: 3,
                message: "mesh triangle count exceeds addressable memory".into(),
            })?;
        let mut vertices = vec![0.0; vertex_values];
        let mut indices = vec![0_u32; triangle_values];
        let status = unsafe {
            (self.api.mesh_copy)(
                mesh.handle.as_ptr(),
                vertices.as_mut_ptr(),
                vertex_count,
                indices.as_mut_ptr(),
                triangle_count,
                &mut error,
            )
        };
        if status != 0 {
            return Err(error.into_error(status));
        }
        let vertices_mm = vertices
            .chunks_exact(3)
            .map(|point| [point[0], point[1], point[2]])
            .collect();
        let triangles = indices
            .chunks_exact(3)
            .map(|triangle| [triangle[0], triangle[1], triangle[2]])
            .collect();
        Ok(TriangleMesh {
            vertices_mm,
            triangles,
        })
    }

    pub fn volume_mm3(&self) -> Result<f64, KernelError> {
        let mut volume = 0.0;
        let mut error = CError::default();
        let status = unsafe { (self.api.volume)(self.handle.as_ptr(), &mut volume, &mut error) };
        if status == 0 {
            Ok(volume)
        } else {
            Err(error.into_error(status))
        }
    }

    pub fn is_valid(&self) -> Result<bool, KernelError> {
        let mut valid = 0;
        let mut error = CError::default();
        let status = unsafe { (self.api.is_valid)(self.handle.as_ptr(), &mut valid, &mut error) };
        if status != 0 {
            return Err(error.into_error(status));
        }
        match valid {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(KernelError::Operation {
                status: 3,
                message: "kernel returned an invalid boolean value".into(),
            }),
        }
    }

    /// Return [xmin, ymin, zmin, xmax, ymax, zmax] bounds in millimetres.
    pub fn bounds_mm(&self) -> Result<[f64; 6], KernelError> {
        let mut bounds = [0.0; 6];
        let mut error = CError::default();
        let status =
            unsafe { (self.api.bounds)(self.handle.as_ptr(), bounds.as_mut_ptr(), &mut error) };
        if status == 0 {
            Ok(bounds)
        } else {
            Err(error.into_error(status))
        }
    }

    /// Return [volume mm^3, area mm^2, center X mm, center Y mm, center Z mm].
    pub fn mass_properties(&self) -> Result<[f64; 5], KernelError> {
        let mut properties = [0.0; 5];
        let mut error = CError::default();
        let status = unsafe {
            (self.api.mass_properties)(self.handle.as_ptr(), properties.as_mut_ptr(), &mut error)
        };
        if status == 0 {
            Ok(properties)
        } else {
            Err(error.into_error(status))
        }
    }

    /// Return total edge length in millimetres.
    pub fn length_mm(&self) -> Result<f64, KernelError> {
        let mut length = 0.0;
        let mut error = CError::default();
        let status = unsafe { (self.api.length)(self.handle.as_ptr(), &mut length, &mut error) };
        if status == 0 {
            Ok(length)
        } else {
            Err(error.into_error(status))
        }
    }

    /// Create a transformed copy using scale, X/Y/Z rotations in degrees, then translation.
    pub fn transformed_mm_degrees(
        &self,
        translation_mm: [f64; 3],
        rotation_deg: [f64; 3],
        uniform_scale: f64,
    ) -> Result<Self, KernelError> {
        let mut raw = std::ptr::null_mut();
        let mut error = CError::default();
        let status = unsafe {
            (self.api.transform)(
                self.handle.as_ptr(),
                translation_mm[0],
                translation_mm[1],
                translation_mm[2],
                rotation_deg[0],
                rotation_deg[1],
                rotation_deg[2],
                uniform_scale,
                &mut raw,
                &mut error,
            )
        };
        if status != 0 {
            return Err(error.into_error(status));
        }
        let handle = NonNull::new(raw).ok_or_else(|| KernelError::Operation {
            status: 3,
            message: "kernel returned success without a shape handle".into(),
        })?;
        Ok(Self {
            handle,
            api: Arc::clone(&self.api),
        })
    }

    /// Mirror a copy across world-origin planes. Bits 0, 1, and 2 select YZ, XZ, and XY.
    pub fn mirrored(&self, plane_mask: u32) -> Result<Self, KernelError> {
        let mut raw = std::ptr::null_mut();
        let mut error = CError::default();
        let status =
            unsafe { (self.api.mirror)(self.handle.as_ptr(), plane_mask, &mut raw, &mut error) };
        if status != 0 {
            return Err(error.into_error(status));
        }
        let handle = NonNull::new(raw).ok_or_else(|| KernelError::Operation {
            status: 3,
            message: "kernel returned success without a shape handle".into(),
        })?;
        Ok(Self {
            handle,
            api: Arc::clone(&self.api),
        })
    }
}

/// Supported exact solid B-rep boolean operations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum BooleanOperation {
    Union = 1,
    Difference = 2,
    Intersection = 3,
}

struct MeshGuard {
    handle: NonNull<c_void>,
    api: Arc<Api>,
}

impl Drop for MeshGuard {
    fn drop(&mut self) {
        unsafe { (self.api.mesh_destroy)(self.handle.as_ptr()) };
    }
}

/// Triangle mesh extracted from OCCT. Coordinates remain in millimetres.
#[derive(Clone, Debug, PartialEq)]
pub struct TriangleMesh {
    pub vertices_mm: Vec<[f64; 3]>,
    pub triangles: Vec<[u32; 3]>,
}

impl Drop for Shape {
    fn drop(&mut self) {
        unsafe { (self.api.destroy)(self.handle.as_ptr()) };
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum KernelError {
    MissingKernelPath,
    Load(String),
    IncompatibleAbi { expected: u32, actual: u32 },
    Operation { status: i32, message: String },
}

impl fmt::Display for KernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingKernelPath => write!(f, "OPENJEWELCAD_KERNEL_DLL is not set"),
            Self::Load(message) => write!(f, "could not load the CAD kernel: {message}"),
            Self::IncompatibleAbi { expected, actual } => write!(
                f,
                "CAD kernel ABI mismatch: expected {expected}, found {actual}"
            ),
            Self::Operation { status, message } => {
                write!(f, "CAD kernel error {status}: {message}")
            }
        }
    }
}

impl Error for KernelError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn kernel() -> Kernel {
        Kernel::from_environment()
            .expect("set OPENJEWELCAD_KERNEL_DLL to run C ABI integration tests")
    }

    #[test]
    fn rust_calls_occt_and_releases_shape() {
        let kernel = kernel();
        let shape = kernel.box_mm(2.0, 3.0, 4.0).unwrap();
        assert!(shape.is_valid().unwrap());
        assert!((shape.volume_mm3().unwrap() - 24.0).abs() < 1e-9);
        drop(shape);
    }

    #[test]
    fn invalid_box_is_a_structured_error() {
        assert!(matches!(
            kernel().box_mm(0.0, 3.0, 4.0),
            Err(KernelError::Operation { status: 1, .. })
        ));
    }

    #[test]
    fn primitives_have_expected_volumes_and_validity() {
        let kernel = kernel();

        let cylinder = kernel.cylinder_mm(2.0, 5.0).unwrap();
        assert!(cylinder.is_valid().unwrap());
        assert!((cylinder.volume_mm3().unwrap() - std::f64::consts::PI * 20.0).abs() < 1e-8);

        let cone = kernel.cone_mm(2.0, 6.0).unwrap();
        assert!(cone.is_valid().unwrap());
        assert!((cone.volume_mm3().unwrap() - std::f64::consts::PI * 8.0).abs() < 1e-8);

        let truncated_cone = kernel.truncated_cone_mm(3.0, 1.0, 4.0).unwrap();
        assert!(truncated_cone.is_valid().unwrap());
        assert!(
            (truncated_cone.volume_mm3().unwrap() - 52.0 / 3.0 * std::f64::consts::PI).abs() < 1e-8
        );

        let ellipsoid = kernel.ellipsoid_mm(4.0, 3.0, 2.0).unwrap();
        assert!(ellipsoid.is_valid().unwrap());
        assert!((ellipsoid.volume_mm3().unwrap() - 32.0 * std::f64::consts::PI).abs() < 1e-4);

        let pipe = kernel.pipe_mm(3.0, 1.5, 4.0).unwrap();
        assert!(pipe.is_valid().unwrap());
        assert!((pipe.volume_mm3().unwrap() - 27.0 * std::f64::consts::PI).abs() < 1e-7);

        let sphere = kernel.sphere_mm(3.0).unwrap();
        assert!(sphere.is_valid().unwrap());
        let sphere_volume = 4.0 / 3.0 * std::f64::consts::PI * 27.0;
        assert!((sphere.volume_mm3().unwrap() - sphere_volume).abs() < 1e-8);

        let torus = kernel.torus_mm(5.0, 1.0).unwrap();
        assert!(torus.is_valid().unwrap());
        let torus_volume = 2.0 * std::f64::consts::PI.powi(2) * 5.0;
        assert!((torus.volume_mm3().unwrap() - torus_volume).abs() < 1e-7);

        let brilliant = kernel.brilliant_gem_mm(6.0, 3.6, 3.3).unwrap();
        assert!(brilliant.is_valid().unwrap());
        assert!(brilliant.volume_mm3().unwrap() > 0.0);
        for (actual, expected) in brilliant
            .bounds_mm()
            .unwrap()
            .into_iter()
            .zip([-3.0, -3.0, 0.0, 3.0, 3.0, 3.6])
        {
            assert!((actual - expected).abs() < 1e-6);
        }
    }

    #[test]
    fn brilliant_gem_rejects_invalid_table_diameter() {
        assert!(matches!(
            kernel().brilliant_gem_mm(6.0, 3.6, 6.0),
            Err(KernelError::Operation { status: 1, .. })
        ));
    }

    #[test]
    fn polygon_extrusion_builds_prismatic_solids_from_concave_profiles() {
        let kernel = kernel();
        let rectangle = [[0.0, 0.0], [4.0, 0.0], [4.0, 3.0], [0.0, 3.0]];
        let prism = kernel.extruded_polygon_mm(&rectangle, 2.0).unwrap();
        assert!(prism.is_valid().unwrap());
        assert!((prism.volume_mm3().unwrap() - 24.0).abs() < 1e-8);
        for (actual, expected) in prism
            .bounds_mm()
            .unwrap()
            .into_iter()
            .zip([0.0, 0.0, 0.0, 4.0, 3.0, 2.0])
        {
            assert!((actual - expected).abs() < 1e-7);
        }

        let concave_l = [
            [0.0, 0.0],
            [3.0, 0.0],
            [3.0, 1.0],
            [1.0, 1.0],
            [1.0, 3.0],
            [0.0, 3.0],
        ];
        let concave_prism = kernel.extruded_polygon_mm(&concave_l, 2.0).unwrap();
        assert!(concave_prism.is_valid().unwrap());
        assert!((concave_prism.volume_mm3().unwrap() - 10.0).abs() < 1e-8);
    }

    #[test]
    fn polygon_extrusion_rejects_self_intersecting_profiles() {
        let bow_tie = [[0.0, 0.0], [2.0, 2.0], [0.0, 2.0], [2.0, 0.0]];
        assert!(matches!(
            kernel().extruded_polygon_mm(&bow_tie, 1.0),
            Err(KernelError::Operation { status: 1, .. })
        ));
    }

    #[test]
    fn profile_revolve_creates_a_valid_360_degree_solid() {
        let kernel = kernel();
        let rectangle = [[3.0, -2.0], [5.0, -2.0], [5.0, 2.0], [3.0, 2.0]];
        let solid = kernel.revolved_polygon_mm(&rectangle).unwrap();
        assert!(solid.is_valid().unwrap());
        assert!((solid.volume_mm3().unwrap() - 64.0 * std::f64::consts::PI).abs() < 1e-7);
        for (actual, expected) in solid
            .bounds_mm()
            .unwrap()
            .into_iter()
            .zip([-5.0, -2.0, -5.0, 5.0, 2.0, 5.0])
        {
            assert!((actual - expected).abs() < 1e-7);
        }
        let mesh = solid.tessellate(0.05, 0.35).unwrap();
        assert!(mesh.vertices_mm.len() > 100);
        assert!(mesh.triangles.len() > 100);

        let profile_touching_axis = [[0.0, -2.0], [5.0, -2.0], [5.0, 2.0], [0.0, 2.0]];
        assert!(matches!(
            kernel.revolved_polygon_mm(&profile_touching_axis),
            Err(KernelError::Operation { status: 1, .. })
        ));
    }

    #[test]
    fn open_lines_and_polylines_extrude_to_measured_surfaces() {
        let kernel = kernel();
        let line = kernel
            .extruded_open_polyline_surface_mm(&[[-5.0, 0.0], [5.0, 0.0]], 4.0)
            .unwrap();
        assert!(line.is_valid().unwrap());
        let line_properties = line.mass_properties().unwrap();
        assert!((line_properties[1] - 40.0).abs() < 1e-8);
        for (actual, expected) in line
            .bounds_mm()
            .unwrap()
            .into_iter()
            .zip([-5.0, 0.0, 0.0, 5.0, 0.0, 4.0])
        {
            assert!((actual - expected).abs() < 1e-7);
        }
        let mesh = line.tessellate(0.05, 0.35).unwrap();
        assert!(mesh.vertices_mm.len() >= 4);
        assert!(mesh.triangles.len() >= 2);

        let path = kernel
            .extruded_open_polyline_surface_mm(&[[-5.0, 0.0], [0.0, 5.0], [5.0, 0.0]], 4.0)
            .unwrap();
        assert!(path.is_valid().unwrap());
        let path_area = 20.0 * 2.0_f64.sqrt() * 2.0;
        assert!((path.mass_properties().unwrap()[1] - path_area).abs() < 1e-7);
        assert!(matches!(
            kernel.extruded_open_polyline_surface_mm(&[[0.0, 0.0], [0.0, 0.0]], 4.0),
            Err(KernelError::Operation { status: 1, .. })
        ));
    }

    #[test]
    fn open_curve_surfaces_extrude_along_arbitrary_3d_directions() {
        let kernel = kernel();
        let line = kernel
            .extruded_open_polyline_surface_vector_mm(
                &[[-5.0, 0.0], [5.0, 0.0]],
                5.0,
                [0.0, 3.0, 4.0],
            )
            .unwrap();
        assert!(line.is_valid().unwrap());
        assert!((line.mass_properties().unwrap()[1] - 50.0).abs() < 1e-8);
        for (actual, expected) in line
            .bounds_mm()
            .unwrap()
            .into_iter()
            .zip([-5.0, 0.0, 0.0, 5.0, 3.0, 4.0])
        {
            assert!((actual - expected).abs() < 1e-8);
        }
        assert!(matches!(
            kernel.extruded_open_polyline_surface_vector_mm(
                &[[-5.0, 0.0], [5.0, 0.0]],
                5.0,
                [0.0; 3]
            ),
            Err(KernelError::Operation { status: 1, .. })
        ));

        let arc = kernel
            .extruded_arc_surface_vector_mm(2.0, 0.0, 90.0, 5.0, [0.0, 3.0, 4.0])
            .unwrap();
        assert!(arc.is_valid().unwrap());
        for (actual, expected) in arc
            .bounds_mm()
            .unwrap()
            .into_iter()
            .zip([0.0, 0.0, 0.0, 2.0, 5.0, 4.0])
        {
            assert!((actual - expected).abs() < 1e-7);
        }
    }

    #[test]
    fn closed_profiles_extrude_to_solids_along_arbitrary_3d_directions() {
        let kernel = kernel();
        let direction = [0.0, 3.0, 4.0];

        let polygon = kernel
            .extruded_polygon_vector_mm(
                &[[0.0, 0.0], [4.0, 0.0], [4.0, 3.0], [0.0, 3.0]],
                5.0,
                direction,
            )
            .unwrap();
        assert!(polygon.is_valid().unwrap());
        assert!((polygon.volume_mm3().unwrap() - 48.0).abs() < 1e-8);
        for (actual, expected) in polygon
            .bounds_mm()
            .unwrap()
            .into_iter()
            .zip([0.0, 0.0, 0.0, 4.0, 6.0, 4.0])
        {
            assert!((actual - expected).abs() < 1e-7);
        }

        let circle = kernel
            .extruded_circle_vector_mm(2.0, 5.0, direction)
            .unwrap();
        assert!(circle.is_valid().unwrap());
        let circle_volume = circle.volume_mm3().unwrap();
        assert!(
            (circle_volume - 16.0 * std::f64::consts::PI).abs() < 1e-4,
            "directional circle volume was {circle_volume}"
        );
        for (actual, expected) in circle
            .bounds_mm()
            .unwrap()
            .into_iter()
            .zip([-2.0, -2.0, 0.0, 2.0, 5.0, 4.0])
        {
            assert!((actual - expected).abs() < 1e-4);
        }

        let ellipse = kernel
            .extruded_ellipse_vector_mm(4.0, 2.0, 5.0, direction)
            .unwrap();
        assert!(ellipse.is_valid().unwrap());
        assert!((ellipse.volume_mm3().unwrap() - 32.0 * std::f64::consts::PI).abs() < 1e-4);
        for (actual, expected) in ellipse
            .bounds_mm()
            .unwrap()
            .into_iter()
            .zip([-4.0, -2.0, 0.0, 4.0, 5.0, 4.0])
        {
            assert!((actual - expected).abs() < 1e-4);
        }

        assert!(matches!(
            kernel.extruded_polygon_vector_mm(&[[0.0, 0.0], [4.0, 0.0], [0.0, 3.0]], 5.0, [0.0; 3]),
            Err(KernelError::Operation { status: 1, .. })
        ));
        assert!(matches!(
            kernel.extruded_circle_vector_mm(2.0, 5.0, [0.0; 3]),
            Err(KernelError::Operation { status: 1, .. })
        ));
        assert!(matches!(
            kernel.extruded_ellipse_vector_mm(4.0, 2.0, 5.0, [0.0; 3]),
            Err(KernelError::Operation { status: 1, .. })
        ));
        assert!(matches!(
            kernel.extruded_circle_vector_mm(2.0, 5.0, [1.0, 0.0, 0.0]),
            Err(KernelError::Operation { status: 1, .. })
        ));
    }

    #[test]
    fn an_open_arc_extrudes_to_an_exact_cylindrical_surface() {
        let kernel = kernel();
        let surface = kernel.extruded_arc_surface_mm(2.0, 0.0, 90.0, 3.0).unwrap();
        assert!(surface.is_valid().unwrap());
        let properties = surface.mass_properties().unwrap();
        assert!(properties[0].abs() < 1e-10);
        assert!((properties[1] - 3.0 * std::f64::consts::PI).abs() < 1e-7);
        for (actual, expected) in surface
            .bounds_mm()
            .unwrap()
            .into_iter()
            .zip([0.0, 0.0, 0.0, 2.0, 2.0, 3.0])
        {
            assert!((actual - expected).abs() < 1e-7);
        }
        let mesh = surface.tessellate(0.05, 0.35).unwrap();
        assert!(mesh.vertices_mm.len() >= 4);
        assert!(mesh.triangles.len() >= 2);
        assert!(matches!(
            kernel.extruded_arc_surface_mm(2.0, 0.0, 360.0, 3.0),
            Err(KernelError::Operation { status: 1, .. })
        ));
    }

    #[test]
    fn an_open_profile_revolves_to_a_measured_surface_shell() {
        let kernel = kernel();
        let surface = kernel
            .revolved_open_polyline_surface_mm(&[[5.0, -2.0], [5.0, 2.0]])
            .unwrap();
        assert!(surface.is_valid().unwrap());
        let properties = surface.mass_properties().unwrap();
        assert!(properties[0].abs() < 1e-10);
        assert!((properties[1] - 40.0 * std::f64::consts::PI).abs() < 1e-7);
        for (actual, expected) in surface
            .bounds_mm()
            .unwrap()
            .into_iter()
            .zip([-5.0, -2.0, -5.0, 5.0, 2.0, 5.0])
        {
            assert!((actual - expected).abs() < 1e-7);
        }
        let mesh = surface.tessellate(0.05, 0.35).unwrap();
        assert!(mesh.vertices_mm.len() >= 32);
        assert!(mesh.triangles.len() >= 32);
        assert!(matches!(
            kernel.revolved_open_polyline_surface_mm(&[[0.0, -2.0], [0.0, 2.0]]),
            Err(KernelError::Operation { status: 1, .. })
        ));
    }

    #[test]
    fn closed_polyline_becomes_a_measured_curve_and_planar_surface() {
        let kernel = kernel();
        let rectangle = [[0.0, 0.0], [4.0, 0.0], [4.0, 3.0], [0.0, 3.0]];
        let curve = kernel.closed_polyline_mm(&rectangle).unwrap();
        assert!(curve.is_valid().unwrap());
        assert!((curve.length_mm().unwrap() - 14.0).abs() < 1e-8);

        let surface = kernel.planar_polygon_surface_mm(&rectangle).unwrap();
        assert!(surface.is_valid().unwrap());
        let properties = surface.mass_properties().unwrap();
        assert!(properties[0].abs() < 1e-12);
        assert!((properties[1] - 12.0).abs() < 1e-8);
        assert!((properties[2] - 2.0).abs() < 1e-8);
        assert!((properties[3] - 1.5).abs() < 1e-8);
        let mesh = surface.tessellate(0.05, 0.35).unwrap();
        assert!(mesh.vertices_mm.len() >= 4);
        assert!(mesh.triangles.len() >= 2);

        let bow_tie = [[0.0, 0.0], [4.0, 3.0], [0.0, 3.0], [4.0, 0.0]];
        assert!(kernel.closed_polyline_mm(&bow_tie).is_err());
        assert!(kernel.planar_polygon_surface_mm(&bow_tie).is_err());
    }

    #[test]
    fn circular_profile_moves_from_curve_to_surface_to_solid() {
        let kernel = kernel();
        let curve = kernel.circle_curve_mm(2.0).unwrap();
        assert!(curve.is_valid().unwrap());
        assert!((curve.length_mm().unwrap() - std::f64::consts::TAU * 2.0).abs() < 1e-7);

        let surface = kernel.planar_circle_surface_mm(2.0).unwrap();
        assert!(surface.is_valid().unwrap());
        let properties = surface.mass_properties().unwrap();
        assert!(properties[0].abs() < 1e-12);
        assert!((properties[1] - std::f64::consts::PI * 4.0).abs() < 1e-7);
        let mesh = surface.tessellate(0.05, 0.35).unwrap();
        assert!(mesh.vertices_mm.len() >= 16);
        assert!(mesh.triangles.len() >= 14);

        let solid = kernel.extruded_circle_mm(2.0, 3.0).unwrap();
        assert!(solid.is_valid().unwrap());
        assert!((solid.volume_mm3().unwrap() - std::f64::consts::PI * 12.0).abs() < 1e-7);
    }

    #[test]
    fn elliptical_profile_moves_from_curve_to_surface_to_solid() {
        let kernel = kernel();
        let curve = kernel.ellipse_curve_mm(4.0, 2.0).unwrap();
        assert!(curve.is_valid().unwrap());
        let length = curve.length_mm().unwrap();
        assert!((length - 19.3769).abs() < 0.002);
        let bounds = curve.bounds_mm().unwrap();
        assert!((bounds[3] - bounds[0] - 8.0).abs() < 1e-7);
        assert!((bounds[4] - bounds[1] - 4.0).abs() < 1e-7);

        let surface = kernel.planar_ellipse_surface_mm(4.0, 2.0).unwrap();
        assert!(surface.is_valid().unwrap());
        let properties = surface.mass_properties().unwrap();
        assert!(properties[0].abs() < 1e-12);
        assert!((properties[1] - std::f64::consts::PI * 8.0).abs() < 1e-7);
        let mesh = surface.tessellate(0.05, 0.35).unwrap();
        assert!(mesh.vertices_mm.len() >= 16);
        assert!(mesh.triangles.len() >= 14);

        let solid = kernel.extruded_ellipse_mm(4.0, 2.0, 3.0).unwrap();
        assert!(solid.is_valid().unwrap());
        let expected_volume = std::f64::consts::PI * 24.0;
        assert!((solid.volume_mm3().unwrap() - expected_volume).abs() < expected_volume * 1e-6);

        let circle = kernel.ellipse_curve_mm(2.0, 2.0).unwrap();
        assert!((circle.length_mm().unwrap() - std::f64::consts::TAU * 2.0).abs() < 1e-7);
    }

    #[test]
    fn open_arc_has_expected_length_bounds_and_validation() {
        let kernel = kernel();
        let arc = kernel.arc_curve_mm(2.0, 270.0, 90.0).unwrap();
        assert!(arc.is_valid().unwrap());
        assert!((arc.length_mm().unwrap() - std::f64::consts::PI).abs() < 1e-7);
        for (actual, expected) in arc
            .bounds_mm()
            .unwrap()
            .into_iter()
            .zip([0.0, -2.0, 0.0, 2.0, 0.0, 0.0])
        {
            assert!((actual - expected).abs() < 1e-7);
        }
        assert!(matches!(
            kernel.arc_curve_mm(2.0, 0.0, 360.0),
            Err(KernelError::Operation { status: 1, .. })
        ));
    }

    #[test]
    fn open_line_has_expected_length_bounds_and_rejects_equal_endpoints() {
        let kernel = kernel();
        let line = kernel.line_curve_mm([-1.0, 2.0], [2.0, 6.0]).unwrap();
        assert!(line.is_valid().unwrap());
        assert!((line.length_mm().unwrap() - 5.0).abs() < 1e-9);
        for (actual, expected) in line
            .bounds_mm()
            .unwrap()
            .into_iter()
            .zip([-1.0, 2.0, 0.0, 2.0, 6.0, 0.0])
        {
            assert!((actual - expected).abs() < 1e-7);
        }
        assert!(matches!(
            kernel.line_curve_mm([1.0, 1.0], [1.0, 1.0]),
            Err(KernelError::Operation { status: 1, .. })
        ));
    }

    #[test]
    fn cubic_bezier_curve_and_directional_surface_are_valid() {
        let kernel = kernel();
        let control_points = [[-5.0, 0.0], [-2.0, 6.0], [2.0, -6.0], [5.0, 0.0]];
        let curve = kernel.bezier_curve_mm(&control_points).unwrap();
        assert!(curve.is_valid().unwrap());
        let length = curve.length_mm().unwrap();
        assert!(
            length > 10.0 && length < 20.0,
            "unexpected Bezier length: {length}"
        );

        let surface = kernel
            .extruded_bezier_surface_vector_mm(&control_points, 5.0, [0.0, 0.0, 1.0])
            .unwrap();
        assert!(surface.is_valid().unwrap());
        let properties = surface.mass_properties().unwrap();
        assert!((properties[1] - length * 5.0).abs() < length * 5.0 * 1e-2);
        let bounds = surface.bounds_mm().unwrap();
        assert!((bounds[5] - 5.0).abs() < 1e-5);
    }

    #[test]
    fn cubic_bezier_rejects_wrong_control_point_count() {
        assert!(matches!(
            kernel().bezier_curve_mm(&[[-1.0, 0.0], [1.0, 0.0]]),
            Err(KernelError::Operation { status: 1, .. })
        ));
    }

    #[test]
    fn open_polyline_preserves_all_segments_without_closing_the_wire() {
        let kernel = kernel();
        let points = [[-1.0, 2.0], [2.0, 6.0], [6.0, 3.0]];
        let curve = kernel.open_polyline_curve_mm(&points).unwrap();
        assert!(curve.is_valid().unwrap());
        assert!((curve.length_mm().unwrap() - 10.0).abs() < 1e-9);
        for (actual, expected) in curve
            .bounds_mm()
            .unwrap()
            .into_iter()
            .zip([-1.0, 2.0, 0.0, 6.0, 6.0, 0.0])
        {
            assert!((actual - expected).abs() < 1e-7);
        }
        assert!(matches!(
            kernel.open_polyline_curve_mm(&[[-1.0, 2.0], [-1.0, 2.0]]),
            Err(KernelError::Operation { status: 1, .. })
        ));
    }

    #[test]
    fn shape_bounds_and_transform_match_expected_dimensions() {
        let kernel = kernel();
        let shape = kernel.box_mm(2.0, 3.0, 4.0).unwrap();
        let bounds = shape.bounds_mm().unwrap();
        for (actual, expected) in bounds.into_iter().zip([0.0, 0.0, 0.0, 2.0, 3.0, 4.0]) {
            assert!((actual - expected).abs() < 1e-7);
        }
        for (actual, expected) in shape
            .mass_properties()
            .unwrap()
            .into_iter()
            .zip([24.0, 52.0, 1.0, 1.5, 2.0])
        {
            assert!((actual - expected).abs() < 1e-7);
        }

        let transformed = shape
            .transformed_mm_degrees([1.0, -2.0, 3.0], [0.0, 0.0, 90.0], 2.0)
            .unwrap();
        assert!(transformed.is_valid().unwrap());
        assert!((transformed.volume_mm3().unwrap() - 192.0).abs() < 1e-7);
        let bounds = transformed.bounds_mm().unwrap();
        for (actual, expected) in bounds.into_iter().zip([-5.0, -2.0, 3.0, 1.0, 2.0, 11.0]) {
            assert!((actual - expected).abs() < 1e-6);
        }
        for (actual, expected) in transformed
            .mass_properties()
            .unwrap()
            .into_iter()
            .zip([192.0, 208.0, -2.0, 0.0, 7.0])
        {
            assert!((actual - expected).abs() < 1e-6);
        }
    }

    #[test]
    fn occt_tessellation_preserves_box_volume_and_valid_indices() {
        let kernel = kernel();
        let shape = kernel.box_mm(2.0, 3.0, 4.0).unwrap();
        let mesh = shape.tessellate(0.05, 0.35).unwrap();
        assert!(mesh.vertices_mm.len() >= 8);
        assert_eq!(mesh.triangles.len(), 12);
        assert!(mesh
            .vertices_mm
            .iter()
            .flatten()
            .all(|coordinate| coordinate.is_finite()));
        let signed_volume = mesh
            .triangles
            .iter()
            .map(|triangle| {
                let a = mesh.vertices_mm[triangle[0] as usize];
                let b = mesh.vertices_mm[triangle[1] as usize];
                let c = mesh.vertices_mm[triangle[2] as usize];
                (a[0] * (b[1] * c[2] - b[2] * c[1])
                    + a[1] * (b[2] * c[0] - b[0] * c[2])
                    + a[2] * (b[0] * c[1] - b[1] * c[0]))
                    / 6.0
            })
            .sum::<f64>();
        assert!((signed_volume - 24.0).abs() < 1e-8);
        assert!(mesh
            .triangles
            .iter()
            .flatten()
            .all(|index| (*index as usize) < mesh.vertices_mm.len()));
    }

    #[test]
    fn tessellation_rejects_invalid_deflection() {
        let shape = kernel().box_mm(2.0, 3.0, 4.0).unwrap();
        assert!(matches!(
            shape.tessellate(0.0, 0.35),
            Err(KernelError::Operation { status: 1, .. })
        ));
    }

    #[test]
    fn mirror_reflects_shapes_across_world_origin_planes() {
        let kernel = kernel();
        let shape = kernel.box_mm(2.0, 3.0, 4.0).unwrap();
        let mirrored = shape.mirrored(1).unwrap();
        assert!(mirrored.is_valid().unwrap());
        assert!((mirrored.volume_mm3().unwrap() - 24.0).abs() < 1e-8);
        for (actual, expected) in mirrored
            .bounds_mm()
            .unwrap()
            .into_iter()
            .zip([-2.0, 0.0, 0.0, 0.0, 3.0, 4.0])
        {
            assert!((actual - expected).abs() < 1e-8);
        }
        assert!(matches!(
            shape.mirrored(8),
            Err(KernelError::Operation { status: 1, .. })
        ));
    }

    #[test]
    fn exact_boolean_operations_return_valid_occt_solids() {
        let kernel = kernel();
        let base = kernel.box_mm(2.0, 2.0, 2.0).unwrap();
        let overlapping_tool = kernel
            .box_mm(2.0, 2.0, 2.0)
            .unwrap()
            .transformed_mm_degrees([1.0, 0.0, 0.0], [0.0; 3], 1.0)
            .unwrap();
        for (operation, expected_volume) in [
            (BooleanOperation::Union, 12.0),
            (BooleanOperation::Difference, 4.0),
            (BooleanOperation::Intersection, 4.0),
        ] {
            let result = base
                .boolean_with(&overlapping_tool, operation, 1e-7)
                .unwrap();
            assert!(result.is_valid().unwrap());
            assert!((result.volume_mm3().unwrap() - expected_volume).abs() < 1e-7);
        }
    }

    #[test]
    fn linear_and_polar_arrays_preserve_valid_solid_instances() {
        let kernel = kernel();
        let block = kernel.box_mm(2.0, 2.0, 2.0).unwrap();
        let linear = block.linear_array(3, [4.0, 0.0, 0.0]).unwrap();
        assert!(linear.is_valid().unwrap());
        assert!((linear.volume_mm3().unwrap() - 24.0).abs() < 1e-8);
        for (actual, expected) in linear
            .bounds_mm()
            .unwrap()
            .into_iter()
            .zip([0.0, 0.0, 0.0, 10.0, 2.0, 2.0])
        {
            assert!((actual - expected).abs() < 1e-7);
        }
        assert_eq!(linear.tessellate(0.05, 0.35).unwrap().triangles.len(), 36);

        let offset = kernel
            .box_mm(1.0, 1.0, 1.0)
            .unwrap()
            .transformed_mm_degrees([2.0, 0.0, 0.0], [0.0; 3], 1.0)
            .unwrap();
        let polar = offset.polar_array_z(4, 90.0).unwrap();
        assert!(polar.is_valid().unwrap());
        assert!((polar.volume_mm3().unwrap() - 4.0).abs() < 1e-8);
        for (actual, expected) in polar
            .bounds_mm()
            .unwrap()
            .into_iter()
            .zip([-3.0, -3.0, 0.0, 3.0, 3.0, 1.0])
        {
            assert!(
                (actual - expected).abs() < 1e-6,
                "polar array bound {actual} did not match expected {expected}"
            );
        }
    }

    #[test]
    fn arrays_reject_unbounded_or_degenerate_inputs() {
        let block = kernel().box_mm(1.0, 1.0, 1.0).unwrap();
        assert!(matches!(
            block.linear_array(1, [1.0, 0.0, 0.0]),
            Err(KernelError::Operation { status: 1, .. })
        ));
        assert!(matches!(
            block.linear_array(2, [0.0; 3]),
            Err(KernelError::Operation { status: 1, .. })
        ));
        assert!(matches!(
            block.polar_array_z(2, 360.0),
            Err(KernelError::Operation { status: 1, .. })
        ));
    }

    #[test]
    fn boolean_operations_report_empty_and_invalid_cases() {
        let kernel = kernel();
        let base = kernel.box_mm(1.0, 1.0, 1.0).unwrap();
        let disjoint_tool = kernel
            .box_mm(1.0, 1.0, 1.0)
            .unwrap()
            .transformed_mm_degrees([3.0, 0.0, 0.0], [0.0; 3], 1.0)
            .unwrap();
        assert!(matches!(
            base.boolean_with(&disjoint_tool, BooleanOperation::Intersection, 1e-7),
            Err(KernelError::Operation { status: 3, .. })
        ));
        assert!(matches!(
            base.boolean_with(&disjoint_tool, BooleanOperation::Union, 0.0),
            Err(KernelError::Operation { status: 1, .. })
        ));
    }

    #[test]
    fn invalid_primitives_are_structured_errors() {
        let kernel = kernel();
        assert!(matches!(
            kernel.cylinder_mm(2.0, 0.0),
            Err(KernelError::Operation { status: 1, .. })
        ));
        assert!(matches!(
            kernel.cone_mm(2.0, 0.0),
            Err(KernelError::Operation { status: 1, .. })
        ));
        assert!(matches!(
            kernel.truncated_cone_mm(2.0, 2.0, 5.0),
            Err(KernelError::Operation { status: 1, .. })
        ));
        assert!(matches!(
            kernel.ellipsoid_mm(2.0, 0.0, 3.0),
            Err(KernelError::Operation { status: 1, .. })
        ));
        assert!(matches!(
            kernel.pipe_mm(2.0, 2.0, 5.0),
            Err(KernelError::Operation { status: 1, .. })
        ));
        assert!(matches!(
            kernel.sphere_mm(f64::NAN),
            Err(KernelError::Operation { status: 1, .. })
        ));
        assert!(matches!(
            kernel.torus_mm(1.0, 1.0),
            Err(KernelError::Operation { status: 1, .. })
        ));
        let box_shape = kernel.box_mm(1.0, 1.0, 1.0).unwrap();
        assert!(matches!(
            box_shape.transformed_mm_degrees([0.0; 3], [0.0; 3], 0.0),
            Err(KernelError::Operation { status: 1, .. })
        ));
    }
}
