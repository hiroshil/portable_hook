use retour::static_detour;
use std::error::Error;
use std::os::raw::c_void;
use std::mem;
use windows::core::{PSTR, PWSTR, HRESULT, BOOL};
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::Foundation::{HANDLE, HWND, HMODULE};
use windows::Win32::System::Environment::{GetCurrentDirectoryA, GetCurrentDirectoryW};
use windows::Win32::System::SystemInformation::OSVERSIONINFOW;
use windows::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows::Win32::System::SystemServices::{
  DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH, DLL_THREAD_ATTACH, DLL_THREAD_DETACH,
};
mod utils;

// https://docs.rs/windows-sys/0.52.0/windows_sys/
static_detour! {
  static GetFolderPathAHook: unsafe extern "system" fn(HWND, i32, HANDLE, u32, PSTR) -> HRESULT;
  static GetFolderPathWHook: unsafe extern "system" fn(HWND, i32, HANDLE, u32, PWSTR) -> HRESULT;
  static GetPathFromIDListAHook: unsafe extern "system" fn(*const ITEMIDLIST, PSTR) -> BOOL;
  static GetPathFromIDListWHook: unsafe extern "system" fn(*const ITEMIDLIST, PWSTR) -> BOOL;
  static GetSpecialFolderPathAHook: unsafe extern "system" fn(HWND, PSTR, i32, BOOL) -> BOOL;
  static GetSpecialFolderPathWHook: unsafe extern "system" fn(HWND, PWSTR, i32, BOOL) -> BOOL;
  static GetVersionExWHook: unsafe extern "system" fn(*mut OSVERSIONINFOW) -> BOOL;
}

type FnGetFolderPathA = unsafe extern "system" fn(HWND, i32, HANDLE, u32, PSTR) -> HRESULT;
type FnGetFolderPathW = unsafe extern "system" fn(HWND, i32, HANDLE, u32, PWSTR) -> HRESULT;
type FnGetPathFromIDListA = unsafe extern "system" fn(*const ITEMIDLIST, PSTR) -> BOOL;
type FnGetPathFromIDListW = unsafe extern "system" fn(*const ITEMIDLIST, PWSTR) -> BOOL;
type FnGetSpecialFolderPathA = unsafe extern "system" fn(HWND, PSTR, i32, BOOL) -> BOOL;
type FnGetSpecialFolderPathW = unsafe extern "system" fn(HWND, PWSTR, i32, BOOL) -> BOOL;
type FnGetVersionExW = unsafe extern "system" fn(*mut OSVERSIONINFOW) -> BOOL;

unsafe fn main() -> Result<(), Box<dyn Error>> {
	
  let gfpa_address = utils::get_module_symbol_address("shell32.dll", "SHGetFolderPathA")
    .expect("could not find 'SHGetFolderPathA' address");
  let gfpa_target: FnGetFolderPathA = mem::transmute(gfpa_address);

  GetFolderPathAHook
    .initialize(gfpa_target, getfolderpatha_detour)?
    .enable()?;
  
  let gfpw_address = utils::get_module_symbol_address("shell32.dll", "SHGetFolderPathW")
    .expect("could not find 'SHGetFolderPathW' address");
  let gfpw_target: FnGetFolderPathW = mem::transmute(gfpw_address);

  GetFolderPathWHook
    .initialize(gfpw_target, getfolderpathw_detour)?
    .enable()?;
  
  let gpfila_address = utils::get_module_symbol_address("shell32.dll", "SHGetPathFromIDListA")
    .expect("could not find 'SHGetPathFromIDListA' address");
  let gpfila_target: FnGetPathFromIDListA = mem::transmute(gpfila_address);

  GetPathFromIDListAHook
    .initialize(gpfila_target, getpathfromidlista_detour)?
    .enable()?;
  
  let gpfilw_address = utils::get_module_symbol_address("shell32.dll", "SHGetPathFromIDListW")
    .expect("could not find 'SHGetPathFromIDListW' address");
  let gpfilw_target: FnGetPathFromIDListW = mem::transmute(gpfilw_address);

  GetPathFromIDListWHook
    .initialize(gpfilw_target, getpathfromidlistw_detour)?
    .enable()?;
  
  let gsfpa_address = utils::get_module_symbol_address("shell32.dll", "SHGetSpecialFolderPathA")
    .expect("could not find 'SHGetSpecialFolderPathA' address");
  let gsfpa_target: FnGetSpecialFolderPathA = mem::transmute(gsfpa_address);

  GetSpecialFolderPathAHook
    .initialize(gsfpa_target, getspecialfolderpatha_detour)?
    .enable()?;
  
  let gsfpw_address = utils::get_module_symbol_address("shell32.dll", "SHGetSpecialFolderPathW")
    .expect("could not find 'SHGetSpecialFolderPathW' address");
  let gsfpw_target: FnGetSpecialFolderPathW = mem::transmute(gsfpw_address);

  GetSpecialFolderPathWHook
    .initialize(gsfpw_target, getspecialfolderpathw_detour)?
    .enable()?;
  
  let gvew_address = utils::get_module_symbol_address("kernel32.dll", "GetVersionExW")
    .expect("could not find 'GetVersionExW' address");
  let gvew_target: FnGetVersionExW = mem::transmute(gvew_address);

  GetVersionExWHook
    .initialize(gvew_target, getversionexw_detour)?
    .enable()?;
  Ok(())
}

