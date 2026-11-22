use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogApp {
    pub name: String,
    pub path: String,
}

fn collect_lnk_recursive(dir: &Path, depth: usize, seen: &mut HashSet<String>, out: &mut Vec<CatalogApp>) {
    if depth > 4 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_lnk_recursive(&path, depth + 1, seen, out);
        } else if path
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("lnk"))
        {
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let lower = stem.to_ascii_lowercase();
            if lower.contains("uninstall") || lower.contains("readme") || lower.contains("help") {
                continue;
            }
            if seen.insert(lower) {
                out.push(CatalogApp {
                    name: stem.to_string(),
                    path: path.display().to_string(),
                });
            }
        }
    }
}

pub fn scan_start_menu_apps() -> Vec<CatalogApp> {
    let mut roots = Vec::new();
    if let Some(pd) = std::env::var_os("ProgramData") {
        roots.push(PathBuf::from(pd).join("Microsoft").join("Windows").join("Start Menu").join("Programs"));
    }
    if let Some(ad) = std::env::var_os("APPDATA") {
        roots.push(PathBuf::from(ad).join("Microsoft").join("Windows").join("Start Menu").join("Programs"));
    }
    let mut seen = HashSet::new();
    let mut apps = Vec::with_capacity(128);
    for root in roots {
        collect_lnk_recursive(&root, 0, &mut seen, &mut apps);
    }
    apps.sort_by(|a, b| a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()));
    apps
}
