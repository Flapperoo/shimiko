use anyhow::Context;

use crate::extract;

#[derive(Debug)]
pub enum Extractor {
    Zip,
    SevenZip,
}

#[derive(Debug)]
pub struct PackerTaskFail {
    pack_num: u16,
    error: anyhow::Error,
}

impl PackerTaskFail {
    #[must_use]
    pub fn new(pack_num: u16, error: anyhow::Error) -> Self {
        PackerTaskFail {
            pack_num: (pack_num),
            error: (error),
        }
    }

    #[must_use]
    pub fn pack_num(&self) -> &u16 {
        &self.pack_num
    }

    #[must_use]
    pub fn error(&self) -> &anyhow::Error {
        &self.error
    }
}

#[derive(Debug)]
pub struct PackerTask {
    archive_path: tempfile::TempPath,
    extractor: Extractor,
    pack_num: u16,
    task_progress: indicatif::ProgressBar,
}

impl PackerTask {
    /// # Errors
    /// This functions returns an error when the following occurs:
    /// * The URL is malformed and returns a non-200 code
    /// * The download fails for any reason
    pub async fn new(
        pack_num: u16,
        client: &reqwest::Client,
        pb: indicatif::ProgressBar,
    ) -> anyhow::Result<Self> {
        pb.set_message("Downloading...");

        let url = form_url(pack_num)?;
        let extractor = if url.to_string().to_lowercase().ends_with(".zip") {
            Extractor::Zip
        } else {
            Extractor::SevenZip
        };
        let archive_path = match download(url, client).await {
            Ok(path) => path,
            Err(e) => {
                pb.finish_with_message("Downloading...Failed");
                return Err(e);
            }
        };

        Ok(PackerTask {
            archive_path,
            extractor,
            pack_num,
            task_progress: pb,
        })
    }

    /// # Errors
    /// This functions returns an error when the following occurs:
    /// * The archive is malformed or corrupted
    pub fn extract(&self, pack_path: &std::path::Path) -> anyhow::Result<()> {
        match self.extractor {
            Extractor::Zip => extract::decompress_zip(&self.archive_path, pack_path),
            Extractor::SevenZip => extract::decompress_sevenzip(&self.archive_path, pack_path),
        }
    }

    #[must_use]
    pub fn progress_bar(&self) -> &indicatif::ProgressBar {
        &self.task_progress
    }

    #[must_use]
    pub fn pack_num(&self) -> &u16 {
        &self.pack_num
    }
}

async fn download(
    url: reqwest::Url,
    client: &reqwest::Client,
) -> anyhow::Result<tempfile::TempPath> {
    let response = fetch_valid_response(url, client).await?;
    let response_data = response.bytes().await?;
    let file = tempfile::NamedTempFile::new()?;
    tokio::fs::write(file.path(), response_data)
        .await
        .with_context(|| "failed to write archive into file")?;
    Ok(file.into_temp_path())
}

async fn fetch_valid_response(
    url: reqwest::Url,
    client: &reqwest::Client,
) -> anyhow::Result<reqwest::Response> {
    let response = client.get(url).send().await?;
    if response.status() != reqwest::StatusCode::OK {
        anyhow::bail!(
            "failed get request, response returned status code: {}",
            response.status()
        );
    }
    Ok(response)
}

fn form_url(pack_num: u16) -> anyhow::Result<reqwest::Url> {
    match pack_num {
        (1318..) | 5 | 124 | 267 | 415 | 479 | 884 => Ok(reqwest::Url::parse(&format!(
            "https://packs.ppy.sh/S{pack_num}%20-%20osu%21%20Beatmap%20Pack%20%23{pack_num}.zip"
        ))?),
        1300.. => Ok(reqwest::Url::parse(&format!(
            "https://packs.ppy.sh/S{pack_num}%20-%20Beatmap%20Pack%20%23{pack_num}.zip"
        ))?),
        _ => Ok(reqwest::Url::parse(&format!(
            "https://packs.ppy.sh/S{pack_num}%20-%20Beatmap%20Pack%20%23{pack_num}.7z"
        ))?),
    }
}

#[cfg(test)]
mod url_validity {
    use super::*;

    /// Checks URL validity as endpoints are sensitive and will change without notice.
    /// `pack_num_limit` follows latest available standard pack at the time of testing.
    #[tokio::test]
    #[ignore]
    async fn test_valid_urls() {
        let pack_num_limit: u16 = 1740;
        let http_client = reqwest::Client::new();

        for pack_num in 1..=pack_num_limit {
            let url = form_url(pack_num).expect("failure to parse url");
            if let Err(e) = fetch_valid_response(url.clone(), &http_client).await {
                panic!("{e:#} in {pack_num}")
            }
        }
    }
}
