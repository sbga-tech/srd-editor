use std::env;
use std::ffi::c_void;
#[cfg(target_os = "linux")]
use std::ffi::{CStr, c_char};
use std::path::{Path, PathBuf};
use std::ptr;

use super::bindings::{D3D_SDK_VERSION, HRESULT, HWND, IDirect3D9Ex, Interface};
use libloading::Library;

#[cfg(target_os = "windows")]
use super::bindings::GetDesktopWindow;

const DXVK_LIBRARY_ENV: &str = "SRD_EDITOR_DXVK_LIBRARY";
#[cfg(target_os = "windows")]
const DXVK_WINDOWS_PACKAGE_DIRECTORY: &str = "dxvk-3.0.2";
#[cfg(target_os = "linux")]
const SDL3_LIBRARY_ENV: &str = "SRD_EDITOR_SDL3_LIBRARY";

#[cfg(target_os = "linux")]
const SDL_INIT_VIDEO: u32 = 0x0000_0020;
#[cfg(target_os = "linux")]
const SDL_WINDOW_HIDDEN: u64 = 0x0000_0000_0000_0008;
#[cfg(target_os = "linux")]
const SDL_WINDOW_VULKAN: u64 = 0x0000_0000_1000_0000;
#[cfg(target_os = "linux")]
const DXVK_NATIVE_D3D9_LIBRARY: &str = "libdxvk_d3d9.so.0.30002";

type Direct3DCreate9ExFn = unsafe extern "system" fn(u32, *mut *mut c_void) -> HRESULT;

/// Loaded DXVK D3D9 provider plus the platform window it presents into.
///
/// The library is retained until after every COM interface has been released.
/// Windows and Linux therefore execute the same D3D9 renderer; only DXVK's WSI
/// handle differs (`HWND` on Windows, `SDL_Window*` on native Linux).
pub(super) struct DxvkProvider {
    window: DxvkWindow,
    _library: Library,
    create9ex: Direct3DCreate9ExFn,
    library_path: PathBuf,
}

impl DxvkProvider {
    pub(super) fn load() -> Result<Self, String> {
        #[cfg(target_os = "linux")]
        ensure_linux_wsi_driver()?;

        let window = DxvkWindow::new()?;
        let (library_path, library) = load_dxvk_library()?;
        let create9ex = unsafe {
            *library
                .get::<Direct3DCreate9ExFn>(b"Direct3DCreate9Ex\0")
                .map_err(|error| {
                    format!(
                        "DXVK library {} does not export Direct3DCreate9Ex: {error}",
                        library_path.display()
                    )
                })?
        };

        Ok(Self {
            window,
            _library: library,
            create9ex,
            library_path,
        })
    }

    pub(super) fn window_handle(&self) -> HWND {
        self.window.handle()
    }

    pub(super) fn create_direct3d9_ex(&self) -> Result<IDirect3D9Ex, String> {
        let mut raw = ptr::null_mut();
        let result = unsafe { (self.create9ex)(D3D_SDK_VERSION, &mut raw) };
        result.ok().map_err(|error| {
            format!(
                "DXVK Direct3DCreate9Ex from {} failed: {error}",
                self.library_path.display()
            )
        })?;
        if raw.is_null() {
            return Err(format!(
                "DXVK Direct3DCreate9Ex from {} succeeded but returned a null interface",
                self.library_path.display()
            ));
        }
        Ok(unsafe { IDirect3D9Ex::from_raw(raw) })
    }

    pub(super) fn label(&self) -> String {
        format!("DXVK D3D9Ex · {}", self.library_path.display())
    }
}

pub(crate) fn prepare_process_environment() {
    #[cfg(target_os = "linux")]
    if env::var_os("DXVK_WSI_DRIVER").is_none() {
        // SAFETY: `editor::run` invokes this before iced starts its executor or
        // creates application state. No other thread can concurrently inspect
        // or mutate the process environment at this point.
        unsafe { env::set_var("DXVK_WSI_DRIVER", "SDL3") };
    }
}

