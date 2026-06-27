//! In-process nimvault via libnimvault.so (dlopen). CLI fallback when missing.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use libloading::Library;

type NvFreeFn = unsafe extern "C" fn(*mut c_void);
type NvLastErrorFn = unsafe extern "C" fn() -> *const c_char;
type NvVersionFn = unsafe extern "C" fn() -> *const c_char;
type NvRR = unsafe extern "C" fn(*const c_char, *const c_char) -> *mut c_char;
type NvUnseal = unsafe extern "C" fn(*const c_char, *const c_char, c_int) -> *mut c_char;
type NvAdd = unsafe extern "C" fn(*const c_char, *const c_char, *const c_char, c_int) -> *mut c_char;
type NvMv = unsafe extern "C" fn(*const c_char, *const c_char, *const c_char, *const c_char) -> *mut c_char;
type NvScan = unsafe extern "C" fn(*const c_char, *const c_char, *const c_char) -> *mut c_char;

struct LibApi {
    _lib: Library,
    free: NvFreeFn,
    last_error: NvLastErrorFn,
    version: NvVersionFn,
    list: Option<NvRR>,
    status: Option<NvRR>,
    seal: Option<NvRR>,
    unseal: Option<NvUnseal>,
    add: Option<NvAdd>,
    add_dir: Option<NvAdd>,
    remove: Option<NvRR>, // repo, path, recipient — wait remove is repo, path, recipient = 3 cstrings
    // redefine remove as 3-arg
    remove3: Option<unsafe extern "C" fn(*const c_char, *const c_char, *const c_char) -> *mut c_char>,
    mv: Option<NvMv>,
    scan: Option<NvScan>,
}

static API: OnceLock<Option<&'static LibApi>> = OnceLock::new();

