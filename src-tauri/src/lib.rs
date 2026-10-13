use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    env, fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const DAY_SECONDS: u64 = 86_400;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Settings {
    downloads_path: String,
    organize: bool,
    temp_mode: String,
    temp_days: u64,
    log_path: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Report {
    lines: Vec<String>,
    freed_bytes: u64,
    moved: usize,
    deleted: usize,
    skipped: usize,
    errors: usize,
}

struct Totals {
    moved: usize,
    deleted: usize,
    removed_dirs: usize,
    skipped: usize,
    errors: usize,
    freed: u64,
}

fn now_seconds() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

fn category(extension: &str) -> &'static str {
    match extension.to_lowercase().as_str() {
        "pdf" => "Documentos\\PDFs",
        "doc" | "docx" | "odt" | "rtf" | "txt" | "md" => "Documentos\\Textos",
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "tif" | "tiff" | "svg" | "heic" => "Imagens\\Organizadas",
        "mp3" | "wav" | "aac" | "flac" | "m4a" | "ogg" => "Mídia\\Áudio",
        "mp4" | "mkv" | "mov" | "avi" | "webm" => "Mídia\\Vídeos",
        "zip" | "7z" | "rar" | "tar" | "gz" => "Arquivos compactados",
        "msi" | "exe" | "msix" | "appx" | "cab" | "dmg" | "deb" | "rpm" => "Instaladores arquivados",
        _ => "Outros",
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes == 0 { return "0 B".into(); }
    let units = ["B", "KB", "MB", "GB", "TB"];
    let idx = ((bytes as f64).log(1024.0).floor() as usize).min(units.len() - 1);
    format!("{:.2} {}", bytes as f64 / 1024_f64.powi(idx as i32), units[idx])
}

fn push_error(lines: &mut Vec<String>, totals: &mut Totals, label: &str, path: &Path, err: &std::io::Error) {
    totals.errors += 1;
    lines.push(format!("[ERRO] {label} {}: {err}", path.display()));
}

fn unique_destination(dir: &Path, name: &std::ffi::OsStr) -> PathBuf {
    let original = dir.join(name);
    if !original.exists() { return original; }
    let stem = Path::new(name).file_stem().unwrap_or(name).to_string_lossy();
    let ext = Path::new(name).extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    for index in 1.. {
        let candidate = dir.join(format!("{stem} ({index}){ext}"));
        if !candidate.exists() { return candidate; }
    }
    unreachable!()
}

fn temp_directories() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    for key in ["TEMP", "TMP"] {
        if let Some(value) = env::var_os(key) { dirs.push(PathBuf::from(value)); }
    }
    dirs.push(env::temp_dir());
    if let Some(root) = env::var_os("SystemRoot") { dirs.push(PathBuf::from(root).join("Temp")); }
    let mut seen = HashSet::new();
    dirs.retain(|p| seen.insert(p.to_string_lossy().to_lowercase()));
    dirs
}

