use clap::Parser;
use rpkg::DEFAULT_PREFIX;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "rpkg", version, about = "Rin Package Manager")]
pub struct Cli {
    #[arg(long, default_value = DEFAULT_PREFIX)]
    pub prefix: PathBuf,

    #[arg(short = 'S', long)]
    pub sync: bool,

    #[arg(short = 'R', long)]
    pub remove: bool,

    #[arg(short = 'Q', long)]
    pub query: bool,

    #[arg(short = 'y', long)]
    pub refresh: bool,

    #[arg(short = 'u', long)]
    pub sysupgrade: bool,

    #[arg(short = 's', long)]
    pub search: bool,

    #[arg(short = 'f', long)]
    pub force: bool,

    pub targets: Vec<String>,
}
