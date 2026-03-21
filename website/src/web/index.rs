use askama::Template;
use axum::{Extension, body::Body, extract::State, response::Response};
use snafu::ResultExt;

use crate::{
    Result,
    ctx::Ctx,
    error::{ResponseBuilderSnafu, TemplateSnafu},
    models::{CspNonce, TemplateData},
};
use crate::{models::Pref, run::AppState};

#[derive(Template)]
#[template(path = "pages/index.html")]
struct IndexTemplate {
    t: TemplateData,
    org_id: String,
}

pub async fn index_handler(
    Extension(csp_nonce): Extension<CspNonce>,
    Extension(ctx): Extension<Ctx>,
    Extension(pref): Extension<Pref>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let org_id = ctx.actor().expect("Actor must be present").org_id.clone();

    let mut t = TemplateData::new(&state, ctx.actor.clone(), &pref, csp_nonce.nonce);
    t.title = String::from("Home");

    let tpl = IndexTemplate { t, org_id };

    Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)
}
