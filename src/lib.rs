#![recursion_limit = "512"]

use retour::static_detour;
use std::error::Error;
use std::mem;
use std::os::raw::c_void;
use windows::core::{BOOL, GUID, HRESULT, PCSTR, PCWSTR, PSTR, PWSTR};
use windows::Win32::Foundation::{HANDLE, HMODULE, HWND};
use windows::Win32::System::Com::CoTaskMemAlloc;
use windows::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows::Win32::System::SystemInformation::OSVERSIONINFOW;
use windows::Win32::System::SystemServices::{
    DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH, DLL_THREAD_ATTACH, DLL_THREAD_DETACH,
};
use windows::Win32::UI::Shell::Common::ITEMIDLIST;

mod utils;
use utils::AppDataKind;

const MAX_PATH_CHARS: usize = 260;
const S_OK: HRESULT = HRESULT(0);
const E_FAIL: HRESULT = HRESULT(0x80004005u32 as i32);
const E_INVALIDARG: HRESULT = HRESULT(0x80070057u32 as i32);
const E_OUTOFMEMORY: HRESULT = HRESULT(0x8007000Eu32 as i32);

const CSIDL_PERSONAL: i32 = 0x0005;
const CSIDL_MYDOCUMENTS: i32 = 0x000c;
const CSIDL_APPDATA: i32 = 0x001a;
const CSIDL_LOCAL_APPDATA: i32 = 0x001c;
const CSIDL_FLAG_MASK: i32 = 0xff00;
const CSIDL_FLAG_CREATE: i32 = 0x8000;

const FOLDERID_DOCUMENTS: GUID = GUID::from_u128(0xfdd39ad0_238f_46af_adb4_6c85480369c7);
const FOLDERID_LOCAL_APPDATA: GUID = GUID::from_u128(0xf1b32785_6fba_4fcf_9d55_7b8e7f157091);
const FOLDERID_LOCAL_APPDATA_LOW: GUID = GUID::from_u128(0xa520a1a4_1780_4ff6_bd18_167343c5af16);
const FOLDERID_ROAMING_APPDATA: GUID = GUID::from_u128(0x3eb685db_65f9_4cf6_a03a_e3ef65729f3d);
const FOLDERID_SAVED_GAMES: GUID = GUID::from_u128(0x4c5c32ff_bb9d_43b0_b5b4_2d72e54eaaa4);

static_detour! {
    static GetFolderPathAHook: unsafe extern "system" fn(HWND, i32, HANDLE, u32, PSTR) -> HRESULT;
    static GetFolderPathWHook: unsafe extern "system" fn(HWND, i32, HANDLE, u32, PWSTR) -> HRESULT;
    static GetKnownFolderPathHook: unsafe extern "system" fn(*const GUID, u32, HANDLE, *mut PWSTR) -> HRESULT;
    static GetPathFromIDListAHook: unsafe extern "system" fn(*const ITEMIDLIST, PSTR) -> BOOL;
    static GetPathFromIDListWHook: unsafe extern "system" fn(*const ITEMIDLIST, PWSTR) -> BOOL;
    static GetSpecialFolderPathAHook: unsafe extern "system" fn(HWND, PSTR, i32, BOOL) -> BOOL;
    static GetSpecialFolderPathWHook: unsafe extern "system" fn(HWND, PWSTR, i32, BOOL) -> BOOL;

    static GetEnvironmentVariableAHook: unsafe extern "system" fn(PCSTR, PSTR, u32) -> u32;
    static GetEnvironmentVariableWHook: unsafe extern "system" fn(PCWSTR, PWSTR, u32) -> u32;
    static ExpandEnvironmentStringsAHook: unsafe extern "system" fn(PCSTR, PSTR, u32) -> u32;
    static ExpandEnvironmentStringsWHook: unsafe extern "system" fn(PCWSTR, PWSTR, u32) -> u32;

    static CreateFileAHook: unsafe extern "system" fn(PCSTR, u32, u32, *const c_void, u32, u32, HANDLE) -> HANDLE;
    static CreateFileWHook: unsafe extern "system" fn(PCWSTR, u32, u32, *const c_void, u32, u32, HANDLE) -> HANDLE;
    static GetFileAttributesAHook: unsafe extern "system" fn(PCSTR) -> u32;
    static GetFileAttributesWHook: unsafe extern "system" fn(PCWSTR) -> u32;
    static SetFileAttributesAHook: unsafe extern "system" fn(PCSTR, u32) -> BOOL;
    static SetFileAttributesWHook: unsafe extern "system" fn(PCWSTR, u32) -> BOOL;
    static CreateDirectoryAHook: unsafe extern "system" fn(PCSTR, *const c_void) -> BOOL;
    static CreateDirectoryWHook: unsafe extern "system" fn(PCWSTR, *const c_void) -> BOOL;
    static RemoveDirectoryAHook: unsafe extern "system" fn(PCSTR) -> BOOL;
    static RemoveDirectoryWHook: unsafe extern "system" fn(PCWSTR) -> BOOL;
    static DeleteFileAHook: unsafe extern "system" fn(PCSTR) -> BOOL;
    static DeleteFileWHook: unsafe extern "system" fn(PCWSTR) -> BOOL;
    static CopyFileAHook: unsafe extern "system" fn(PCSTR, PCSTR, BOOL) -> BOOL;
    static CopyFileWHook: unsafe extern "system" fn(PCWSTR, PCWSTR, BOOL) -> BOOL;
    static MoveFileExAHook: unsafe extern "system" fn(PCSTR, PCSTR, u32) -> BOOL;
    static MoveFileExWHook: unsafe extern "system" fn(PCWSTR, PCWSTR, u32) -> BOOL;
    static FindFirstFileAHook: unsafe extern "system" fn(PCSTR, *mut c_void) -> HANDLE;
    static FindFirstFileWHook: unsafe extern "system" fn(PCWSTR, *mut c_void) -> HANDLE;
    static FindFirstFileExAHook: unsafe extern "system" fn(PCSTR, u32, *mut c_void, u32, *mut c_void, u32) -> HANDLE;
    static FindFirstFileExWHook: unsafe extern "system" fn(PCWSTR, u32, *mut c_void, u32, *mut c_void, u32) -> HANDLE;
    static PathFileExistsAHook: unsafe extern "system" fn(PCSTR) -> BOOL;
    static PathFileExistsWHook: unsafe extern "system" fn(PCWSTR) -> BOOL;

    static GetVersionExWHook: unsafe extern "system" fn(*mut OSVERSIONINFOW) -> BOOL;
}

