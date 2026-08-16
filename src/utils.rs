use std::{ffi::CStr, ffi::CString, path::PathBuf};
use windows::core::{PCSTR, PCWSTR, PSTR, PWSTR};
use windows::Win32::System::Environment::{GetCurrentDirectoryA, GetCurrentDirectoryW};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::core::{PCSTR as WinPCSTR, PCWSTR as WinPCWSTR};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppDataKind {
    Roaming,
    Local,
    LocalLow,
    Documents,
    SavedGames,
}

pub fn get_module_symbol_address(module: &str, symbol: &str) -> Option<usize> {
    let module = module
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<u16>>();
    let symbol = CString::new(symbol).unwrap();
    unsafe {
        let handle = GetModuleHandleW(WinPCWSTR(module.as_ptr())).ok()?;
        match GetProcAddress(handle, WinPCSTR(symbol.as_ptr() as _)) {
            Some(func) => Some(func as usize),
            None => None,
        }
    }
}

pub unsafe fn pcstr_to_bytes(input: PCSTR) -> Option<Vec<u8>> {
    let ptr = input.as_ptr();
    if ptr.is_null() {
        return None;
    }
    Some(unsafe { CStr::from_ptr(ptr as *const i8) }.to_bytes().to_vec())
}


pub unsafe fn pcwstr_to_wide(input: PCWSTR) -> Option<Vec<u16>> {
    let ptr = input.as_ptr();
    if ptr.is_null() {
        return None;
    }
    let mut len = 0usize;
    unsafe {
        while *ptr.add(len) != 0 {
            len += 1;
        }
        Some(std::slice::from_raw_parts(ptr, len).to_vec())
    }
}


pub fn current_dir_a() -> Option<Vec<u8>> {
    unsafe {
        let mut buffer = [0u8; 32768];
        let len = GetCurrentDirectoryA(Some(&mut buffer));
        if len == 0 || len as usize >= buffer.len() {
            return None;
        }
        Some(buffer[..len as usize].to_vec())
    }
}

pub fn current_dir_w() -> Option<Vec<u16>> {
    unsafe {
        let mut buffer = [0u16; 32768];
        let len = GetCurrentDirectoryW(Some(&mut buffer));
        if len == 0 || len as usize >= buffer.len() {
            return None;
        }
        Some(buffer[..len as usize].to_vec())
    }
}

fn push_ascii_path_a(out: &mut Vec<u8>, suffix: &str) {
    if !out.ends_with(b"\\") && !out.ends_with(b"/") {
        out.push(b'\\');
    }
    out.extend_from_slice(suffix.as_bytes());
}

fn push_ascii_path_w(out: &mut Vec<u16>, suffix: &str) {
    if !out.ends_with(&[b'\\' as u16]) && !out.ends_with(&[b'/' as u16]) {
        out.push(b'\\' as u16);
    }
    out.extend(suffix.encode_utf16());
}

pub fn portable_folder_a(kind: AppDataKind) -> Option<Vec<u8>> {
    let mut out = current_dir_a()?;
    match kind {
        AppDataKind::Roaming => {
            push_ascii_path_a(&mut out, "AppData");
            push_ascii_path_a(&mut out, "Roaming");
        }
        AppDataKind::Local => {
            push_ascii_path_a(&mut out, "AppData");
            push_ascii_path_a(&mut out, "Local");
        }
        AppDataKind::LocalLow => {
            push_ascii_path_a(&mut out, "AppData");
            push_ascii_path_a(&mut out, "LocalLow");
        }
        AppDataKind::Documents => push_ascii_path_a(&mut out, "Documents"),
        AppDataKind::SavedGames => push_ascii_path_a(&mut out, "Saved Games"),
    }
    Some(out)
}

pub fn portable_folder_w(kind: AppDataKind) -> Option<Vec<u16>> {
    let mut out = current_dir_w()?;
    match kind {
        AppDataKind::Roaming => {
            push_ascii_path_w(&mut out, "AppData");
            push_ascii_path_w(&mut out, "Roaming");
        }
        AppDataKind::Local => {
            push_ascii_path_w(&mut out, "AppData");
            push_ascii_path_w(&mut out, "Local");
        }
        AppDataKind::LocalLow => {
            push_ascii_path_w(&mut out, "AppData");
            push_ascii_path_w(&mut out, "LocalLow");
        }
        AppDataKind::Documents => push_ascii_path_w(&mut out, "Documents"),
        AppDataKind::SavedGames => push_ascii_path_w(&mut out, "Saved Games"),
    }
    Some(out)
}


