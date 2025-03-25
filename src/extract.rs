use anyhow::Context;

/// # Errors
/// This functions returns an error when the following occurs:
/// * The source archive file cannot be opened
/// * The `ZipArchive` crate cannot read the file
pub fn decompress_zip(
    archive_path: &std::path::Path,
    pack_path: &std::path::Path,
) -> anyhow::Result<()> {
    let archive =
        std::fs::File::open(archive_path).with_context(|| "failed to open temp archive")?;
    let mut file_zip =
        zip::ZipArchive::new(archive).with_context(|| "failed to read the archive file")?;
    for i in 0..file_zip.len() {
        let mut item = match file_zip
            .by_index(i)
            .with_context(|| "failed to get file within archive")
        {
            Ok(item) => item,
            Err(e) => {
                println!("{e:#}");
                continue;
            }
        };

        let Some(item_path) = item.enclosed_name() else {
            continue;
        };
        if item.is_dir() {
            continue;
        }

        let item_name = match item_path
            .file_name()
            .context("failed to fetch archive item's name")
        {
            Ok(item_name) => item_name,
            Err(e) => {
                println!("{e:#}");
                continue;
            }
        };

        let mut file_on_disk = match std::fs::File::create(pack_path.join(item_name))
            .with_context(|| "failed to create extracted file")
        {
            Ok(file) => file,
            Err(e) => {
                println!("{e:#}");
                continue;
            }
        };

        if let Err(e) = std::io::copy(&mut item, &mut file_on_disk)
            .with_context(|| "failed to write archive file contents to disk")
        {
            println!("{e:#}");
            continue;
        };
    }

    Ok(())
}

/// # Errors
/// This functions returns an error when the following occurs:
/// * `sevenz_rust2::decompress_file_with_extract_fn` cannot find/open the source archive file
pub fn decompress_sevenzip(
    archive_path: &std::path::Path,
    pack_path: &std::path::Path,
) -> anyhow::Result<()> {
    sevenz_rust2::decompress_file_with_extract_fn(
        archive_path,
        pack_path,
        |archive_file, reader, _| {
            if archive_file.is_directory() {
                return Ok(true);
            }

            let mut extracted_file = std::fs::File::create(pack_path.join(archive_file.name()))?;
            std::io::copy(reader, &mut extracted_file)?;
            Ok(true)
        },
    )?;

    Ok(())
}