type FnGetFolderPathA = unsafe extern "system" fn(HWND, i32, HANDLE, u32, PSTR) -> HRESULT;
type FnGetFolderPathW = unsafe extern "system" fn(HWND, i32, HANDLE, u32, PWSTR) -> HRESULT;
type FnGetKnownFolderPath = unsafe extern "system" fn(*const GUID, u32, HANDLE, *mut PWSTR) -> HRESULT;
type FnGetPathFromIDListA = unsafe extern "system" fn(*const ITEMIDLIST, PSTR) -> BOOL;
type FnGetPathFromIDListW = unsafe extern "system" fn(*const ITEMIDLIST, PWSTR) -> BOOL;
type FnGetSpecialFolderPathA = unsafe extern "system" fn(HWND, PSTR, i32, BOOL) -> BOOL;
type FnGetSpecialFolderPathW = unsafe extern "system" fn(HWND, PWSTR, i32, BOOL) -> BOOL;
type FnGetEnvironmentVariableA = unsafe extern "system" fn(PCSTR, PSTR, u32) -> u32;
type FnGetEnvironmentVariableW = unsafe extern "system" fn(PCWSTR, PWSTR, u32) -> u32;
type FnExpandEnvironmentStringsA = unsafe extern "system" fn(PCSTR, PSTR, u32) -> u32;
type FnExpandEnvironmentStringsW = unsafe extern "system" fn(PCWSTR, PWSTR, u32) -> u32;
type FnCreateFileA = unsafe extern "system" fn(PCSTR, u32, u32, *const c_void, u32, u32, HANDLE) -> HANDLE;
type FnCreateFileW = unsafe extern "system" fn(PCWSTR, u32, u32, *const c_void, u32, u32, HANDLE) -> HANDLE;
type FnGetFileAttributesA = unsafe extern "system" fn(PCSTR) -> u32;
type FnGetFileAttributesW = unsafe extern "system" fn(PCWSTR) -> u32;
type FnSetFileAttributesA = unsafe extern "system" fn(PCSTR, u32) -> BOOL;
type FnSetFileAttributesW = unsafe extern "system" fn(PCWSTR, u32) -> BOOL;
type FnCreateDirectoryA = unsafe extern "system" fn(PCSTR, *const c_void) -> BOOL;
type FnCreateDirectoryW = unsafe extern "system" fn(PCWSTR, *const c_void) -> BOOL;
type FnRemoveDirectoryA = unsafe extern "system" fn(PCSTR) -> BOOL;
type FnRemoveDirectoryW = unsafe extern "system" fn(PCWSTR) -> BOOL;
type FnDeleteFileA = unsafe extern "system" fn(PCSTR) -> BOOL;
type FnDeleteFileW = unsafe extern "system" fn(PCWSTR) -> BOOL;
type FnCopyFileA = unsafe extern "system" fn(PCSTR, PCSTR, BOOL) -> BOOL;
type FnCopyFileW = unsafe extern "system" fn(PCWSTR, PCWSTR, BOOL) -> BOOL;
type FnMoveFileExA = unsafe extern "system" fn(PCSTR, PCSTR, u32) -> BOOL;
type FnMoveFileExW = unsafe extern "system" fn(PCWSTR, PCWSTR, u32) -> BOOL;
type FnFindFirstFileA = unsafe extern "system" fn(PCSTR, *mut c_void) -> HANDLE;
type FnFindFirstFileW = unsafe extern "system" fn(PCWSTR, *mut c_void) -> HANDLE;
type FnFindFirstFileExA = unsafe extern "system" fn(PCSTR, u32, *mut c_void, u32, *mut c_void, u32) -> HANDLE;
type FnFindFirstFileExW = unsafe extern "system" fn(PCWSTR, u32, *mut c_void, u32, *mut c_void, u32) -> HANDLE;
type FnPathFileExistsA = unsafe extern "system" fn(PCSTR) -> BOOL;
type FnPathFileExistsW = unsafe extern "system" fn(PCWSTR) -> BOOL;
type FnGetVersionExW = unsafe extern "system" fn(*mut OSVERSIONINFOW) -> BOOL;