fn normalize_ascii_byte(b: u8) -> u8 {
    let sep = if b == b'/' { b'\\' } else { b };
    if sep >= b'A' && sep <= b'Z' {
        sep + 32
    } else {
        sep
    }
}

fn normalized_bytes(input: &[u8]) -> Vec<u8> {
    input.iter().map(|&b| normalize_ascii_byte(b)).collect()
}

fn normalize_ascii_wide(ch: u16) -> u16 {
    let sep = if ch == b'/' as u16 { b'\\' as u16 } else { ch };
    if sep >= b'A' as u16 && sep <= b'Z' as u16 {
        sep + 32
    } else {
        sep
    }
}

fn normalized_wide(input: &[u16]) -> Vec<u16> {
    input.iter().map(|&ch| normalize_ascii_wide(ch)).collect()
}

fn boundary_a(norm: &[u8], index: usize) -> bool {
    index >= norm.len() || norm[index] == b'\\'
}

fn boundary_w(norm: &[u16], index: usize) -> bool {
    index >= norm.len() || norm[index] == b'\\' as u16
}

fn has_left_boundary_a(norm: &[u8], start: usize) -> bool {
    start == 0 || norm[start - 1] == b'\\'
}

fn has_left_boundary_w(norm: &[u16], start: usize) -> bool {
    start == 0 || norm[start - 1] == b'\\' as u16
}

fn find_token_a(norm: &[u8], token: &[u8]) -> Option<usize> {
    if norm.len() < token.len() {
        return None;
    }
    norm.windows(token.len()).position(|w| w == token)
}

fn find_token_w(norm: &[u16], token: &[u16]) -> Option<usize> {
    if norm.len() < token.len() {
        return None;
    }
    norm.windows(token.len()).position(|w| w == token)
}

fn append_suffix_a(mut base: Vec<u8>, suffix: &[u8]) -> Vec<u8> {
    if suffix.is_empty() {
        return base;
    }
    let suffix_starts_with_sep = suffix[0] == b'\\' || suffix[0] == b'/';
    let base_ends_with_sep = base.ends_with(b"\\") || base.ends_with(b"/");
    if !base_ends_with_sep && !suffix_starts_with_sep {
        base.push(b'\\');
    }
    if base_ends_with_sep && suffix_starts_with_sep {
        base.extend_from_slice(&suffix[1..]);
    } else {
        base.extend_from_slice(suffix);
    }
    base
}

fn append_suffix_w(mut base: Vec<u16>, suffix: &[u16]) -> Vec<u16> {
    if suffix.is_empty() {
        return base;
    }
    let suffix_starts_with_sep = suffix[0] == b'\\' as u16 || suffix[0] == b'/' as u16;
    let base_ends_with_sep = base.ends_with(&[b'\\' as u16]) || base.ends_with(&[b'/' as u16]);
    if !base_ends_with_sep && !suffix_starts_with_sep {
        base.push(b'\\' as u16);
    }
    if base_ends_with_sep && suffix_starts_with_sep {
        base.extend_from_slice(&suffix[1..]);
    } else {
        base.extend_from_slice(suffix);
    }
    base
}

pub fn redirect_appdata_path_a(path: &[u8]) -> Option<Vec<u8>> {
    let norm = normalized_bytes(path);

    let env_patterns: [(&[u8], AppDataKind); 6] = [
        (b"%appdata%", AppDataKind::Roaming),
        (b"%localappdata%", AppDataKind::Local),
        (b"%userprofile%\\appdata\\locallow", AppDataKind::LocalLow),
        (b"%userprofile%\\documents", AppDataKind::Documents),
        (b"%userprofile%\\my documents", AppDataKind::Documents),
        (b"%userprofile%\\saved games", AppDataKind::SavedGames),
    ];

    for (token, kind) in env_patterns {
        if norm.starts_with(token) && boundary_a(&norm, token.len()) {
            return portable_folder_a(kind).map(|base| append_suffix_a(base, &path[token.len()..]));
        }
    }

    let path_patterns: [(&[u8], AppDataKind); 6] = [
        (b"\\appdata\\locallow", AppDataKind::LocalLow),
        (b"appdata\\locallow", AppDataKind::LocalLow),
        (b"\\appdata\\roaming", AppDataKind::Roaming),
        (b"appdata\\roaming", AppDataKind::Roaming),
        (b"\\appdata\\local", AppDataKind::Local),
        (b"appdata\\local", AppDataKind::Local),
    ];

    for (token, kind) in path_patterns {
        if let Some(start) = find_token_a(&norm, token) {
            let end = start + token.len();
            let left_ok = token[0] == b'\\' || has_left_boundary_a(&norm, start);
            if left_ok && boundary_a(&norm, end) {
                return portable_folder_a(kind).map(|base| append_suffix_a(base, &path[end..]));
            }
        }
    }

    if let Some(redirected) = redirect_profile_subfolder_path_a(path, &norm) {
        return Some(redirected);
    }

    None
}

