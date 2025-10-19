use askama::Template;
use axum::{Extension, body::Body, extract::State, response::Response};
use snafu::ResultExt;

use crate::{
    Result,
    ctx::Ctx,
    error::{ResponseBuilderSnafu, TemplateSnafu},
    models::TemplateData,
};
use crate::{models::Pref, run::AppState};

#[derive(Template)]
#[template(path = "pages/index.html")]
struct IndexTemplate {
    t: TemplateData,
}

pub async fn index_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(pref): Extension<Pref>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    // if ctx.actor.is_system_admin() {
    //     // Redirect to orgs page
    //     return Ok(Redirect::to("/orgs").into_response());
    // }

    let mut t = TemplateData::new(&state, ctx.actor.clone(), &pref);
    t.title = String::from("Home");

    let tpl = IndexTemplate { t };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}