#[cfg(target_os = "linux")]
fn ensure_linux_wsi_driver() -> Result<(), String> {
    match env::var("DXVK_WSI_DRIVER") {
        Ok(driver) if driver == "SDL3" => Ok(()),
        Ok(driver) => Err(format!(
            "DXVK native requires DXVK_WSI_DRIVER=SDL3 for the editor's window provider; found {driver:?}"
        )),
        Err(_) => Err(
            "DXVK_WSI_DRIVER is unset; call renderer::prepare_process_environment before creating the preview backend"
                .into(),
        ),
    }
}

fn load_dxvk_library() -> Result<(PathBuf, Library), String> {
    let candidates = dxvk_library_candidates();
    let mut failures = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        match unsafe { Library::new(&candidate) } {
            Ok(library) => return Ok((candidate, library)),
            Err(error) => failures.push(format!("{}: {error}", candidate.display())),
        }
    }
    Err(format!(
        "DXVK D3D9 runtime was not found. Set {DXVK_LIBRARY_ENV} to the DXVK 3.0.2 D3D9 library or install it below runtime/dxvk. Tried: {}",
        failures.join("; ")
    ))
}

fn dxvk_library_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(path) = env::var_os(DXVK_LIBRARY_ENV) {
        extend_dxvk_candidate(&mut candidates, PathBuf::from(path));
    }
    if let Ok(executable) = env::current_exe()
        && let Some(directory) = executable.parent()
    {
        extend_dxvk_candidate(&mut candidates, directory.join("runtime/dxvk"));
    }
    if let Ok(directory) = env::current_dir() {
        extend_dxvk_candidate(&mut candidates, directory.join("runtime/dxvk"));
    }
    deduplicate_paths(candidates)
}

fn is_explicit_dxvk_library(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };

    #[cfg(target_os = "windows")]
    {
        name.eq_ignore_ascii_case("d3d9.dll")
    }
    #[cfg(target_os = "linux")]
    {
        name == "libdxvk_d3d9.so" || name.starts_with("libdxvk_d3d9.so.")
    }
}

fn extend_dxvk_candidate(candidates: &mut Vec<PathBuf>, path: PathBuf) {
    if path.is_file() || (!path.is_dir() && is_explicit_dxvk_library(&path)) {
        candidates.push(path);
        return;
    }

    #[cfg(target_os = "windows")]
    {
        let architecture = if cfg!(target_pointer_width = "64") {
            "x64"
        } else {
            "x32"
        };
        for directory in [
            path.join(architecture),
            path.join(DXVK_WINDOWS_PACKAGE_DIRECTORY).join(architecture),
            path,
        ] {
            candidates.push(directory.join("d3d9.dll"));
        }
    }

    #[cfg(target_os = "linux")]
    {
        let library_directory = if cfg!(target_pointer_width = "64") {
            "lib"
        } else {
            "lib32"
        };
        for directory in [
            path.join("usr").join(library_directory),
            path.join(library_directory),
            path,
        ] {
            for name in [
                DXVK_NATIVE_D3D9_LIBRARY,
                "libdxvk_d3d9.so.0",
                "libdxvk_d3d9.so",
            ] {
                candidates.push(directory.join(name));
            }
        }
    }
}

fn deduplicate_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut unique = Vec::with_capacity(paths.len());
    for path in paths {
        if !unique.contains(&path) {
            unique.push(path);
        }
    }
    unique
}

#[cfg(target_os = "windows")]
struct DxvkWindow {
    handle: HWND,
}

#[cfg(target_os = "windows")]
impl DxvkWindow {
    fn new() -> Result<Self, String> {
        let handle = unsafe { GetDesktopWindow() };
        if handle.0.is_null() {
            return Err("GetDesktopWindow returned a null HWND for DXVK presentation".into());
        }
        Ok(Self { handle })
    }

    fn handle(&self) -> HWND {
        self.handle
    }
}

