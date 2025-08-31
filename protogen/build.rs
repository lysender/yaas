use std::io::Result;

fn main() -> Result<()> {
    prost_build::compile_protos(&["src/buffed.proto"], &["src/"])?;
    Ok(())
}