unsafe fn hook_symbol<T>(dll: &str, symbol: &str) -> Option<T> {
    utils::get_module_symbol_address(dll, symbol).map(|address| unsafe { mem::transmute_copy(&address) })
}

unsafe fn main() -> Result<(), Box<dyn Error>> {
    unsafe {
    if let Some(target) = hook_symbol::<FnGetFolderPathA>("shell32.dll", "SHGetFolderPathA") {
        GetFolderPathAHook.initialize(target, getfolderpatha_detour)?.enable()?;
    }
    if let Some(target) = hook_symbol::<FnGetFolderPathW>("shell32.dll", "SHGetFolderPathW") {
        GetFolderPathWHook.initialize(target, getfolderpathw_detour)?.enable()?;
    }
    if let Some(target) = hook_symbol::<FnGetKnownFolderPath>("shell32.dll", "SHGetKnownFolderPath") {
        GetKnownFolderPathHook.initialize(target, getknownfolderpath_detour)?.enable()?;
    }
    if let Some(target) = hook_symbol::<FnGetPathFromIDListA>("shell32.dll", "SHGetPathFromIDListA") {
        GetPathFromIDListAHook.initialize(target, getpathfromidlista_detour)?.enable()?;
    }
    if let Some(target) = hook_symbol::<FnGetPathFromIDListW>("shell32.dll", "SHGetPathFromIDListW") {
        GetPathFromIDListWHook.initialize(target, getpathfromidlistw_detour)?.enable()?;
    }
    if let Some(target) = hook_symbol::<FnGetSpecialFolderPathA>("shell32.dll", "SHGetSpecialFolderPathA") {
        GetSpecialFolderPathAHook.initialize(target, getspecialfolderpatha_detour)?.enable()?;
    }
    if let Some(target) = hook_symbol::<FnGetSpecialFolderPathW>("shell32.dll", "SHGetSpecialFolderPathW") {
        GetSpecialFolderPathWHook.initialize(target, getspecialfolderpathw_detour)?.enable()?;
    }

    if let Some(target) = hook_symbol::<FnGetEnvironmentVariableA>("kernel32.dll", "GetEnvironmentVariableA") {
        GetEnvironmentVariableAHook.initialize(target, getenvvara_detour)?.enable()?;
    }
    if let Some(target) = hook_symbol::<FnGetEnvironmentVariableW>("kernel32.dll", "GetEnvironmentVariableW") {
        GetEnvironmentVariableWHook.initialize(target, getenvvarw_detour)?.enable()?;
    }
    if let Some(target) = hook_symbol::<FnExpandEnvironmentStringsA>("kernel32.dll", "ExpandEnvironmentStringsA") {
        ExpandEnvironmentStringsAHook.initialize(target, expandenvstringsa_detour)?.enable()?;
    }
    if let Some(target) = hook_symbol::<FnExpandEnvironmentStringsW>("kernel32.dll", "ExpandEnvironmentStringsW") {
        ExpandEnvironmentStringsWHook.initialize(target, expandenvstringsw_detour)?.enable()?;
    }

    if let Some(target) = hook_symbol::<FnCreateFileA>("kernel32.dll", "CreateFileA") {
        CreateFileAHook.initialize(target, createfilea_detour)?.enable()?;
    }
    if let Some(target) = hook_symbol::<FnCreateFileW>("kernel32.dll", "CreateFileW") {
        CreateFileWHook.initialize(target, createfilew_detour)?.enable()?;
    }
    if let Some(target) = hook_symbol::<FnGetFileAttributesA>("kernel32.dll", "GetFileAttributesA") {
        GetFileAttributesAHook.initialize(target, getfileattributesa_detour)?.enable()?;
    }
    if let Some(target) = hook_symbol::<FnGetFileAttributesW>("kernel32.dll", "GetFileAttributesW") {
        GetFileAttributesWHook.initialize(target, getfileattributesw_detour)?.enable()?;
    }
    if let Some(target) = hook_symbol::<FnSetFileAttributesA>("kernel32.dll", "SetFileAttributesA") {
        SetFileAttributesAHook.initialize(target, setfileattributesa_detour)?.enable()?;
    }
    if let Some(target) = hook_symbol::<FnSetFileAttributesW>("kernel32.dll", "SetFileAttributesW") {
        SetFileAttributesWHook.initialize(target, setfileattributesw_detour)?.enable()?;
    }
    if let Some(target) = hook_symbol::<FnCreateDirectoryA>("kernel32.dll", "CreateDirectoryA") {
        CreateDirectoryAHook.initialize(target, createdirectorya_detour)?.enable()?;
    }
    if let Some(target) = hook_symbol::<FnCreateDirectoryW>("kernel32.dll", "CreateDirectoryW") {
        CreateDirectoryWHook.initialize(target, createdirectoryw_detour)?.enable()?;
    }
    if let Some(target) = hook_symbol::<FnRemoveDirectoryA>("kernel32.dll", "RemoveDirectoryA") {
        RemoveDirectoryAHook.initialize(target, removedirectorya_detour)?.enable()?;
    }
    if let Some(target) = hook_symbol::<FnRemoveDirectoryW>("kernel32.dll", "RemoveDirectoryW") {
        RemoveDirectoryWHook.initialize(target, removedirectoryw_detour)?.enable()?;
    }
    if let Some(target) = hook_symbol::<FnDeleteFileA>("kernel32.dll", "DeleteFileA") {
        DeleteFileAHook.initialize(target, deletefilea_detour)?.enable()?;
    }
    if let Some(target) = hook_symbol::<FnDeleteFileW>("kernel32.dll", "DeleteFileW") {
        DeleteFileWHook.initialize(target, deletefilew_detour)?.enable()?;
    }
    if let Some(target) = hook_symbol::<FnCopyFileA>("kernel32.dll", "CopyFileA") {
        CopyFileAHook.initialize(target, copyfilea_detour)?.enable()?;
    }
    if let Some(target) = hook_symbol::<FnCopyFileW>("kernel32.dll", "CopyFileW") {
        CopyFileWHook.initialize(target, copyfilew_detour)?.enable()?;
    }
    if let Some(target) = hook_symbol::<FnMoveFileExA>("kernel32.dll", "MoveFileExA") {
        MoveFileExAHook.initialize(target, movefileexa_detour)?.enable()?;
    }
    if let Some(target) = hook_symbol::<FnMoveFileExW>("kernel32.dll", "MoveFileExW") {
        MoveFileExWHook.initialize(target, movefileexw_detour)?.enable()?;
    }
    if let Some(target) = hook_symbol::<FnFindFirstFileA>("kernel32.dll", "FindFirstFileA") {
        FindFirstFileAHook.initialize(target, findfirstfilea_detour)?.enable()?;
    }
    if let Some(target) = hook_symbol::<FnFindFirstFileW>("kernel32.dll", "FindFirstFileW") {
        FindFirstFileWHook.initialize(target, findfirstfilew_detour)?.enable()?;
    }
    if let Some(target) = hook_symbol::<FnFindFirstFileExA>("kernel32.dll", "FindFirstFileExA") {
        FindFirstFileExAHook.initialize(target, findfirstfileexa_detour)?.enable()?;
    }
    if let Some(target) = hook_symbol::<FnFindFirstFileExW>("kernel32.dll", "FindFirstFileExW") {
        FindFirstFileExWHook.initialize(target, findfirstfileexw_detour)?.enable()?;
    }
    if let Some(target) = hook_symbol::<FnPathFileExistsA>("shlwapi.dll", "PathFileExistsA") {
        PathFileExistsAHook.initialize(target, pathfileexistsa_detour)?.enable()?;
    }
    if let Some(target) = hook_symbol::<FnPathFileExistsW>("shlwapi.dll", "PathFileExistsW") {
        PathFileExistsWHook.initialize(target, pathfileexistsw_detour)?.enable()?;
    }

    if let Some(target) = hook_symbol::<FnGetVersionExW>("kernel32.dll", "GetVersionExW") {
        GetVersionExWHook.initialize(target, getversionexw_detour)?.enable()?;
    }

        Ok(())
    }
}

