#[derive(serde::Deserialize, Debug)]
pub struct Files {
    pub files: Vec<File>,
}

#[derive(serde::Deserialize, Debug)]
pub struct File {
    pub file_id: u64,
}

#[derive(serde::Deserialize, Debug)]
pub struct DownloadUrl {
    #[serde(rename = "URI")]
    pub uri: String,
}