fn inspect_temp(path: &Path, cutoff: u64, all: bool, apply: bool, lines: &mut Vec<String>, totals: &mut Totals) {
    let meta = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) => { push_error(lines, totals, "ao inspecionar", path, &e); return; }
    };
    let kind = meta.file_type();
    if kind.is_symlink() {
        let modified = meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
        if !all && modified > cutoff { return; }
        if apply {
            match fs::remove_file(path).or_else(|_| fs::remove_dir(path)) { Ok(()) => { totals.deleted += 1; lines.push(format!("[LINK removido] {}", path.display())); }, Err(e) => { totals.skipped += 1; lines.push(format!("[IGNORADO] {}: {e}", path.display())); } }
        } else { lines.push(format!("[SIMULAÇÃO: remover link] {}", path.display())); }
        return;
    }
    if kind.is_dir() {
        let entries = match fs::read_dir(path) {
            Ok(v) => v,
            Err(e) => { totals.skipped += 1; lines.push(format!("[IGNORADA: sem acesso] {}: {e}", path.display())); return; }
        };
        for item in entries {
            match item { Ok(e) => inspect_temp(&e.path(), cutoff, all, apply, lines, totals), Err(e) => { totals.errors += 1; lines.push(format!("[ERRO ao listar TEMP] {}: {e}", path.display())); } }
        }
        if all {
            if apply {
                match fs::remove_dir(path) { Ok(()) => { totals.removed_dirs += 1; lines.push(format!("[PASTA removida] {}", path.display())); }, Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied || e.kind() == std::io::ErrorKind::DirectoryNotEmpty => { totals.skipped += 1; lines.push(format!("[PASTA mantida, em uso ou sem acesso] {}: {e}", path.display())); }, Err(e) => push_error(lines, totals, "ao remover pasta", path, &e) }
            } else { lines.push(format!("[SIMULAÇÃO: remover pasta] {}", path.display())); }
        }
        return;
    }
    if !kind.is_file() { totals.skipped += 1; lines.push(format!("[IGNORADO: tipo não suportado] {}", path.display())); return; }
    let modified = meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
    if !all && modified > cutoff { return; }
    let size = meta.len();
    if !apply { lines.push(format!("[SIMULAÇÃO: apagar] {} ({})", path.display(), format_bytes(size))); return; }
    match fs::remove_file(path) {
        Ok(()) => { totals.deleted += 1; totals.freed += size; lines.push(format!("[APAGADO] {} ({})", path.display(), format_bytes(size))); }
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => { totals.skipped += 1; lines.push(format!("[IGNORADO, em uso ou sem permissão] {}: {e}", path.display())); }
        Err(e) => push_error(lines, totals, "ao apagar", path, &e),
    }
}

#[tauri::command]
fn default_downloads() -> Result<String, String> {
    let home = env::var_os("USERPROFILE").map(PathBuf::from).or_else(|| dirs_home());
    home.map(|p| p.join("Downloads").to_string_lossy().into_owned()).ok_or_else(|| "Não foi possível localizar a pasta do usuário.".to_string())
}

fn dirs_home() -> Option<PathBuf> { env::var_os("HOME").map(PathBuf::from) }

