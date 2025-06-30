// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    build_util::expose_target_board();

    idol::server::build_server_support(
        "../../idl/hmac-hash.idol",
        "server_stub.rs",
        idol::server::ServerStyle::InOrder,
    )?;

    // No notifications for now
    let out = std::env::var("OUT_DIR")?;
    let mut nots = std::fs::File::create(format!("{}/notifications.rs", out))?;
    writeln!(&mut nots, "// no notifications")?;

    Ok(())
}
