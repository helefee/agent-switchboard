use std::path::Path;

pub(crate) fn replace(source: &Path, target: &Path) -> Result<(), String> { publish(source, target, true) }

pub(crate) fn move_new(source: &Path, target: &Path) -> Result<(), String> {
    if target.try_exists().map_err(|error| error.to_string())? { return Err(format!("目标已存在，未覆盖：{}", target.display())); }
    publish(source, target, false)
}

#[cfg(windows)]
fn publish(source: &Path, target: &Path, replace: bool) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH};
    let source = source.canonicalize().map_err(|error| error.to_string())?;
    let target = target.parent().ok_or("目标位置无效")?.canonicalize().map_err(|error| error.to_string())?
        .join(target.file_name().ok_or("目标文件名无效")?);
    let wide = |path: &Path| -> Result<Vec<u16>, String> {
        let mut value = path.as_os_str().encode_wide().collect::<Vec<_>>();
        if value.contains(&0) { return Err("路径包含无效字符".into()); }
        value.push(0); Ok(value)
    };
    let source = wide(&source)?;
    let target = wide(&target)?;
    let flags = MOVEFILE_WRITE_THROUGH | if replace { MOVEFILE_REPLACE_EXISTING } else { 0 };
    if unsafe { MoveFileExW(source.as_ptr(), target.as_ptr(), flags) } == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(())
}

#[cfg(not(windows))]
fn publish(source: &Path, target: &Path, replace: bool) -> Result<(), String> {
    if replace { std::fs::rename(source, target).map_err(|error| error.to_string())?; }
    else { rename_new(source, target)?; }
    sync_parent(target)?;
    if source.parent() != target.parent() { sync_parent(source)?; }
    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn rename_new(source: &Path, target: &Path) -> Result<(), String> {
    use std::os::unix::ffi::OsStrExt;
    let source = std::ffi::CString::new(source.as_os_str().as_bytes()).map_err(|error| error.to_string())?;
    let target = std::ffi::CString::new(target.as_os_str().as_bytes()).map_err(|error| error.to_string())?;
    #[cfg(target_os = "linux")]
    let result = unsafe { libc::renameat2(libc::AT_FDCWD, source.as_ptr(), libc::AT_FDCWD, target.as_ptr(), libc::RENAME_NOREPLACE) };
    #[cfg(target_os = "macos")]
    let result = unsafe { libc::renamex_np(source.as_ptr(), target.as_ptr(), libc::RENAME_EXCL) };
    if result != 0 { return Err(std::io::Error::last_os_error().to_string()); }
    Ok(())
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn rename_new(_source: &Path, _target: &Path) -> Result<(), String> {
    Err("当前平台不支持不覆盖既有目标的材料移动，原件已保留".into())
}

#[cfg(not(windows))]
pub(crate) fn sync_parent(path: &Path) -> Result<(), String> {
    std::fs::File::open(path.parent().ok_or("目标位置无效")?).and_then(|directory| directory.sync_all())
        .map_err(|error| format!("材料已移动，但目录持久化未确认：{error}"))
}

#[cfg(windows)]
pub(crate) fn sync_parent(_path: &Path) -> Result<(), String> { Ok(()) }
