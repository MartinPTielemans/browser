//! `mcp-bridge <socket>`: what an MCP client runs to reach the browser (src/agents/bridge.rs).

use browser::agents::bridge;

fn main() {
    let Some(socket) = std::env::args_os().nth(1) else {
        eprintln!(
            "usage: {} <socket>, with the agent's token in {}",
            bridge::NAME,
            bridge::TOKEN
        );
        std::process::exit(2);
    };
    let token = std::env::var(bridge::TOKEN).unwrap_or_default();
    let ran = bridge::run(socket.as_ref(), &token, std::io::stdin(), std::io::stdout());
    if let Err(why) = ran {
        eprintln!("{why}");
        std::process::exit(1);
    }
}