#[tauri::command]
fn run_cleaner(settings: Settings, apply: bool) -> Result<Report, String> {
    if !settings.organize && settings.temp_mode == "none" { return Err("Selecione ao menos uma ação.".into()); }
    let downloads = PathBuf::from(&settings.downloads_path);
    if settings.organize && (!downloads.is_absolute() || !downloads.is_dir()) { return Err("A pasta Downloads informada não existe ou não é válida.".into()); }
    let mut lines = vec![format!("--- {} ---", if apply { "Início da execução" } else { "Início da simulação" })];
    let mut totals = Totals { moved: 0, deleted: 0, removed_dirs: 0, skipped: 0, errors: 0, freed: 0 };

    if settings.organize {
        lines.push(format!("[DOWNLOADS] Pasta: {}", downloads.display()));
        match fs::read_dir(&downloads) {
            Ok(entries) => {
                let mut examined = 0;
                for item in entries {
                    let entry = match item { Ok(v) => v, Err(e) => { totals.errors += 1; lines.push(format!("[ERRO ao listar Downloads] {e}")); continue; } };
                    let source = entry.path();
                    let metadata = match fs::symlink_metadata(&source) { Ok(v) => v, Err(e) => { push_error(&mut lines, &mut totals, "ao ler", &source, &e); continue; } };
                    if !metadata.is_file() || metadata.file_type().is_symlink() { continue; }
                    examined += 1;
                    let ext = source.extension().unwrap_or_default().to_string_lossy();
                    let category = category(&ext);
                    let target_dir = downloads.join(category);
                    let target = unique_destination(&target_dir, entry.file_name().as_os_str());
                    if !apply { lines.push(format!("[SIMULAÇÃO: mover] {} -> {}", source.display(), target.display())); continue; }
                    if let Err(e) = fs::create_dir_all(&target_dir) { push_error(&mut lines, &mut totals, "ao criar pasta", &target_dir, &e); continue; }
                    match fs::rename(&source, &target) {
                        Ok(()) => { totals.moved += 1; lines.push(format!("[MOVIDO] {} -> {}", source.display(), target.display())); }
                        Err(first) if first.raw_os_error() == Some(EXDEV_CODE) => {
                            match fs::copy(&source, &target) { Ok(_) => match fs::remove_file(&source) { Ok(()) => { totals.moved += 1; lines.push(format!("[MOVIDO] {} -> {}", source.display(), target.display())); }, Err(e) => push_error(&mut lines, &mut totals, "ao remover original", &source, &e) }, Err(e) => push_error(&mut lines, &mut totals, "ao copiar", &source, &e) }
                        }
                        Err(e) => push_error(&mut lines, &mut totals, "ao mover", &source, &e),
                    }
                }
                lines.push(format!("[DOWNLOADS] Verificados {examined} arquivo(s) na raiz; organização sem filtro de idade."));
                lines.push("[POLÍTICA] Instaladores são arquivados; não são apagados automaticamente.".into());
            }
            Err(e) => push_error(&mut lines, &mut totals, "ao abrir Downloads", &downloads, &e),
        }
    }

    if settings.temp_mode != "none" {
        let all = settings.temp_mode == "all";
        let cutoff = now_seconds().saturating_sub(settings.temp_days.saturating_mul(DAY_SECONDS));
        for dir in temp_directories() {
            let home = env::var_os("USERPROFILE").map(PathBuf::from).or_else(dirs_home);
            let cwd = env::current_dir().ok();
            let too_broad = home.as_ref().is_some_and(|p| &dir == p) || cwd.as_ref().is_some_and(|p| &dir == p);
            if !dir.is_absolute() || dir.parent().is_none() || dir.to_string_lossy().trim().is_empty() || too_broad || dir == downloads || dir.starts_with(&downloads) || downloads.starts_with(&dir) {
                totals.skipped += 1; lines.push(format!("[IGNORADA por segurança] Pasta TEMP inválida ou sobreposta: {}", dir.display())); continue;
            }
            lines.push(format!("[TEMP] Pasta: {}", dir.display()));
            match fs::read_dir(&dir) {
                Ok(entries) => for item in entries {
                    match item { Ok(e) => inspect_temp(&e.path(), cutoff, all, apply, &mut lines, &mut totals), Err(e) => { totals.errors += 1; lines.push(format!("[ERRO ao listar TEMP] {}: {e}", dir.display())); } }
                },
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => { totals.skipped += 1; lines.push(format!("[IGNORADA: pasta ausente] {}", dir.display())); }
                Err(e) => { totals.skipped += 1; lines.push(format!("[IGNORADA: sem acesso] {}: {e}", dir.display())); }
            }
        }
    }
    lines.push(format!("[RESUMO] Movidos: {}; arquivos apagados: {}; pastas removidas: {}; ignorados: {}; erros: {}; espaço liberado: {}", totals.moved, totals.deleted, totals.removed_dirs, totals.skipped, totals.errors, format_bytes(totals.freed)));
    lines.push(format!("--- Fim da {} ---", if apply { "execução" } else { "simulação" }));
    if !settings.log_path.trim().is_empty() {
        let log_path = PathBuf::from(&settings.log_path);
        if let Some(parent) = log_path.parent() { let _ = fs::create_dir_all(parent); }
        let content = format!("{}\n", lines.join("\r\n"));
        match fs::OpenOptions::new().create(true).append(true).open(&log_path) {
            Ok(mut file) => { use std::io::Write; if let Err(e) = file.write_all(content.as_bytes()) { lines.push(format!("[ERRO ao salvar log] {e}")); } }
            Err(e) => lines.push(format!("[ERRO ao abrir log] {}: {e}", log_path.display())),
        }
    }
    Ok(Report { lines, freed_bytes: totals.freed, moved: totals.moved, deleted: totals.deleted, skipped: totals.skipped, errors: totals.errors })
}

// ERROR_NOT_SAME_DEVICE (Windows): fallback from rename to copy/remove.
#[cfg(windows)] const EXDEV_CODE: i32 = 17;
#[cfg(not(windows))] const EXDEV_CODE: i32 = 18;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![default_downloads, run_cleaner])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}
