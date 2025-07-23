// build.rs

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // tonic_build'a proto dosyasının yerini söylüyoruz
    tonic_build::compile_protos("proto/core.proto")?;
    Ok(())
}