use std::io::Result;

fn main() -> Result<()> {
    prost_build::compile_protos(
        &[
            "src/buffed/actor.proto",
            "src/buffed/dto.proto",
            "src/buffed/pagination.proto",
            "src/buffed/role.proto",
        ],
        &["src/"],
    )?;
    Ok(())
}