fn csidl_to_kind(csidl: i32) -> Option<AppDataKind> {
    let clean = csidl & !CSIDL_FLAG_MASK;
    match clean {
        CSIDL_PERSONAL | CSIDL_MYDOCUMENTS => Some(AppDataKind::Documents),
        CSIDL_APPDATA => Some(AppDataKind::Roaming),
        CSIDL_LOCAL_APPDATA => Some(AppDataKind::Local),
        _ => None,
    }
}

fn known_folder_to_kind(rfid: *const GUID) -> Option<AppDataKind> {
    if rfid.is_null() {
        return None;
    }
    unsafe {
        if *rfid == FOLDERID_ROAMING_APPDATA {
            Some(AppDataKind::Roaming)
        } else if *rfid == FOLDERID_LOCAL_APPDATA {
            Some(AppDataKind::Local)
        } else if *rfid == FOLDERID_LOCAL_APPDATA_LOW {
            Some(AppDataKind::LocalLow)
        } else if *rfid == FOLDERID_DOCUMENTS {
            Some(AppDataKind::Documents)
        } else if *rfid == FOLDERID_SAVED_GAMES {
            Some(AppDataKind::SavedGames)
        } else {
            None
        }
    }
}

fn getfolderpatha_detour(hwnd: HWND, csidl: i32, htoken: HANDLE, dwflags: u32, pszpath: PSTR) -> HRESULT {
    if let Some(kind) = csidl_to_kind(csidl) {
        if let Some(path) = utils::portable_folder_a(kind) {
            if (csidl & CSIDL_FLAG_CREATE) != 0 {
                utils::ensure_dir_a(&path);
            }
            unsafe {
                return if utils::write_fixed_a(pszpath, &path, MAX_PATH_CHARS) {
                    S_OK
                } else {
                    E_FAIL
                };
            }
        }
    }
    unsafe { GetFolderPathAHook.call(hwnd, csidl, htoken, dwflags, pszpath) }
}

