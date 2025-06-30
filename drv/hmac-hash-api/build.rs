fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    idol::client::build_client_stub("../../idl/hmac-hash.idol", "client_stub.rs")?;
    Ok(())
}
