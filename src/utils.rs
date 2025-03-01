use std::{ffi::CString, iter};
use windows::core::{PCSTR, PCWSTR};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};

pub fn get_module_symbol_address(module: &str, symbol: &str) -> Option<usize> {
  let module = module
    .encode_utf16()
    .chain(iter::once(0))
    .collect::<Vec<u16>>();
  let symbol = CString::new(symbol).unwrap();
  unsafe {
    let handle = GetModuleHandleW(PCWSTR(module.as_ptr() as _)).unwrap();
    match GetProcAddress(handle, PCSTR(symbol.as_ptr() as _)) {
      Some(func) => Some(func as usize),
      None => None,
    }
  }
}