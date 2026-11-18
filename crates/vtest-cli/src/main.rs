//! The `vtest` binary: parse the arguments, run the command, exit.
//!
//! SPEC-403: `vtest mcp` starts the MCP server over stdio. This binary is
//! the one place that depends on both entrypoint crates; `vtest-cli` and
//! `vtest-mcp` depend on `vtest-ops` and not on each other (#175 D2).

fn main() {
    let cli = match <vtest_cli::Cli as clap::Parser>::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            // DS-1706: `vtest export --quiet` prints nothing even when its
            // arguments are rejected; the exit code stays 2.
            let arguments: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
            if error.use_stderr() && vtest_cli::is_quiet_export(&arguments) {
                std::process::exit(vtest_model::ExitCode::Usage as i32);
            }
            error.exit()
        }
    };
    if matches!(cli.command, vtest_cli::Command::Mcp) {
        let code = match vtest_cli::project_root(&cli) {
            Ok(root) => match vtest_mcp::serve(&root) {
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