#[cfg(target_os = "linux")]
type SdlInitFn = unsafe extern "C" fn(u32) -> bool;
#[cfg(target_os = "linux")]
type SdlCreateWindowFn = unsafe extern "C" fn(*const c_char, i32, i32, u64) -> *mut c_void;
#[cfg(target_os = "linux")]
type SdlDestroyWindowFn = unsafe extern "C" fn(*mut c_void);
#[cfg(target_os = "linux")]
type SdlQuitSubsystemFn = unsafe extern "C" fn(u32);
#[cfg(target_os = "linux")]
type SdlGetErrorFn = unsafe extern "C" fn() -> *const c_char;

#[cfg(target_os = "linux")]
struct DxvkWindow {
    handle: *mut c_void,
    destroy_window: SdlDestroyWindowFn,
    quit_subsystem: SdlQuitSubsystemFn,
    _sdl: Library,
}

#[cfg(target_os = "linux")]
impl DxvkWindow {
    fn new() -> Result<Self, String> {
        let (path, sdl) = load_sdl3_library()?;
        let init = unsafe { load_symbol::<SdlInitFn>(&sdl, &path, b"SDL_Init\0")? };
        let create_window =
            unsafe { load_symbol::<SdlCreateWindowFn>(&sdl, &path, b"SDL_CreateWindow\0")? };
        let destroy_window =
            unsafe { load_symbol::<SdlDestroyWindowFn>(&sdl, &path, b"SDL_DestroyWindow\0")? };
        let quit_subsystem =
            unsafe { load_symbol::<SdlQuitSubsystemFn>(&sdl, &path, b"SDL_QuitSubSystem\0")? };
        let get_error = unsafe { load_symbol::<SdlGetErrorFn>(&sdl, &path, b"SDL_GetError\0")? };

        if !unsafe { init(SDL_INIT_VIDEO) } {
            return Err(format!(
                "SDL3 video initialization from {} failed: {}",
                path.display(),
                unsafe { sdl_error(get_error) }
            ));
        }

        let handle = unsafe {
            create_window(
                c"SRD Editor DXVK Preview".as_ptr(),
                1,
                1,
                SDL_WINDOW_HIDDEN | SDL_WINDOW_VULKAN,
            )
        };
        if handle.is_null() {
            unsafe { quit_subsystem(SDL_INIT_VIDEO) };
            return Err(format!(
                "SDL3 failed to create the hidden Vulkan window: {}",
                unsafe { sdl_error(get_error) }
            ));
        }

        Ok(Self {
            handle,
            destroy_window,
            quit_subsystem,
            _sdl: sdl,
        })
    }

    fn handle(&self) -> HWND {
        HWND(self.handle)
    }
}

#[cfg(target_os = "linux")]
impl Drop for DxvkWindow {
    fn drop(&mut self) {
        unsafe {
            (self.destroy_window)(self.handle);
            (self.quit_subsystem)(SDL_INIT_VIDEO);
        }
    }
}

#[cfg(target_os = "linux")]
fn load_sdl3_library() -> Result<(PathBuf, Library), String> {
    let mut candidates = Vec::new();
    if let Some(path) = env::var_os(SDL3_LIBRARY_ENV) {
        candidates.push(PathBuf::from(path));
    }
    if let Ok(executable) = env::current_exe()
        && let Some(directory) = executable.parent()
    {
        candidates.push(directory.join("runtime/dxvk/libSDL3.so.0"));
        candidates.push(directory.join("runtime/dxvk/libSDL3.so"));
    }
    if let Ok(directory) = env::current_dir() {
        candidates.push(directory.join("runtime/dxvk/libSDL3.so.0"));
        candidates.push(directory.join("runtime/dxvk/libSDL3.so"));
    }
    candidates.push(PathBuf::from("libSDL3.so.0"));
    candidates.push(PathBuf::from("libSDL3.so"));

    let mut failures = Vec::with_capacity(candidates.len());
    for candidate in deduplicate_paths(candidates) {
        match unsafe { Library::new(&candidate) } {
            Ok(library) => return Ok((candidate, library)),
            Err(error) => failures.push(format!("{}: {error}", candidate.display())),
        }
    }
    Err(format!(
        "SDL3 is required by DXVK Native. Set {SDL3_LIBRARY_ENV} or package libSDL3.so.0 beside runtime/dxvk. Tried: {}",
        failures.join("; ")
    ))
}

