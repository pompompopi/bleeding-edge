use std::path::{Path, PathBuf};

pub fn walk_dir_recursively<F>(path: &Path, apply: &mut F) -> anyhow::Result<()>
where
    F: FnMut(PathBuf),
{
    path.read_dir()?
        .filter_map(|dr| dr.ok())
        .map(|d| d.path())
        .for_each(|p| {
            apply(p.clone());
            let _ = walk_dir_recursively(p.as_path(), apply);
        });

    Ok(())
}
