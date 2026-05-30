use std::io::{BufWriter, Write};

use color_eyre::eyre::Result;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::de::DeserializeOwned;
use tempfile::NamedTempFile;
use url::Url;

pub struct Api {
    base_url: Url,
    client: reqwest::Client,
}

impl Api {
    pub fn new(api_key: &str) -> Result<Self> {
        let base_url = Url::parse("https://api.nexusmods.com/v1/")?;

        let user_agent = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);
        let mut headers = HeaderMap::new();
        headers.insert("apikey", HeaderValue::from_str(api_key)?);
        let client = reqwest::ClientBuilder::new()
            .user_agent(user_agent)
            .default_headers(headers)
            .build()?;

        Ok(Self { base_url, client })
    }

    pub async fn get<T: DeserializeOwned>(&self, f: impl FnOnce(Url) -> Result<Url>) -> Result<T> {
        let get = self.client.get(f(self.base_url.clone())?);
        Ok(get.send().await?.json().await?)
    }

    pub async fn download(&self, url: Url) -> Result<NamedTempFile> {
        let mut writer = BufWriter::new(NamedTempFile::new()?);

        let mut response = self.client.get(url).send().await?;

        // TODO: progress bar should be passed in, multi progress bars?
        let template = "{spinner:.green} {elapsed:>3}/{duration:>3} {bar} {bytes}/{total_bytes} ({bytes_per_sec})";
        let style = indicatif::ProgressStyle::with_template(template)?;
        let pb = indicatif::ProgressBar::no_length()
            .with_finish(indicatif::ProgressFinish::Abandon)
            .with_style(style);
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
        if let Some(len) = response.content_length() {
            pb.set_length(len);
        }

        while let Some(chunk) = response.chunk().await? {
            writer.write_all(&chunk)?;
            pb.set_position(pb.position() + (chunk.len() as u64));
        }

        Ok(writer.into_inner()?)
    }
}
