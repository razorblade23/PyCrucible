use reqwest::blocking::get;
use std::io::Cursor;

pub enum Archive<'a> {
    Zip(Cursor<Vec<u8>>),
    TarGz(&'a mut reqwest::blocking::Response),
}

pub fn build_release_url(version: &str, target: &str) -> String {
    let ext = if target.contains("windows") {
        "zip"
    } else {
        "tar.gz"
    };

    format!(
        "https://github.com/astral-sh/uv/releases/download/{v}/uv-{target}.{ext}",
        v = version
    )
}

pub fn download(url: &str) -> Result<DownloadResult, Box<dyn std::error::Error>> {
    let response = get(url)?.error_for_status()?;
    if url.ends_with(".zip") {
        let bytes = response.bytes()?.to_vec();
        Ok(DownloadResult::Zip(Cursor::new(bytes)))
    } else {
        Ok(DownloadResult::TarGz(response))
    }
}

pub enum DownloadResult {
    Zip(Cursor<Vec<u8>>),
    TarGz(reqwest::blocking::Response),
}
