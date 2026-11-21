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
type AbiVersion = unsafe extern "C" fn() -> u32;
type Volume = unsafe extern "C" fn(*const c_void, *mut f64, *mut CError) -> i32;
type IsValid = unsafe extern "C" fn(*const c_void, *mut i32, *mut CError) -> i32;
type Destroy = unsafe extern "C" fn(*mut c_void);

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn LoadLibraryW(name: *const u16) -> *mut c_void;
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
            let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            let handle = unsafe { LoadLibraryW(wide.as_ptr()) };
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
    volume: Volume,
    is_valid: IsValid,
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
            if actual != 1 {
                return Err(KernelError::IncompatibleAbi {
                    expected: 1,
                    actual,
                });
            }
            Api {
                box_create: library.symbol(b"ojcad_box_create\0")?,
                volume: library.symbol(b"ojcad_shape_volume_mm3\0")?,
                is_valid: library.symbol(b"ojcad_shape_is_valid\0")?,
                destroy: library.symbol(b"ojcad_shape_destroy\0")?,
                _library: library,
            }
        };
        Ok(Self { api: Arc::new(api) })
    }

    /// Load the path set in `OPENJEWELCAD_KERNEL_DLL`.
    pub fn from_environment() -> Result<Self, KernelError> {
        let path = env::var_os("OPENJEWELCAD_KERNEL_DLL").ok_or(KernelError::MissingKernelPath)?;
        Self::load(path)
    }

    /// Create an OCCT box with positive dimensions in millimetres.
    pub fn box_mm(&self, x: f64, y: f64, z: f64) -> Result<Shape, KernelError> {
        let mut raw = std::ptr::null_mut();
        let mut error = CError::default();
        let status = unsafe { (self.api.box_create)(x, y, z, &mut raw, &mut error) };
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
}