pub fn redirect_appdata_path_w(path: &[u16]) -> Option<Vec<u16>> {
    let norm = normalized_wide(path);

    let env_patterns: [(Vec<u16>, AppDataKind); 6] = [
        ("%appdata%".encode_utf16().collect(), AppDataKind::Roaming),
        ("%localappdata%".encode_utf16().collect(), AppDataKind::Local),
        ("%userprofile%\\appdata\\locallow".encode_utf16().collect(), AppDataKind::LocalLow),
        ("%userprofile%\\documents".encode_utf16().collect(), AppDataKind::Documents),
        ("%userprofile%\\my documents".encode_utf16().collect(), AppDataKind::Documents),
        ("%userprofile%\\saved games".encode_utf16().collect(), AppDataKind::SavedGames),
    ];

    for (token, kind) in env_patterns.iter() {
        if norm.starts_with(token) && boundary_w(&norm, token.len()) {
            return portable_folder_w(*kind).map(|base| append_suffix_w(base, &path[token.len()..]));
        }
    }

    let path_patterns: [(Vec<u16>, AppDataKind); 6] = [
        ("\\appdata\\locallow".encode_utf16().collect(), AppDataKind::LocalLow),
        ("appdata\\locallow".encode_utf16().collect(), AppDataKind::LocalLow),
        ("\\appdata\\roaming".encode_utf16().collect(), AppDataKind::Roaming),
        ("appdata\\roaming".encode_utf16().collect(), AppDataKind::Roaming),
        ("\\appdata\\local".encode_utf16().collect(), AppDataKind::Local),
        ("appdata\\local".encode_utf16().collect(), AppDataKind::Local),
    ];

    for (token, kind) in path_patterns.iter() {
        if let Some(start) = find_token_w(&norm, token) {
            let end = start + token.len();
            let left_ok = token[0] == b'\\' as u16 || has_left_boundary_w(&norm, start);
            if left_ok && boundary_w(&norm, end) {
                return portable_folder_w(*kind).map(|base| append_suffix_w(base, &path[end..]));
            }
        }
    }

    if let Some(redirected) = redirect_profile_subfolder_path_w(path, &norm) {
        return Some(redirected);
    }

    None
}

fn redirect_profile_subfolder_path_a(path: &[u8], norm: &[u8]) -> Option<Vec<u8>> {
    let profile_markers: [&[u8]; 2] = [b"%userprofile%", b"\\users\\"];
    let mut has_profile_marker = profile_markers.iter().any(|marker| find_token_a(norm, marker).is_some());
    if !has_profile_marker {
        if let Ok(profile) = std::env::var("USERPROFILE") {
            let profile_norm = normalized_bytes(profile.as_bytes());
            has_profile_marker = !profile_norm.is_empty() && find_token_a(norm, &profile_norm).is_some();
        }
    }
    if !has_profile_marker {
        return None;
    }

    let folder_tokens: [(&[u8], AppDataKind); 3] = [
        (b"\\saved games", AppDataKind::SavedGames),
        (b"\\my documents", AppDataKind::Documents),
        (b"\\documents", AppDataKind::Documents),
    ];

    for (token, kind) in folder_tokens {
        if let Some(start) = find_token_a(norm, token) {
            let end = start + token.len();
            if boundary_a(norm, end) {
                return portable_folder_a(kind).map(|base| append_suffix_a(base, &path[end..]));
            }
        }
    }

    None
}