fn getfolderpathw_detour(hwnd: HWND, csidl: i32, htoken: HANDLE, dwflags: u32, pszpath: PWSTR) -> HRESULT {
    if let Some(kind) = csidl_to_kind(csidl) {
        if let Some(path) = utils::portable_folder_w(kind) {
            if (csidl & CSIDL_FLAG_CREATE) != 0 {
                utils::ensure_dir_w(&path);
            }
            unsafe {
                return if utils::write_fixed_w(pszpath, &path, MAX_PATH_CHARS) {
                    S_OK
                } else {
                    E_FAIL
                };
            }
        }
    }
    unsafe { GetFolderPathWHook.call(hwnd, csidl, htoken, dwflags, pszpath) }
}

fn getknownfolderpath_detour(rfid: *const GUID, dwflags: u32, htoken: HANDLE, ppszpath: *mut PWSTR) -> HRESULT {
    if ppszpath.is_null() {
        return E_INVALIDARG;
    }
    if let Some(kind) = known_folder_to_kind(rfid) {
        if let Some(path) = utils::portable_folder_w(kind) {
            utils::ensure_dir_w(&path);
            let bytes = (path.len() + 1) * std::mem::size_of::<u16>();
            unsafe {
                let mem = CoTaskMemAlloc(bytes);
                if mem.is_null() {
                    return E_OUTOFMEMORY;
                }
                let dst = mem as *mut u16;
                std::ptr::copy_nonoverlapping(path.as_ptr(), dst, path.len());
                *dst.add(path.len()) = 0;
                *ppszpath = PWSTR(dst);
            }
            return S_OK;
        }
    }
    unsafe { GetKnownFolderPathHook.call(rfid, dwflags, htoken, ppszpath) }
}

fn getpathfromidlista_detour(pidl: *const ITEMIDLIST, pszpath: PSTR) -> BOOL {
    unsafe {
        let result = GetPathFromIDListAHook.call(pidl, pszpath);
        if result.0 == 0 {
            return result;
        }
        if let Some(raw) = utils::pcstr_to_bytes(PCSTR(pszpath.as_ptr() as _)) {
            if let Some(redirected) = utils::redirect_appdata_path_a(&raw) {
                return BOOL(utils::write_fixed_a(pszpath, &redirected, MAX_PATH_CHARS) as i32);
            }
        }
        result
    }
}

fn getpathfromidlistw_detour(pidl: *const ITEMIDLIST, pszpath: PWSTR) -> BOOL {
    unsafe {
        let result = GetPathFromIDListWHook.call(pidl, pszpath);
        if result.0 == 0 {
            return result;
        }
        if let Some(wide) = utils::pcwstr_to_wide(PCWSTR(pszpath.as_ptr() as _)) {
            if let Some(redirected) = utils::redirect_appdata_path_w(&wide) {
                return BOOL(utils::write_fixed_w(pszpath, &redirected, MAX_PATH_CHARS) as i32);
            }
        }
        result
    }
}

fn getspecialfolderpatha_detour(hwnd: HWND, pszpath: PSTR, csidl: i32, fcreate: BOOL) -> BOOL {
    if let Some(kind) = csidl_to_kind(csidl) {
        if let Some(path) = utils::portable_folder_a(kind) {
            if fcreate.0 != 0 {
                utils::ensure_dir_a(&path);
            }
            unsafe { return BOOL(utils::write_fixed_a(pszpath, &path, MAX_PATH_CHARS) as i32); }
        }
    }
    unsafe { GetSpecialFolderPathAHook.call(hwnd, pszpath, csidl, fcreate) }
}

fn getspecialfolderpathw_detour(hwnd: HWND, pszpath: PWSTR, csidl: i32, fcreate: BOOL) -> BOOL {
    if let Some(kind) = csidl_to_kind(csidl) {
        if let Some(path) = utils::portable_folder_w(kind) {
            if fcreate.0 != 0 {
                utils::ensure_dir_w(&path);
            }
            unsafe { return BOOL(utils::write_fixed_w(pszpath, &path, MAX_PATH_CHARS) as i32); }
        }
    }
    unsafe { GetSpecialFolderPathWHook.call(hwnd, pszpath, csidl, fcreate) }
}

fn env_name_to_kind_a(name: PCSTR) -> Option<AppDataKind> {
    let bytes = unsafe { utils::pcstr_to_bytes(name)? };
    let upper = bytes.iter().map(|b| b.to_ascii_uppercase()).collect::<Vec<u8>>();
    match upper.as_slice() {
        b"APPDATA" => Some(AppDataKind::Roaming),
        b"LOCALAPPDATA" => Some(AppDataKind::Local),
        _ => None,
    }
}

fn env_name_to_kind_w(name: PCWSTR) -> Option<AppDataKind> {
    let wide = unsafe { utils::pcwstr_to_wide(name)? };
    let s = String::from_utf16_lossy(&wide).to_ascii_uppercase();
    match s.as_str() {
        "APPDATA" => Some(AppDataKind::Roaming),
        "LOCALAPPDATA" => Some(AppDataKind::Local),
        _ => None,
    }
}

