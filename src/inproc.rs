//! Optional in-process nimvault via libnimvault.so (dlopen).
//!
//! Set `NIMVAULT_LIB` to the .so path, or we try common locations.
//! Falls back to CLI spawn when the library is missing.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use libloading::{Library, Symbol};

use crate::cli::NimvaultOutput;

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

static API: OnceLock<Option<LibApi>> = OnceLock::new();

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
    v.push(PathBuf::from("/home/rgoswami/Git/Github/Tools/nimvault/lib/libnimvault.so"));
    if let Ok(h) = std::env::var("HOME") {
        v.push(PathBuf::from(h).join(".local/lib/libnimvault.so"));
    }
    v.push(PathBuf::from("/usr/local/lib/libnimvault.so"));
    v
}

fn load() -> Option<&'static LibApi> {
    API.get_or_init(|| {
        for path in candidate_paths() {
            if !path.is_file() {
                continue;
            }
            // Safety: we only load our own built library.
            let lib = match unsafe { Library::new(&path) } {
                Ok(l) => l,
                Err(_) => continue,
            };
            unsafe {
                let list: Symbol<NvListFn> = match lib.get(b"nv_list\0") {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let status: Symbol<NvStatusFn> = match lib.get(b"nv_status\0") {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let free: Symbol<NvFreeFn> = match lib.get(b"nv_free\0") {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let last_error: Symbol<NvLastErrorFn> = match lib.get(b"nv_last_error\0") {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let version: Symbol<NvVersionFn> = match lib.get(b"nv_version\0") {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                // Leak library into static (process lifetime)
                let list = *list;
                let status = *status;
                let free = *free;
                let last_error = *last_error;
                let version = *version;
                let boxed = Box::new(LibApi {
                    _lib: lib,
                    list,
                    status,
                    free,
                    last_error,
                    version,
                });
                return Some(Box::leak(boxed));
            }
        }
        None
    })
    .as_ref()
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

fn take_c_string(api: &LibApi, ptr: *mut c_char) -> Result<String, String> {
    if ptr.is_null() {
        let err = unsafe { (api.last_error)() };
        let msg = if err.is_null() {
            "libnimvault returned null".into()
        } else {
            unsafe { CStr::from_ptr(err).to_string_lossy().into_owned() }
        };
        return Err(msg);
    }
    let s = unsafe { CStr::from_ptr(ptr).to_string_lossy().into_owned() };
    unsafe {
        (api.free)(ptr as *mut std::ffi::c_void);
    }
    Ok(s)
}

/// Run list/status in-process. Other commands return None → CLI fallback.
pub fn try_inproc(
    op: &str,
    workdir: &Path,
    recipient: Option<&str>,
) -> Option<Result<NimvaultOutput, String>> {
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
    let out = match take_c_string(api, ptr) {
        Ok(stdout) => Ok(NimvaultOutput {
            ok: true,
            code: Some(0),
            stdout,
            stderr: String::new(),
        }),
        Err(e) => Ok(NimvaultOutput {
            ok: false,
            code: Some(1),
            stdout: String::new(),
            stderr: e,
        }),
    };
    Some(out)
}
