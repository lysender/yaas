use serde::Deserialize;
use snafu::ResultExt;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::{env, fs};

use crate::error::{ManifestParseSnafu, ManifestReadSnafu};
use crate::{Error, Result};

#[derive(Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub jwt_secret: String,
    pub api_url: String,
    pub frontend_dir: PathBuf,
    pub captcha_site_key: Option<String>,
    pub captcha_api_key: Option<String>,
    pub ga_tag_id: Option<String>,
    pub assets: AssetManifest,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub address: String,
    pub https: bool,
}

#[derive(Deserialize)]
struct BundleEntry {
    pub file: String,
}

type BundleConfigMap = HashMap<String, BundleEntry>;

#[derive(Clone, Deserialize)]
pub struct AssetManifest {
    pub main_css: String,
    pub main_js: String,
}

impl AssetManifest {
    pub fn build(frontend_dir: &PathBuf) -> Result<Self> {
        let filename = Path::new(frontend_dir).join("public/assets/bundles/.vite/manifest.json");
        let contents = fs::read_to_string(filename).context(ManifestReadSnafu)?;
        let config_map = serde_json::from_str::<BundleConfigMap>(contents.as_str())
            .context(ManifestParseSnafu)?;

        let main_css = config_map
            .get("bundles/main.css")
            .expect("main.css bundle is required");

        let main_js = config_map
            .get("bundles/main.js")
            .expect("main.js bundle is required");

        Ok(AssetManifest {
            main_css: format!("/assets/bundles/{}", main_css.file),
            main_js: format!("/assets/bundles/{}", main_js.file),
        })
    }
}

impl Config {
    pub fn captcha_enabled(&self) -> bool {
        self.captcha_site_key.is_some() && self.captcha_api_key.is_some()
    }

    pub fn build() -> Result<Self> {
        // Build the config from ENV vars
        let frontend_dir = PathBuf::from(required_env("FRONTEND_DIR")?);

        if !frontend_dir.exists() {
            panic!("FRONTEND_DIR does not exist");
        }

        let assets = AssetManifest::build(&frontend_dir).expect("Asset manifest should be valid");

        Ok(Config {
            server: ServerConfig {
                address: required_env("SERVER_ADDRESS")?,
                https: required_env("HTTPS")? == "1",
            },
            jwt_secret: required_env("JWT_SECRET")?,
            api_url: required_env("API_URL")?,
            frontend_dir,
            captcha_site_key: optional_env("CAPTCHA_SITE_KEY"),
            captcha_api_key: optional_env("CAPTCHA_API_KEY"),
            ga_tag_id: optional_env("GA_TAG_ID"),
            assets,
        })
    }
}

fn required_env(name: &str) -> Result<String> {
    match env::var(name) {
        Ok(val) => {
            if val.is_empty() {
                return Err(Error::Config {
                    msg: format!("{} is required.", name),
                });
            }
            Ok(val)
        }
        Err(_) => Err(Error::Config {
            msg: format!("{} is required.", name),
        }),
    }
}

fn optional_env(name: &str) -> Option<String> {
    match env::var(name) {
        Ok(val) if !val.trim().is_empty() => Some(val),
        _ => None,
    }
}
