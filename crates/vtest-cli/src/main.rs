//! The `vtest` binary.
//!
//! SPEC-403: `vtest mcp` starts the MCP server over stdio. `vtest-mcp`
//! depends on this crate's library (it calls the same `ops::*` the CLI
//! does, DS-1563), so this package cannot depend on `vtest-mcp` without a
//! cycle. The binary therefore compiles the MCP transport module from its
//! source file directly: the library crate graph stays acyclic and the
//! `vtest-mcp` crate stays the single place that code lives.
#[allow(dead_code, unused_imports, clippy::all)]
#[path = "../../vtest-mcp/src/lib.rs"]
mod mcp;

fn main() {
    let cli = <vtest_cli::Cli as clap::Parser>::parse();
    if matches!(cli.command, vtest_cli::Command::Mcp) {
        let code = match vtest_cli::project_root(&cli) {
            Ok(root) => match mcp::serve(&root) {
                Ok(()) => vtest_model::ExitCode::Ok,
                Err(error) => {
                    eprintln!("vtest mcp: {error}");
                    vtest_model::ExitCode::Internal
                }
            },
            Err(code) => code,
        };
        std::process::exit(code as i32);
    }
    std::process::exit(vtest_cli::run(cli) as i32);
}
