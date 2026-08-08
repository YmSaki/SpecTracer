fn main() {
    let cli = <vtest_cli::Cli as clap::Parser>::parse();
    std::process::exit(vtest_cli::run(cli) as i32);
}
