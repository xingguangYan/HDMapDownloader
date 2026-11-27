//! 导出：把已下载的瓦片打包为 ZIP，并在资源管理器中打开
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

/// 把 cache_root/tiles/<layer>/ 下的瓦片打包到 out_path
pub fn zip_tiles(cache_root: &Path, layer: &str, out_path: &Path) -> Result<usize, String> {
    let tiles_dir = cache_root.join("tiles").join(layer);
    if !tiles_dir.exists() {
        return Err(format!("没有已下载的瓦片目录: {}", tiles_dir.display()));
    }
    let files = walk(&tiles_dir, &tiles_dir)?;
    if files.is_empty() {
        return Err("瓦片目录为空，请先下载".into());
    }
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let file = File::create(out_path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let opts = zip::write::FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let mut count = 0usize;
    for (rel, full) in files {
        let mut buf = Vec::new();
        File::open(&full)
            .and_then(|mut f| f.read_to_end(&mut buf))
            .map_err(|e| e.to_string())?;
        zip.start_file(format!("{layer}/{rel}"), opts).map_err(|e| e.to_string())?;
        zip.write_all(&buf).map_err(|e| e.to_string())?;
        count += 1;
    }
    zip.finish().map_err(|e| e.to_string())?;
    Ok(count)
}

fn walk(dir: &Path, root: &Path) -> Result<Vec<(String, std::path::PathBuf)>, String> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
        let e = e.map_err(|e| e.to_string())?;
        let p = e.path();
        if p.is_dir() {
            out.extend(walk(&p, root)?);
        } else {
            let rel = p
                .strip_prefix(root)
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .replace('\\', "/");
            out.push((rel, p));
        }
    }
    Ok(out)
}

#[cfg(target_os = "windows")]
pub fn open_in_explorer(path: &Path) {
    let _ = std::process::Command::new("explorer").arg(path).spawn();
}

#[cfg(not(target_os = "windows"))]
pub fn open_in_explorer(path: &Path) {
    let _ = std::process::Command::new("xdg-open").arg(path).spawn();
}