fn redirect_profile_subfolder_path_w(path: &[u16], norm: &[u16]) -> Option<Vec<u16>> {
    let profile_markers: [Vec<u16>; 2] = [
        "%userprofile%".encode_utf16().collect(),
        "\\users\\".encode_utf16().collect(),
    ];
    let mut has_profile_marker = profile_markers.iter().any(|marker| find_token_w(norm, marker).is_some());
    if !has_profile_marker {
        if let Ok(profile) = std::env::var("USERPROFILE") {
            let profile_wide: Vec<u16> = profile.encode_utf16().collect();
            let profile_norm = normalized_wide(&profile_wide);
            has_profile_marker = !profile_norm.is_empty() && find_token_w(norm, &profile_norm).is_some();
        }
    }
    if !has_profile_marker {
        return None;
    }

    let folder_tokens: [(Vec<u16>, AppDataKind); 3] = [
        ("\\saved games".encode_utf16().collect(), AppDataKind::SavedGames),
        ("\\my documents".encode_utf16().collect(), AppDataKind::Documents),
        ("\\documents".encode_utf16().collect(), AppDataKind::Documents),
    ];

    for (token, kind) in folder_tokens.iter() {
        if let Some(start) = find_token_w(norm, token) {
            let end = start + token.len();
            if boundary_w(norm, end) {
                return portable_folder_w(*kind).map(|base| append_suffix_w(base, &path[end..]));
            }
        }
    }

    None
}

pub fn bytes_with_nul(bytes: Vec<u8>) -> Option<CString> {
    if bytes.iter().any(|&b| b == 0) {
        return None;
    }
    CString::new(bytes).ok()
}

pub fn wide_with_nul(mut wide: Vec<u16>) -> Vec<u16> {
    if wide.last().copied() != Some(0) {
        wide.push(0);
    }
    wide
}

pub unsafe fn write_fixed_a(dst: PSTR, src: &[u8], max_chars: usize) -> bool {
    if dst.as_ptr().is_null() || src.len() + 1 > max_chars {
        return false;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(src.as_ptr(), dst.as_ptr(), src.len());
        *dst.as_ptr().add(src.len()) = 0;
    }
    true
}

pub unsafe fn write_fixed_w(dst: PWSTR, src: &[u16], max_chars: usize) -> bool {
    if dst.as_ptr().is_null() || src.len() + 1 > max_chars {
        return false;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(src.as_ptr(), dst.as_ptr(), src.len());
        *dst.as_ptr().add(src.len()) = 0;
    }
    true
}

pub unsafe fn write_sized_a(dst: PSTR, size: u32, src: &[u8], include_nul_in_success: bool) -> u32 {
    let required = src.len() as u32 + 1;
    if size == 0 || dst.as_ptr().is_null() || size < required {
        return required;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(src.as_ptr(), dst.as_ptr(), src.len());
        *dst.as_ptr().add(src.len()) = 0;
    }
    if include_nul_in_success {
        required
    } else {
        src.len() as u32
    }
}

pub unsafe fn write_sized_w(dst: PWSTR, size: u32, src: &[u16], include_nul_in_success: bool) -> u32 {
    let required = src.len() as u32 + 1;
    if size == 0 || dst.as_ptr().is_null() || size < required {
        return required;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(src.as_ptr(), dst.as_ptr(), src.len());
        *dst.as_ptr().add(src.len()) = 0;
    }
    if include_nul_in_success {
        required
    } else {
        src.len() as u32
    }
}

pub fn ensure_parent_dir_a(path: &[u8]) {
    if let Ok(s) = String::from_utf8(path.to_vec()) {
        if let Some(parent) = PathBuf::from(s).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
    }
}

pub fn ensure_parent_dir_w(path: &[u16]) {
    let s = String::from_utf16_lossy(path);
    if let Some(parent) = PathBuf::from(s).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
}

pub fn ensure_dir_a(path: &[u8]) {
    if let Ok(s) = String::from_utf8(path.to_vec()) {
        let _ = std::fs::create_dir_all(s);
    }
}

pub fn ensure_dir_w(path: &[u16]) {
    let s = String::from_utf16_lossy(path);
    let _ = std::fs::create_dir_all(s);
}


pub fn u16_array_to_string(input: &[u16]) -> Result<String, std::string::FromUtf16Error> {
    let nul_position = input.iter().position(|&r| r == 0).unwrap_or(input.len());
    if nul_position == 0 {
        return Ok(String::new());
    }
    String::from_utf16(&input[0..nul_position])
}
