use clap::Parser;

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(
        default_value = "/etc/netlay.conf",
        short,
        long,
        help = "Path to the configuration file"
    )]
    pub config_file: String,
}

impl Args {
    pub fn parse() -> Self {
        clap::Parser::parse()
    }
}
