//! Optional in-process nimvault via libnimvault.so (dlopen).
//! Set `NIMVAULT_LIB` or install to `~/.local/lib/libnimvault.so`.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use libloading::Library;

type NvListFn = unsafe extern "C" fn(*const c_char, *const c_char) -> *mut c_char;
type NvStatusFn = unsafe extern "C" fn(*const c_char, *const c_char) -> *mut c_char;
type NvFreeFn = unsafe extern "C" fn(*mut std::ffi::c_void);
type NvLastErrorFn = unsafe extern "C" fn() -> *const c_char;
type NvVersionFn = unsafe extern "C" fn() -> *const c_char;

struct LibApi {
    _lib: Library,
    list: NvListFn,
    status: NvStatusFn,
    free: NvFreeFn,
    last_error: NvLastErrorFn,
    version: NvVersionFn,
}

static API: OnceLock<Option<&'static LibApi>> = OnceLock::new();

fn candidate_paths() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Ok(p) = std::env::var("NIMVAULT_LIB") {
        v.push(PathBuf::from(p));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            v.push(dir.join("libnimvault.so"));
            v.push(dir.join("../lib/libnimvault.so"));
        }
    }
    v.push(PathBuf::from("lib/libnimvault.so"));
    if let Ok(h) = std::env::var("HOME") {
        v.push(PathBuf::from(&h).join(".local/lib/libnimvault.so"));
        v.push(PathBuf::from(&h).join("Git/Github/Tools/nimvault/lib/libnimvault.so"));
    }
    v.push(PathBuf::from("/usr/local/lib/libnimvault.so"));
    v
}

fn load() -> Option<&'static LibApi> {
    *API.get_or_init(|| {
        for path in candidate_paths() {
            if !path.is_file() {
                continue;
            }
            let lib = match unsafe { Library::new(&path) } {
                Ok(l) => l,
                Err(_) => continue,
            };
            unsafe {
                let Ok(list) = lib.get::<NvListFn>(b"nv_list\0") else { continue };
                let Ok(status) = lib.get::<NvStatusFn>(b"nv_status\0") else { continue };
                let Ok(free) = lib.get::<NvFreeFn>(b"nv_free\0") else { continue };
                let Ok(last_error) = lib.get::<NvLastErrorFn>(b"nv_last_error\0") else { continue };
                let Ok(version) = lib.get::<NvVersionFn>(b"nv_version\0") else { continue };
                let api = LibApi {
                    list: *list,
                    status: *status,
                    free: *free,
                    last_error: *last_error,
                    version: *version,
                    _lib: lib,
                };
                return Some(Box::leak(Box::new(api)));
            }
        }
        None
    })
}

pub fn lib_loaded() -> bool {
    load().is_some()
}

pub fn lib_version() -> Option<String> {
    let api = load()?;
    unsafe {
        let p = (api.version)();
        if p.is_null() {
            return None;
        }
        Some(CStr::from_ptr(p).to_string_lossy().into_owned())
    }
}

/// Returns (ok, stdout, stderr) for list/status; None if lib missing or op unsupported.
pub fn try_inproc(op: &str, workdir: &Path, recipient: Option<&str>) -> Option<(bool, String, String)> {
    let api = load()?;
    if op != "list" && op != "status" {
        return None;
    }
    let repo = CString::new(workdir.to_string_lossy().as_bytes()).ok()?;
    let rec = CString::new(recipient.unwrap_or("")).ok()?;
    let ptr = unsafe {
        if op == "list" {
            (api.list)(repo.as_ptr(), rec.as_ptr())
        } else {
            (api.status)(repo.as_ptr(), rec.as_ptr())
        }
    };
    if ptr.is_null() {
        let err = unsafe { (api.last_error)() };
        let msg = if err.is_null() {
            "libnimvault error".into()
        } else {
            unsafe { CStr::from_ptr(err).to_string_lossy().into_owned() }
        };
        return Some((false, String::new(), msg));
    }
    let s = unsafe { CStr::from_ptr(ptr).to_string_lossy().into_owned() };
    unsafe {
        (api.free)(ptr as *mut std::ffi::c_void);
    }
    Some((true, s, String::new()))
}
