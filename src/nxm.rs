use color_eyre::eyre::{Result, eyre};
use url::Url;

#[derive(Debug)]
pub struct Nxm {
    pub game: String,
    pub mod_id: u64,
    pub file_id: u64,
    pub key: String,
    pub expires: String,
}

impl Nxm {
    pub fn parse(input: &str) -> Result<Self> {
        let url = Url::parse(input)?;

        if url.scheme() != "nxm" {
            return Err(eyre!("expected `nxm://` scheme, got `{}`", url.scheme()));
        }

        let invalid_format =
            || eyre!("invalid path, expected format: nxm://<game>/mods/<mod_id>/files/<file_id>");

        let game = url.domain().ok_or_else(invalid_format)?;
        let parts: Vec<&str> = url
            .path_segments()
            .ok_or_else(|| eyre!("cannot be a base"))?
            .collect();

        let [_, mod_id, _, file_id] = &parts[..] else {
            return Err(invalid_format());
        };
        let mut me = Self {
            game: game.into(),
            mod_id: mod_id.parse()?,
            file_id: file_id.parse()?,
            key: String::new(),
            expires: String::new(),
        };

        for (key, value) in url.query_pairs() {
            match key.as_ref() {
                "key" => me.key = value.into_owned(),
                "expires" => me.expires = value.into_owned(),
                _ => (),
            }
        }

        if me.key.is_empty() {
            return Err(eyre!("missing `key` parameter"));
        }

        if me.expires.is_empty() {
            return Err(eyre!("missing `expires` parameter"));
        }

        Ok(me)
    }
}
