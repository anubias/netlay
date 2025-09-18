mod cmdline;
mod config;

fn main() {
    let args = cmdline::Args::parse();
    let config = config::Config::load_config(&args.config_file);

    dbg!(config);
}