fn candidates() -> Vec<PathBuf> {
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
        for path in candidates() {
            if !path.is_file() {
                continue;
            }
            let lib = match unsafe { Library::new(&path) } {
                Ok(l) => l,
                Err(_) => continue,
            };
            unsafe {
                let Ok(free) = lib.get::<NvFreeFn>(b"nv_free\0") else { continue };
                let Ok(last_error) = lib.get::<NvLastErrorFn>(b"nv_last_error\0") else { continue };
                let Ok(version) = lib.get::<NvVersionFn>(b"nv_version\0") else { continue };
                let list = lib.get::<NvRR>(b"nv_list\0").ok().map(|s| *s);
                let status = lib.get::<NvRR>(b"nv_status\0").ok().map(|s| *s);
                let seal = lib.get::<NvRR>(b"nv_seal\0").ok().map(|s| *s);
                let unseal = lib.get::<NvUnseal>(b"nv_unseal\0").ok().map(|s| *s);
                let add = lib.get::<NvAdd>(b"nv_add\0").ok().map(|s| *s);
                let add_dir = lib.get::<NvAdd>(b"nv_add_dir\0").ok().map(|s| *s);
                let remove3 = lib
                    .get::<unsafe extern "C" fn(*const c_char, *const c_char, *const c_char) -> *mut c_char>(
                        b"nv_remove\0",
                    )
                    .ok()
                    .map(|s| *s);
                let mv = lib.get::<NvMv>(b"nv_mv\0").ok().map(|s| *s);
                let scan = lib.get::<NvScan>(b"nv_scan\0").ok().map(|s| *s);
                let api = LibApi {
                    free: *free,
                    last_error: *last_error,
                    version: *version,
                    list,
                    status,
                    seal,
                    unseal,
                    add,
                    add_dir,
                    remove: None,
                    remove3,
                    mv,
                    scan,
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
            None
        } else {
            Some(CStr::from_ptr(p).to_string_lossy().into_owned())
        }
    }
}

fn take(api: &LibApi, ptr: *mut c_char) -> (bool, String, String) {
    if ptr.is_null() {
        let err = unsafe { (api.last_error)() };
        let msg = if err.is_null() {
            "libnimvault error".into()
        } else {
            unsafe { CStr::from_ptr(err).to_string_lossy().into_owned() }
        };
        return (false, String::new(), msg);
    }
    let s = unsafe { CStr::from_ptr(ptr).to_string_lossy().into_owned() };
    unsafe {
        (api.free)(ptr as *mut c_void);
    }
    (true, s, String::new())
}

/// Map CLI-style argv (first token = op) to in-process call. None => use CLI spawn.
pub fn try_inproc(args: &[String], workdir: &Path) -> Option<(bool, String, String)> {
    let api = load()?;
    let op = args.first()?.as_str();
    let recip = args
        .windows(2)
        .find(|w| w[0] == "--recipient")
        .map(|w| w[1].as_str())
        .unwrap_or("");
    let repo = CString::new(workdir.to_string_lossy().as_bytes()).ok()?;
    let rec = CString::new(recip).ok()?;

    let ptr = unsafe {
        match op {
            "list" => {
                let f = api.list?;
                f(repo.as_ptr(), rec.as_ptr())
            }
            "status" => {
                let f = api.status?;
                f(repo.as_ptr(), rec.as_ptr())
            }
            "seal" => {
                let f = api.seal?;
                f(repo.as_ptr(), rec.as_ptr())
            }
            "unseal" => {
                let f = api.unseal?;
                let allow = if args.iter().any(|a| a == "--allow-unsigned") {
                    1
                } else {
                    0
                };
                f(repo.as_ptr(), rec.as_ptr(), allow)
            }
            "add" => {
                let f = api.add?;
                let path = args.get(1)?;
                let p = CString::new(path.as_str()).ok()?;
                let ng = if args.iter().any(|a| a == "--no-gitignore") {
                    1
                } else {
                    0
                };
                f(repo.as_ptr(), p.as_ptr(), rec.as_ptr(), ng)
            }
            "add-dir" => {
                let f = api.add_dir?;
                let path = args.get(1)?;
                let p = CString::new(path.as_str()).ok()?;
                let ng = if args.iter().any(|a| a == "--no-gitignore") {
                    1
                } else {
                    0
                };
                f(repo.as_ptr(), p.as_ptr(), rec.as_ptr(), ng)
            }
            "rm" | "remove" => {
                let f = api.remove3?;
                let path = args.get(1)?;
                let p = CString::new(path.as_str()).ok()?;
                f(repo.as_ptr(), p.as_ptr(), rec.as_ptr())
            }
            "mv" => {
                let f = api.mv?;
                let old = args.get(1)?;
                let newp = args.get(2)?;
                let o = CString::new(old.as_str()).ok()?;
                let n = CString::new(newp.as_str()).ok()?;
                f(repo.as_ptr(), o.as_ptr(), n.as_ptr(), rec.as_ptr())
            }
            "scan" => {
                let f = api.scan?;
                let path = args.get(1).map(|s| s.as_str()).unwrap_or("");
                let p = CString::new(path).ok()?;
                f(repo.as_ptr(), p.as_ptr(), rec.as_ptr())
            }
            _ => return None,
        }
    };
    Some(take(api, ptr))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn try_inproc_unknown_op_is_none_or_err() {
        // Without lib, None; with lib, unknown op returns None (CLI fallback).
        let r = try_inproc(&["not-a-real-op".into()], Path::new("/tmp"));
        assert!(r.is_none(), "unknown op must not claim inproc success");
    }

    #[test]
    fn try_inproc_bad_repo_errors_without_panic() {
        let lib = std::env::var("NIMVAULT_LIB").ok().filter(|p| Path::new(p).is_file());
        if lib.is_none() && !Path::new("/home/rgoswami/Git/Github/Tools/nimvault/lib/libnimvault.so").is_file() {
            return; // skip if no .so in CI without buildLib
        }
        if lib.is_none() {
            std::env::set_var(
                "NIMVAULT_LIB",
                "/home/rgoswami/Git/Github/Tools/nimvault/lib/libnimvault.so",
            );
        }
        // Reset OnceLock not possible; rely on env set before first load in this process.
        let r = try_inproc(
            &["list".into()],
            Path::new("/nonexistent/nimvault_lib_test_repo"),
        );
        if let Some((ok, _o, err)) = r {
            assert!(!ok, "bad repo must fail");
            assert!(!err.is_empty(), "error text required");
        }
    }
}
