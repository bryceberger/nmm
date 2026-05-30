#[cfg(target_os = "linux")]
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use color_eyre::eyre::{Result, eyre};
use url::Url;

use crate::{
    api::Api,
    cache::{Cache, CachePath},
};

mod api;
mod cache;
mod models;
mod nxm;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[command(flatten)]
    config: Config,
}

#[derive(clap::Args)]
struct Config {
    #[arg(long, env = "NMM_API_KEY")]
    api_key: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Download a mod file
    Download {
        game: String,
        mod_id: u64,
        file_id: Option<u64>,
    },

    HandleNxm {
        link: String,
    },

    /// Register nmm as the handler for nxm:// protocol links
    Register,

    /// Unregister nmm as the handler for nxm:// protocol links
    Unregister,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

    let api = Api::new(&cli.config.api_key)?;
    let cache = Cache::new()?;

    match cli.command {
        Commands::Download {
            game,
            mod_id,
            file_id,
        } => handle_download(&api, &cache, &game, mod_id, file_id).await,
        Commands::HandleNxm { link } => handle_nxm(&api, &cache, &link).await,
        Commands::Register => register_handler(),
        Commands::Unregister => unregister_handler(),
    }
}

async fn handle_nxm(api: &Api, cache: &Cache, link: &str) -> Result<()> {
    let nxm::Nxm {
        game,
        mod_id,
        file_id,
        key,
        expires,
    } = nxm::Nxm::parse(link)?;
    let cache_path = CachePath::new(game.as_str(), mod_id, file_id)?;
    if let Some(_path) = cache.get(&cache_path)? {
        println!("skipping download for {game}/{mod_id}/{file_id} --- already in cache");
        return Ok(());
    }

    let links: Vec<models::DownloadUrl> = api
        .get(|url| {
            let mut url = url.join(&format!(
                "games/{game}/mods/{mod_id}/files/{file_id}/download_link.json"
            ))?;
            url.set_query(Some(&format!("key={}&expires={}", key, expires)));
            Ok(url)
        })
        .await?;
    let Some(link) = links.first() else {
        return Err(eyre!("got 0 download links"));
    };

    let file_name = link.uri.rsplit_once('/');
    let (_, file_name) =
        file_name.ok_or_else(|| eyre!("cannot determine file name from link: {}", link.uri))?;
    let file_name = file_name
        .split_once('?')
        .map(|(l, _)| l)
        .unwrap_or(file_name);

    let path = api.download(Url::parse(&link.uri)?).await?;
    cache.put(&cache_path, path.path(), file_name)?;

    Ok(())
}

async fn handle_download(
    api: &Api,
    cache: &Cache,
    game: &str,
    mod_id: u64,
    file_id: Option<u64>,
) -> Result<()> {
    let file_id = match file_id {
        Some(x) => x,
        None => get_default_file(api, game, mod_id).await?,
    };

    if let Some(_path) = cache.get(&CachePath::new(game, mod_id, file_id)?)? {
        println!("skipping download for {game}/{mod_id}/{file_id} --- already in cache");
        return Ok(());
    }

    open::that(format!(
        "https://www.nexusmods.com/{game}/mods/{mod_id}?tab=files&file_id={file_id}&nmm=1"
    ))?;

    Ok(())
}

async fn get_default_file(
    api: &Api,
    game: &str,
    mod_id: u64,
) -> Result<u64, color_eyre::eyre::Error> {
    let files: models::Files = api
        .get(|url| {
            Ok(url.join(&format!(
                "games/{game}/mods/{mod_id}/files.json?category=main"
            ))?)
        })
        .await?;
    let [file] = &files.files[..] else {
        return Err(eyre!(
            "more than one file_id found as primary: https://www.nexusmods.com/{game}/mods/{mod_id}?tab=files"
        ));
    };
    Ok(file.file_id)
}

#[cfg(target_os = "linux")]
fn register_handler() -> Result<()> {
    let desktop_path = desktop_file_path()?;

    let desktop_content = r#"[Desktop Entry]
Version=1.0
Type=Application
Name=Nexus Mods Manager
Comment=Handle nxm:// protocol links
Exec=nmm handle-nxm %u
Terminal=true
MimeType=x-scheme-handler/nxm;
"#;

    if let Some(parent) = desktop_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&desktop_path, desktop_content)?;

    let result = std::process::Command::new("xdg-mime")
        .arg("default")
        .arg(&desktop_path)
        .arg("x-scheme-handler/nxm")
        .output()
        .map_err(|e| eyre!("failed to run xdg-mime: {}", e))?;

    if result.status.success() {
        println!("Desktop entry created at: {}", desktop_path.display());
        Ok(())
    } else {
        Err(eyre!("xdg-mime failed to register the handler"))
    }
}

#[cfg(target_os = "linux")]
fn unregister_handler() -> Result<()> {
    let desktop_path = desktop_file_path()?;
    if !desktop_path.exists() {
        eprintln!("Desktop entry not found at: {}", desktop_path.display());
        return Ok(());
    }

    std::process::Command::new("xdg-mime")
        .arg("uninstall")
        .arg(DESKTOP_FILE_NAME)
        .output()?;

    std::fs::remove_file(&desktop_path)?;
    Ok(())
}

const DESKTOP_FILE_NAME: &str = "nmm-nmm.desktop";
#[cfg(target_os = "linux")]
fn desktop_file_path() -> Result<PathBuf> {
    let mut path = dirs::data_dir().ok_or_else(|| eyre!("cannot determine data directory"))?;
    path.push("applications");
    path.push(DESKTOP_FILE_NAME);
    Ok(path)
}

#[cfg(not(target_os = "linux"))]
fn register_handler() -> Result<()> {
    Err(eyre!(
        "Protocol handler registration is only supported on Linux."
    ))
}

#[cfg(not(target_os = "linux"))]
fn unregister_handler() -> Result<()> {
    Err(eyre!(
        "Protocol handler registration is only supported on Linux."
    ))
}