#[cfg(target_os = "linux")]
unsafe fn load_symbol<T: Copy>(library: &Library, path: &Path, symbol: &[u8]) -> Result<T, String> {
    unsafe { library.get::<T>(symbol) }
        .map(|symbol| *symbol)
        .map_err(|error| {
            let name = CStr::from_bytes_with_nul(symbol)
                .ok()
                .and_then(|name| name.to_str().ok())
                .unwrap_or("<invalid symbol>");
            format!(
                "SDL3 library {} does not export {name}: {error}",
                path.display()
            )
        })
}

#[cfg(target_os = "linux")]
unsafe fn sdl_error(get_error: SdlGetErrorFn) -> String {
    let error = unsafe { get_error() };
    if error.is_null() {
        "unknown SDL3 error".into()
    } else {
        unsafe { CStr::from_ptr(error) }
            .to_string_lossy()
            .into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directory_candidates_never_include_system_d3d9() {
        let mut candidates = Vec::new();
        extend_dxvk_candidate(&mut candidates, PathBuf::from("runtime/dxvk"));
        assert!(!candidates.is_empty());
        assert!(
            candidates
                .iter()
                .all(|path| path.starts_with("runtime/dxvk"))
        );
        assert!(candidates.iter().all(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name != "d3d9")
        }));
    }

    #[test]
    fn versioned_directory_names_are_expanded_as_directories() {
        let root = PathBuf::from("runtime/dxvk-3.0.2");
        let mut candidates = Vec::new();

        extend_dxvk_candidate(&mut candidates, root.clone());

        assert!(!candidates.is_empty());
        assert!(candidates.iter().all(|path| path.starts_with(&root)));
        assert!(candidates.iter().all(|path| path != &root));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn official_windows_archive_layout_is_a_runtime_candidate() {
        let root = PathBuf::from("runtime/dxvk");
        let architecture = if cfg!(target_pointer_width = "64") {
            "x64"
        } else {
            "x32"
        };
        let expected = root
            .join(DXVK_WINDOWS_PACKAGE_DIRECTORY)
            .join(architecture)
            .join("d3d9.dll");
        let mut candidates = Vec::new();

        extend_dxvk_candidate(&mut candidates, root);

        assert!(candidates.contains(&expected));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn official_native_archive_layout_is_a_runtime_candidate() {
        let root = PathBuf::from("runtime/dxvk");
        let architecture = if cfg!(target_pointer_width = "64") {
            "lib"
        } else {
            "lib32"
        };
        let expected = root
            .join("usr")
            .join(architecture)
            .join(DXVK_NATIVE_D3D9_LIBRARY);
        let mut candidates = Vec::new();

        extend_dxvk_candidate(&mut candidates, root);

        assert!(candidates.contains(&expected));
    }

    #[test]
    fn explicit_library_paths_are_not_expanded() {
        #[cfg(target_os = "windows")]
        let library = PathBuf::from("custom/d3d9.dll");
        #[cfg(target_os = "linux")]
        let library = PathBuf::from("custom/libdxvk_d3d9.so.0");
        let mut candidates = Vec::new();

        extend_dxvk_candidate(&mut candidates, library.clone());

        assert_eq!(candidates, vec![library]);
    }

    #[test]
    fn candidate_deduplication_preserves_first_occurrence() {
        let first = PathBuf::from("runtime/dxvk/d3d9.dll");
        let second = PathBuf::from("runtime/dxvk/x64/d3d9.dll");
        assert_eq!(
            deduplicate_paths(vec![first.clone(), second.clone(), first.clone()]),
            vec![first, second]
        );
    }
}