fn getfolderpatha_detour(hwnd: HWND, csidl: i32, htoken: HANDLE, dwflags: u32, pszpath: PSTR) -> HRESULT {
  unsafe { 
		let mut current_path = [0u8; 260];
		let result: u32 = GetCurrentDirectoryA(Some(&mut current_path));
		std::ptr::copy_nonoverlapping(current_path.as_ptr(), pszpath.as_ptr(), 260);
		HRESULT::from_win32(result) 
	}
}

fn getfolderpathw_detour(hwnd: HWND, csidl: i32, htoken: HANDLE, dwflags: u32, pszpath: PWSTR) -> HRESULT {
  unsafe { 
		let mut current_path = [0u16; 260];
		let result: u32 = GetCurrentDirectoryW(Some(&mut current_path));
		std::ptr::copy_nonoverlapping(current_path.as_ptr(), pszpath.as_ptr(), 260);
		HRESULT::from_win32(result) 
	}
}

fn getpathfromidlista_detour(pidl: *const ITEMIDLIST, pszpath: PSTR) -> BOOL {
  unsafe { 
		let result: BOOL = GetPathFromIDListAHook.call(pidl, pszpath);
		let raw_path= utils::u8_array_to_string(pszpath.as_bytes()).unwrap();
		if raw_path.to_lowercase().starts_with("c") {
			let mut current_path = [0u8; 260];
			match GetCurrentDirectoryA(Some(&mut current_path)) {
					0u32 => BOOL(0),
					_ => {
						std::ptr::copy_nonoverlapping(current_path.as_ptr(), pszpath.as_ptr(), 260);
						BOOL(1)
					},
				}
		}
		else{
			  result
		} 
	}
}

fn getpathfromidlistw_detour(pidl: *const ITEMIDLIST, pszpath: PWSTR) -> BOOL {
  unsafe { 
		let result: BOOL = GetPathFromIDListWHook.call(pidl, pszpath);
		let raw_path= utils::u16_array_to_string(pszpath.as_wide()).unwrap();
		if raw_path.to_lowercase().starts_with("c") {
			let mut current_path = [0u16; 260];
			match GetCurrentDirectoryW(Some(&mut current_path)) {
					0u32 => BOOL(0),
					_ => {
						std::ptr::copy_nonoverlapping(current_path.as_ptr(), pszpath.as_ptr(), 260);
						BOOL(1)
					},
				}
		}
		else{
			  result
		} 
	}
}

fn getspecialfolderpatha_detour(hwnd: HWND, pszpath: PSTR, csidl: i32, fcreate: BOOL) -> BOOL {
  unsafe { 
		let mut current_path = [0u8; 260];
		match GetCurrentDirectoryA(Some(&mut current_path)) {
				0u32 => BOOL(0),
				_ => {
					std::ptr::copy_nonoverlapping(current_path.as_ptr(), pszpath.as_ptr(), 260);
					BOOL(1)
				},
			}
	}
}

fn getspecialfolderpathw_detour(hwnd: HWND, pszpath: PWSTR, csidl: i32, fcreate: BOOL) -> BOOL {
  unsafe { 
		let mut current_path = [0u16; 260];
		match GetCurrentDirectoryW(Some(&mut current_path)) {
				0u32 => BOOL(0),
				_ => {
					std::ptr::copy_nonoverlapping(current_path.as_ptr(), pszpath.as_ptr(), 260);
					BOOL(1)
				},
			}
	}
}

fn getversionexw_detour(lpversioninformation: *mut OSVERSIONINFOW) -> BOOL {
  unsafe { 
		let result: BOOL = GetVersionExWHook.call(lpversioninformation);
		let mut filename_buf = [0u16; 260];
		GetModuleFileNameW(Some(HMODULE::default()), &mut filename_buf);
		let filename = utils::u16_array_to_string(&filename_buf).unwrap();
		if filename.contains("AdvHD") {
			(*lpversioninformation).dwMajorVersion = 0u32;
		}
		result
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
