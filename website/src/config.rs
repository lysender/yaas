use serde::Deserialize;
use snafu::ResultExt;
use std::path::{Path, PathBuf};
use std::{env, fs};

use crate::Result;
use crate::error::{ManifestParseSnafu, ManifestReadSnafu};

#[derive(Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub jwt_secret: String,
    pub api_url: String,
    pub frontend_dir: PathBuf,
    pub captcha_site_key: String,
    pub captcha_api_key: String,
    pub ga_tag_id: Option<String>,
    pub assets: AssetManifest,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub port: u16,
    pub https: bool,
}

#[derive(Clone, Deserialize)]
pub struct AssetManifest {
    pub main_js: String,
    pub gallery_js: String,
    pub upload_js: String,
    pub main_css: String,
    pub gallery_css: String,
}

#[derive(Deserialize)]
struct BundleConfig {
    suffix: String,
}

impl Config {
    pub fn build() -> Self {
        // Build the config from ENV vars
        let jwt_secret = env::var("JWT_SECRET").expect("JWT_SECRET is required");
        let port = env::var("PORT")
            .expect("PORT is required")
            .parse::<u16>()
            .expect("PORT must be a valid u16");

        let mut https = false;
        if let Ok(https_str) = env::var("HTTPS") {
            https = &https_str == "1"
        }

        let api_url = env::var("API_URL").expect("API_URL is required");
        let frontend_dir: PathBuf = env::var("FRONTEND_DIR")
            .expect("FRONTEND_DIR is required")
            .into();

        let captcha_site_key = env::var("CAPTCHA_SITE_KEY").expect("CAPTCHA_SITE_KEY is required");
        let captcha_api_key = env::var("CAPTCHA_API_KEY").expect("CAPTCHA_API_KEY is required");

        let ga_tag_id = match env::var("GA_TAG_ID") {
            Ok(val) => {
                if !val.is_empty() {
                    Some(val)
                } else {
                    None
                }
            }
            Err(_) => None,
        };

        // Validate config values
        if api_url.is_empty() {
            panic!("API_URL is required");
        }

        if port == 0 {
            panic!("PORT is required");
        }

        if jwt_secret.is_empty() {
            panic!("JWT_SECRET is required");
        }

        if !frontend_dir.exists() {
            panic!("FRONTEND_DIR does not exist");
        }

        if captcha_site_key.is_empty() {
            panic!("CAPTCHA_SITE_KEY is required");
        }

        if captcha_api_key.is_empty() {
            panic!("CAPTCHA_API_KEY is required");
        }

        let assets = AssetManifest::build(&frontend_dir).expect("Asset manifest should be valid");

        Config {
            server: ServerConfig { port, https },
            jwt_secret,
            api_url,
            frontend_dir,
            captcha_site_key,
            captcha_api_key,
            ga_tag_id,
            assets,
        }
    }
}

impl AssetManifest {
    pub fn build(frontend_dir: &PathBuf) -> Result<Self> {
        let filename = Path::new(frontend_dir).join("bundles.json");
        let contents = fs::read_to_string(filename).context(ManifestReadSnafu)?;
        let config =
            serde_json::from_str::<BundleConfig>(contents.as_str()).context(ManifestParseSnafu)?;

        Ok(AssetManifest {
            main_js: format!("/assets/bundles/js/main-{}.js", config.suffix),
            gallery_js: format!("/assets/bundles/js/gallery-{}.js", config.suffix),
            upload_js: format!("/assets/bundles/js/upload-{}.js", config.suffix),
            main_css: format!("/assets/bundles/css/main-{}.css", config.suffix),
            gallery_css: format!("/assets/bundles/css/gallery-{}.css", config.suffix),
        })
    }
}
