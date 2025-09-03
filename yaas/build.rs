use std::io::Result;

fn main() -> Result<()> {
    prost_build::compile_protos(
        &[
            "src/buffed/actor.proto",
            "src/buffed/org.proto",
            "src/buffed/org_member.proto",
            "src/buffed/org_app.proto",
            "src/buffed/app.proto",
            "src/buffed/user.proto",
            "src/buffed/superuser.proto",
            "src/buffed/password.proto",
            "src/buffed/error.proto",
            "src/buffed/oauth_code.proto",
            "src/buffed/pagination.proto",
            "src/buffed/role.proto",
        ],
        &["src/"],
    )?;
    Ok(())
}
