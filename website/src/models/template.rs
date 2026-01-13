use crate::{config::AssetManifest, run::AppState};

use super::Pref;
use yaas::dto::Actor;

#[derive(Clone)]
pub struct TemplateData {
    pub theme: String,
    pub title: String,
    pub assets: AssetManifest,
    pub styles: Vec<String>,
    pub scripts: Vec<String>,
    pub async_scripts: Vec<String>,
    pub script_vars: Vec<String>,
    pub ga_tag_id: Option<String>,
    pub actor: Actor,
    pub is_system_admin: bool,
}

impl TemplateData {
    pub fn new(state: &AppState, actor: Actor, pref: &Pref) -> TemplateData {
        let config = state.config.clone();
        let assets = config.assets.clone();
        let is_system_admin = actor.is_system_admin();

        TemplateData {
            theme: pref.theme.clone(),
            title: String::from(""),
            assets,
            styles: Vec::new(),
            scripts: Vec::new(),
            async_scripts: Vec::new(),
            script_vars: Vec::new(),
            ga_tag_id: config.ga_tag_id.clone(),
            actor,
            is_system_admin,
        }
    }
}
