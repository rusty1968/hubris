fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    build_util::expose_target_board();
    idol::Generator::new()
        .with_counters(
            idol::CounterSettings::default().with_server_counters(false),
        )
        .build_server_support(
            "../../idl/hmac-hash.idol",
            "server_stub.rs",
            idol::server::ServerStyle::InOrder,
        )?;

    Ok(())
}
