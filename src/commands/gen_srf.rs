use crate::md5_digest::Md5Digest;
use crate::mod_cache::ModCache;
use crate::{mod_cache, srf};
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use walkdir::WalkDir;

pub fn gen_srf_for_mod(mod_path: &Path, output_dir: Option<&Path>) -> srf::Mod {
    let generated_srf = srf::scan_mod(mod_path).unwrap();

    let path = match output_dir {
        Some(out_dir) => {
            let mod_name = mod_path.file_name().unwrap();
            out_dir.join(mod_name).join("mod.srf")
        }
        None => mod_path.join("mod.srf"),
    };

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }

    let writer = BufWriter::new(File::create(path).unwrap());
    serde_json::to_writer(writer, &generated_srf).unwrap();

    generated_srf
}

pub fn open_cache_or_gen_srf(base_path: &Path) -> Result<ModCache, mod_cache::Error> {
    match ModCache::from_disk(base_path) {
        Ok(cache) => Ok(cache),
        Err(mod_cache::Error::FileOpen { source })
            if source.kind() == std::io::ErrorKind::NotFound =>
        {
            println!("nimble-cache.json not found, generating...");
            gen_srf(base_path, None, None)?;
            ModCache::from_disk_or_empty(base_path)
        }
        Err(e) => Err(e),
    }
}

pub fn gen_srf(
    base_path: &Path, 
    output_dir: Option<&Path>,
    progress_callback: Option<Box<dyn Fn(String, f32, usize, usize) + Send + Sync>>
) -> Result<(), mod_cache::Error> {
    let progress_fn = progress_callback.unwrap_or_else(|| Box::new(|_, _, _, _| {}));
    let mod_dirs: Vec<_> = WalkDir::new(base_path)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_dir() && e.file_name().to_string_lossy().starts_with('@'))
        .collect();

    let total_mods = mod_dirs.len();
    let processed_count = Arc::new(AtomicUsize::new(0));

    let mods: HashMap<Md5Digest, srf::Mod> = mod_dirs
        .into_par_iter()
        .map(|entry| {
            let path = entry.path();
            let mod_name = path.file_name().unwrap().to_string_lossy().to_string();
            let srf = gen_srf_for_mod(path, output_dir);

            let processed = processed_count.fetch_add(1, Ordering::SeqCst) + 1;
            let progress = processed as f32 / total_mods as f32;
            progress_fn(mod_name, progress, processed, total_mods);

            (srf.checksum.clone(), srf)
        })
        .collect();

    let cache = ModCache::new(mods)?;
    progress_fn("Saving cache".to_string(), 1.0, total_mods, total_mods);
    cache.to_disk(output_dir.unwrap_or(base_path))
}