fn getenvvara_detour(lpname: PCSTR, lpbuffer: PSTR, nsize: u32) -> u32 {
    if let Some(kind) = env_name_to_kind_a(lpname) {
        if let Some(value) = utils::portable_folder_a(kind) {
            unsafe { return utils::write_sized_a(lpbuffer, nsize, &value, false); }
        }
    }
    unsafe { GetEnvironmentVariableAHook.call(lpname, lpbuffer, nsize) }
}

fn getenvvarw_detour(lpname: PCWSTR, lpbuffer: PWSTR, nsize: u32) -> u32 {
    if let Some(kind) = env_name_to_kind_w(lpname) {
        if let Some(value) = utils::portable_folder_w(kind) {
            unsafe { return utils::write_sized_w(lpbuffer, nsize, &value, false); }
        }
    }
    unsafe { GetEnvironmentVariableWHook.call(lpname, lpbuffer, nsize) }
}

fn expandenvstringsa_detour(lpsrc: PCSTR, lpdst: PSTR, nsize: u32) -> u32 {
    unsafe {
        if let Some(src) = utils::pcstr_to_bytes(lpsrc) {
            if let Some(redirected) = utils::redirect_appdata_path_a(&src) {
                return utils::write_sized_a(lpdst, nsize, &redirected, true);
            }
        }
    }

    let result = unsafe { ExpandEnvironmentStringsAHook.call(lpsrc, lpdst, nsize) };
    if result == 0 || lpdst.as_ptr().is_null() || nsize == 0 || result > nsize {
        return result;
    }
    unsafe {
        if let Some(expanded) = utils::pcstr_to_bytes(PCSTR(lpdst.as_ptr() as _)) {
            if let Some(redirected) = utils::redirect_appdata_path_a(&expanded) {
                return utils::write_sized_a(lpdst, nsize, &redirected, true);
            }
        }
    }
    result
}

fn expandenvstringsw_detour(lpsrc: PCWSTR, lpdst: PWSTR, nsize: u32) -> u32 {
    unsafe {
        if let Some(src) = utils::pcwstr_to_wide(lpsrc) {
            if let Some(redirected) = utils::redirect_appdata_path_w(&src) {
                return utils::write_sized_w(lpdst, nsize, &redirected, true);
            }
        }
    }

    let result = unsafe { ExpandEnvironmentStringsWHook.call(lpsrc, lpdst, nsize) };
    if result == 0 || lpdst.as_ptr().is_null() || nsize == 0 || result > nsize {
        return result;
    }
    unsafe {
        if let Some(expanded) = utils::pcwstr_to_wide(PCWSTR(lpdst.as_ptr() as _)) {
            if let Some(redirected) = utils::redirect_appdata_path_w(&expanded) {
                return utils::write_sized_w(lpdst, nsize, &redirected, true);
            }
        }
    }
    result
}

fn redirected_pcstr(path: PCSTR) -> Option<(Vec<u8>, std::ffi::CString)> {
    let raw = unsafe { utils::pcstr_to_bytes(path)? };
    let redirected = utils::redirect_appdata_path_a(&raw)?;
    let cstring = utils::bytes_with_nul(redirected.clone())?;
    Some((redirected, cstring))
}

fn redirected_pcwstr(path: PCWSTR) -> Option<(Vec<u16>, Vec<u16>)> {
    let raw = unsafe { utils::pcwstr_to_wide(path)? };
    let redirected = utils::redirect_appdata_path_w(&raw)?;
    let nul = utils::wide_with_nul(redirected.clone());
    Some((redirected, nul))
}

fn createfilea_detour(lpfilename: PCSTR, desiredaccess: u32, sharemode: u32, securityattributes: *const c_void, creationdisposition: u32, flagsandattributes: u32, templatefile: HANDLE) -> HANDLE {
    if let Some((redirected, cstr)) = redirected_pcstr(lpfilename) {
        utils::ensure_parent_dir_a(&redirected);
        unsafe {
            return CreateFileAHook.call(PCSTR(cstr.as_ptr() as _), desiredaccess, sharemode, securityattributes, creationdisposition, flagsandattributes, templatefile);
        }
    }
    unsafe { CreateFileAHook.call(lpfilename, desiredaccess, sharemode, securityattributes, creationdisposition, flagsandattributes, templatefile) }
}

fn createfilew_detour(lpfilename: PCWSTR, desiredaccess: u32, sharemode: u32, securityattributes: *const c_void, creationdisposition: u32, flagsandattributes: u32, templatefile: HANDLE) -> HANDLE {
    if let Some((redirected, wide)) = redirected_pcwstr(lpfilename) {
        utils::ensure_parent_dir_w(&redirected);
        unsafe {
            return CreateFileWHook.call(PCWSTR(wide.as_ptr()), desiredaccess, sharemode, securityattributes, creationdisposition, flagsandattributes, templatefile);
        }
    }
    unsafe { CreateFileWHook.call(lpfilename, desiredaccess, sharemode, securityattributes, creationdisposition, flagsandattributes, templatefile) }
}

fn getfileattributesa_detour(lpfilename: PCSTR) -> u32 {
    if let Some((_redirected, cstr)) = redirected_pcstr(lpfilename) {
        unsafe { return GetFileAttributesAHook.call(PCSTR(cstr.as_ptr() as _)); }
    }
    unsafe { GetFileAttributesAHook.call(lpfilename) }
}

