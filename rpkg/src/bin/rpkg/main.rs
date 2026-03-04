mod cli;
mod proxy;

use clap::Parser;
use cli::Cli;
use colored::Colorize;
use rpkg::manager::PackageManager;

#[allow(dead_code)]
fn format_size(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = KIB * 1024;
    const GIB: u64 = MIB * 1024;

    if bytes >= GIB {
        format!("{:.1} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.1} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.1} KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .format_timestamp(None)
        .init();

    proxy::handle_multicall();

    let cli = Cli::parse();
    let mut pm = PackageManager::new(&cli.prefix)?;

    if cli.sync {
        if cli.refresh {
            pm.sync()?;
        }

        if cli.search {
            for query in &cli.targets {
                let results = pm.search(query)?;
                if results.is_empty() {
                    continue;
                }

                for pkg in &results {
                    let name_styled = pkg.name.bold();
                    let ver_styled = pkg.version.green().bold();
                    let installed_tag =
                        if pm.list_installed().iter().any(|i| i.info.name == pkg.name) {
                            " [installed]".cyan().bold()
                        } else {
                            "".normal()
                        };

                    println!("rin/{} {} {}", name_styled, ver_styled, installed_tag);
                    println!("    {}", pkg.description);
                }
            }
            return Ok(());
        }

        if cli.sysupgrade {
            pm.upgrade()?;
        }

        if !cli.targets.is_empty() {
            pm.install(&cli.targets, cli.force)?;
        }
    } else if cli.remove {
        if !cli.targets.is_empty() {
            pm.remove(&cli.targets)?;
        }
    } else if cli.query {
        let installed = pm.list_installed();
        for pkg in installed {
            let name_styled = pkg.info.name.bold();
            let ver_styled = pkg.info.version.green().bold();
            println!("{} {}", name_styled, ver_styled);
        }
    } else {
        println!(
            "{}",
            "error: no operation specified (use -h for help)"
                .red()
                .bold()
        );
    }

    Ok(())
}
