use std::{fs::File, path::Path};

use flate2::{write::GzEncoder, Compression};
use tracing::info;

pub fn archive(
    directory: &Path,
    output_directory: &Path,
    output_file_name: &str,
) -> anyhow::Result<()> {
    let archive_name = output_file_name.to_owned() + ".tar.gz";
    info!("Creating archive {}...", archive_name);
    let mut tar_builder = tar::Builder::new(GzEncoder::new(
        File::create(output_directory.join(&archive_name))?,
        Compression::fast(),
    ));
    tar_builder.append_dir_all(output_file_name, directory)?;
    tar_builder.finish()?;
    info!("Created archive {}!", archive_name);

    Ok(())
}