fn getfileattributesw_detour(lpfilename: PCWSTR) -> u32 {
    if let Some((_redirected, wide)) = redirected_pcwstr(lpfilename) {
        unsafe { return GetFileAttributesWHook.call(PCWSTR(wide.as_ptr())); }
    }
    unsafe { GetFileAttributesWHook.call(lpfilename) }
}

fn setfileattributesa_detour(lpfilename: PCSTR, fileattributes: u32) -> BOOL {
    if let Some((_redirected, cstr)) = redirected_pcstr(lpfilename) {
        unsafe { return SetFileAttributesAHook.call(PCSTR(cstr.as_ptr() as _), fileattributes); }
    }
    unsafe { SetFileAttributesAHook.call(lpfilename, fileattributes) }
}

fn setfileattributesw_detour(lpfilename: PCWSTR, fileattributes: u32) -> BOOL {
    if let Some((_redirected, wide)) = redirected_pcwstr(lpfilename) {
        unsafe { return SetFileAttributesWHook.call(PCWSTR(wide.as_ptr()), fileattributes); }
    }
    unsafe { SetFileAttributesWHook.call(lpfilename, fileattributes) }
}

fn createdirectorya_detour(lppathname: PCSTR, lpsecurityattributes: *const c_void) -> BOOL {
    if let Some((redirected, cstr)) = redirected_pcstr(lppathname) {
        utils::ensure_parent_dir_a(&redirected);
        unsafe { return CreateDirectoryAHook.call(PCSTR(cstr.as_ptr() as _), lpsecurityattributes); }
    }
    unsafe { CreateDirectoryAHook.call(lppathname, lpsecurityattributes) }
}

fn createdirectoryw_detour(lppathname: PCWSTR, lpsecurityattributes: *const c_void) -> BOOL {
    if let Some((redirected, wide)) = redirected_pcwstr(lppathname) {
        utils::ensure_parent_dir_w(&redirected);
        unsafe { return CreateDirectoryWHook.call(PCWSTR(wide.as_ptr()), lpsecurityattributes); }
    }
    unsafe { CreateDirectoryWHook.call(lppathname, lpsecurityattributes) }
}

fn removedirectorya_detour(lppathname: PCSTR) -> BOOL {
    if let Some((_redirected, cstr)) = redirected_pcstr(lppathname) {
        unsafe { return RemoveDirectoryAHook.call(PCSTR(cstr.as_ptr() as _)); }
    }
    unsafe { RemoveDirectoryAHook.call(lppathname) }
}

fn removedirectoryw_detour(lppathname: PCWSTR) -> BOOL {
    if let Some((_redirected, wide)) = redirected_pcwstr(lppathname) {
        unsafe { return RemoveDirectoryWHook.call(PCWSTR(wide.as_ptr())); }
    }
    unsafe { RemoveDirectoryWHook.call(lppathname) }
}

fn deletefilea_detour(lpfilename: PCSTR) -> BOOL {
    if let Some((_redirected, cstr)) = redirected_pcstr(lpfilename) {
        unsafe { return DeleteFileAHook.call(PCSTR(cstr.as_ptr() as _)); }
    }
    unsafe { DeleteFileAHook.call(lpfilename) }
}

fn deletefilew_detour(lpfilename: PCWSTR) -> BOOL {
    if let Some((_redirected, wide)) = redirected_pcwstr(lpfilename) {
        unsafe { return DeleteFileWHook.call(PCWSTR(wide.as_ptr())); }
    }
    unsafe { DeleteFileWHook.call(lpfilename) }
}

fn copyfilea_detour(lpexistingfilename: PCSTR, lpnewfilename: PCSTR, bfailifexists: BOOL) -> BOOL {
    let old = redirected_pcstr(lpexistingfilename);
    let new = redirected_pcstr(lpnewfilename);
    let old_ptr = old.as_ref().map(|(_, c)| PCSTR(c.as_ptr() as _)).unwrap_or(lpexistingfilename);
    let new_ptr = new.as_ref().map(|(p, c)| {
        utils::ensure_parent_dir_a(p);
        PCSTR(c.as_ptr() as _)
    }).unwrap_or(lpnewfilename);
    unsafe { CopyFileAHook.call(old_ptr, new_ptr, bfailifexists) }
}

fn copyfilew_detour(lpexistingfilename: PCWSTR, lpnewfilename: PCWSTR, bfailifexists: BOOL) -> BOOL {
    let old = redirected_pcwstr(lpexistingfilename);
    let new = redirected_pcwstr(lpnewfilename);
    let old_ptr = old.as_ref().map(|(_, w)| PCWSTR(w.as_ptr())).unwrap_or(lpexistingfilename);
    let new_ptr = new.as_ref().map(|(p, w)| {
        utils::ensure_parent_dir_w(p);
        PCWSTR(w.as_ptr())
    }).unwrap_or(lpnewfilename);
    unsafe { CopyFileWHook.call(old_ptr, new_ptr, bfailifexists) }
}

