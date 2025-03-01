use retour::static_detour;
use std::error::Error;
use std::os::raw::c_void;
use std::mem;
use windows::core::{PSTR, PWSTR, HRESULT};
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::Foundation::{BOOL, HANDLE, HWND};
use windows::Win32::System::SystemServices::{
  DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH, DLL_THREAD_ATTACH, DLL_THREAD_DETACH,
};
use windows_sys::Win32::System::Environment::{GetCurrentDirectoryA, GetCurrentDirectoryW};
mod utils;

// https://docs.rs/windows-sys/0.52.0/windows_sys/
static_detour! {
  static SHGetFolderPathAHook: unsafe extern "system" fn(HWND, i32, HANDLE, u32, PSTR) -> HRESULT;
  static SHGetFolderPathWHook: unsafe extern "system" fn(HWND, i32, HANDLE, u32, PWSTR) -> HRESULT;
  static SHGetPathFromIDListAHook: unsafe extern "system" fn(*const ITEMIDLIST, PSTR) -> BOOL;
  static SHGetPathFromIDListWHook: unsafe extern "system" fn(*const ITEMIDLIST, PWSTR) -> BOOL;
}

type FnSHGetFolderPathA = unsafe extern "system" fn(HWND, i32, HANDLE, u32, PSTR) -> HRESULT;
type FnSHGetFolderPathW = unsafe extern "system" fn(HWND, i32, HANDLE, u32, PWSTR) -> HRESULT;
type FnSHGetPathFromIDListA = unsafe extern "system" fn(*const ITEMIDLIST, PSTR) -> BOOL;
type FnSHGetPathFromIDListW = unsafe extern "system" fn(*const ITEMIDLIST, PWSTR) -> BOOL;

unsafe fn main() -> Result<(), Box<dyn Error>> {
	
  let gfpa_address = utils::get_module_symbol_address("shell32.dll", "SHGetFolderPathA")
    .expect("could not find 'SHGetFolderPathA' address");
  let gfpa_target: FnSHGetFolderPathA = mem::transmute(gfpa_address);

  SHGetFolderPathAHook
    .initialize(gfpa_target, shgetfolderpatha_detour)?
    .enable()?;
  
  let gfpw_address = utils::get_module_symbol_address("shell32.dll", "SHGetFolderPathW")
    .expect("could not find 'SHGetFolderPathW' address");
  let gfpw_target: FnSHGetFolderPathW = mem::transmute(gfpw_address);

  SHGetFolderPathWHook
    .initialize(gfpw_target, shgetfolderpathw_detour)?
    .enable()?;
  
  let gpfila_address = utils::get_module_symbol_address("shell32.dll", "SHGetPathFromIDListA")
    .expect("could not find 'SHGetPathFromIDListA' address");
  let gpfila_target: FnSHGetPathFromIDListA = mem::transmute(gpfila_address);

  SHGetPathFromIDListAHook
    .initialize(gpfila_target, shgetpathfromidlista_detour)?
    .enable()?;
  
  let gpfilw_address = utils::get_module_symbol_address("shell32.dll", "SHGetPathFromIDListW")
    .expect("could not find 'SHGetPathFromIDListW' address");
  let gpfilw_target: FnSHGetPathFromIDListW = mem::transmute(gpfilw_address);

  SHGetPathFromIDListWHook
    .initialize(gpfilw_target, shgetpathfromidlistw_detour)?
    .enable()?;
  Ok(())
}

fn shgetfolderpatha_detour(hwnd: HWND, csidl: i32, htoken: HANDLE, dwflags: u32, pszpath: PSTR) -> HRESULT {
  let max_len: u32 = 260;
  unsafe { HRESULT::from_win32(GetCurrentDirectoryA(max_len, pszpath.as_ptr())) }
}

fn shgetfolderpathw_detour(hwnd: HWND, csidl: i32, htoken: HANDLE, dwflags: u32, pszpath: PWSTR) -> HRESULT {
  let max_len: u32 = 260;
  unsafe { HRESULT::from_win32(GetCurrentDirectoryW(max_len, pszpath.as_ptr())) }
}

fn shgetpathfromidlista_detour(pidl: *const ITEMIDLIST, pszpath: PSTR) -> BOOL {
  unsafe { 
	  let result: BOOL = SHGetPathFromIDListAHook.call(pidl, pszpath);
	  if pszpath.to_string().expect("").to_lowercase().starts_with("c") {
		let max_len: u32 = 260;
		match GetCurrentDirectoryA(max_len, pszpath.as_ptr()) {
				0 => BOOL(1),
				_ => BOOL(0),
			}
	  }
	  else{
		  result
	  }
  }
}

fn shgetpathfromidlistw_detour(pidl: *const ITEMIDLIST, pszpath: PWSTR) -> BOOL {
  unsafe { 
	  let result: BOOL = SHGetPathFromIDListWHook.call(pidl, pszpath);
	  if pszpath.to_string().expect("").to_lowercase().starts_with("c") {
		let max_len: u32 = 260;
		match GetCurrentDirectoryW(max_len, pszpath.as_ptr()) {
				0 => BOOL(1),
				_ => BOOL(0),
			}
	  }
	  else{
		  result
	  }
  }
}

#[unsafe(no_mangle)]
unsafe extern "system" fn DllMain(_hinst: HANDLE, reason: u32, _reserved: *mut c_void) -> BOOL {
  match reason {
    DLL_PROCESS_ATTACH => { unsafe { main().unwrap() } },
    DLL_PROCESS_DETACH => {},
    DLL_THREAD_ATTACH => {},
    DLL_THREAD_DETACH => {},
    _ => {},
  };
  return BOOL::from(true);
}