fn movefileexa_detour(lpexistingfilename: PCSTR, lpnewfilename: PCSTR, dwflags: u32) -> BOOL {
    let old = redirected_pcstr(lpexistingfilename);
    let new = redirected_pcstr(lpnewfilename);
    let old_ptr = old.as_ref().map(|(_, c)| PCSTR(c.as_ptr() as _)).unwrap_or(lpexistingfilename);
    let new_ptr = new.as_ref().map(|(p, c)| {
        utils::ensure_parent_dir_a(p);
        PCSTR(c.as_ptr() as _)
    }).unwrap_or(lpnewfilename);
    unsafe { MoveFileExAHook.call(old_ptr, new_ptr, dwflags) }
}

fn movefileexw_detour(lpexistingfilename: PCWSTR, lpnewfilename: PCWSTR, dwflags: u32) -> BOOL {
    let old = redirected_pcwstr(lpexistingfilename);
    let new = redirected_pcwstr(lpnewfilename);
    let old_ptr = old.as_ref().map(|(_, w)| PCWSTR(w.as_ptr())).unwrap_or(lpexistingfilename);
    let new_ptr = new.as_ref().map(|(p, w)| {
        utils::ensure_parent_dir_w(p);
        PCWSTR(w.as_ptr())
    }).unwrap_or(lpnewfilename);
    unsafe { MoveFileExWHook.call(old_ptr, new_ptr, dwflags) }
}

fn findfirstfilea_detour(lpfilename: PCSTR, lpfindfiledata: *mut c_void) -> HANDLE {
    if let Some((_redirected, cstr)) = redirected_pcstr(lpfilename) {
        unsafe { return FindFirstFileAHook.call(PCSTR(cstr.as_ptr() as _), lpfindfiledata); }
    }
    unsafe { FindFirstFileAHook.call(lpfilename, lpfindfiledata) }
}

fn findfirstfilew_detour(lpfilename: PCWSTR, lpfindfiledata: *mut c_void) -> HANDLE {
    if let Some((_redirected, wide)) = redirected_pcwstr(lpfilename) {
        unsafe { return FindFirstFileWHook.call(PCWSTR(wide.as_ptr()), lpfindfiledata); }
    }
    unsafe { FindFirstFileWHook.call(lpfilename, lpfindfiledata) }
}

fn findfirstfileexa_detour(lpfilename: PCSTR, finfolevelid: u32, lpfindfiledata: *mut c_void, fsearchop: u32, lpsearchfilter: *mut c_void, dwadditionalflags: u32) -> HANDLE {
    if let Some((_redirected, cstr)) = redirected_pcstr(lpfilename) {
        unsafe { return FindFirstFileExAHook.call(PCSTR(cstr.as_ptr() as _), finfolevelid, lpfindfiledata, fsearchop, lpsearchfilter, dwadditionalflags); }
    }
    unsafe { FindFirstFileExAHook.call(lpfilename, finfolevelid, lpfindfiledata, fsearchop, lpsearchfilter, dwadditionalflags) }
}

fn findfirstfileexw_detour(lpfilename: PCWSTR, finfolevelid: u32, lpfindfiledata: *mut c_void, fsearchop: u32, lpsearchfilter: *mut c_void, dwadditionalflags: u32) -> HANDLE {
    if let Some((_redirected, wide)) = redirected_pcwstr(lpfilename) {
        unsafe { return FindFirstFileExWHook.call(PCWSTR(wide.as_ptr()), finfolevelid, lpfindfiledata, fsearchop, lpsearchfilter, dwadditionalflags); }
    }
    unsafe { FindFirstFileExWHook.call(lpfilename, finfolevelid, lpfindfiledata, fsearchop, lpsearchfilter, dwadditionalflags) }
}

fn pathfileexistsa_detour(pszpath: PCSTR) -> BOOL {
    if let Some((_redirected, cstr)) = redirected_pcstr(pszpath) {
        unsafe { return PathFileExistsAHook.call(PCSTR(cstr.as_ptr() as _)); }
    }
    unsafe { PathFileExistsAHook.call(pszpath) }
}

fn pathfileexistsw_detour(pszpath: PCWSTR) -> BOOL {
    if let Some((_redirected, wide)) = redirected_pcwstr(pszpath) {
        unsafe { return PathFileExistsWHook.call(PCWSTR(wide.as_ptr())); }
    }
    unsafe { PathFileExistsWHook.call(pszpath) }
}

fn getversionexw_detour(lpversioninformation: *mut OSVERSIONINFOW) -> BOOL {
    unsafe {
        let result = GetVersionExWHook.call(lpversioninformation);
        let mut filename_buf = [0u16; 260];
        GetModuleFileNameW(Some(HMODULE::default()), &mut filename_buf);
        let filename = utils::u16_array_to_string(&filename_buf).unwrap_or_default();
        if filename.contains("AdvHD") && !lpversioninformation.is_null() {
            (*lpversioninformation).dwMajorVersion = 0u32;
        }
        result
    }
}

#[unsafe(no_mangle)]
unsafe extern "system" fn DllMain(_hinst: HANDLE, reason: u32, _reserved: *mut c_void) -> BOOL {
    match reason {
        DLL_PROCESS_ATTACH => {
            let _ = unsafe { main() };
        }
        DLL_PROCESS_DETACH => {}
        DLL_THREAD_ATTACH => {}
        DLL_THREAD_DETACH => {}
        _ => {}
    };
    BOOL::from(true)
}
