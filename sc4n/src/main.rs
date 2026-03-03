mod cli;
mod profiles;
mod scanner;
mod output;

use clap::Parser;
use cli::{Cli, ProfileArg};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use output::ResultWriter;
use scanner::{Scanner, PortStatus};
use tokio::sync::mpsc;

fn print_banner() {
    println!("{}", r#"
  ___  ___ _  _ _ __
 / __|/ __| || | '_ \
 \__ \ (__| || | | | |
 |___/\___|\_,_|_| |_|
"#.bright_red().bold());
    println!("{}", "  Stealthy Adaptive Network Scanner".dimmed());
    println!("{}", "  © 2025 RowanDark\n".dimmed());
}

fn parse_ports(ports_str: &str) -> anyhow::Result<Vec<u16>> {
    let mut ports = Vec::new();
    for part in ports_str.split(',') {
        let part = part.trim();
        if part.contains('-') {
            let bounds: Vec<&str> = part.splitn(2, '-').collect();
            if bounds.len() != 2 {
                anyhow::bail!("Invalid port range: {}", part);
            }
            let start: u16 = bounds[0].parse()
                .map_err(|_| anyhow::anyhow!("Invalid port: {}", bounds[0]))?;
            let end: u16 = bounds[1].parse()
                .map_err(|_| anyhow::anyhow!("Invalid port: {}", bounds[1]))?;
            if start > end {
                anyhow::bail!("Port range start must be <= end: {}", part);
            }
            ports.extend(start..=end);
        } else {
            let port: u16 = part.parse()
                .map_err(|_| anyhow::anyhow!("Invalid port: {}", part))?;
            ports.push(port);
        }
    }
    Ok(ports)
}

fn get_profile(arg: &ProfileArg) -> profiles::ScanProfile {
    match arg {
        ProfileArg::Aggressive => profiles::builtin::aggressive(),
        ProfileArg::Balanced   => profiles::builtin::balanced(),
        ProfileArg::Stealth    => profiles::builtin::stealth(),
        ProfileArg::Paranoid   => profiles::builtin::paranoid(),
    }
}

fn print_result(result: &scanner::ScanResult) {
    match result.status {
        PortStatus::Open => println!(
            "  {} {} {}  {} {}",
            "▶".green().bold(),
            result.host.white(),
            format!(":{}", result.port).bright_green().bold(),
            "open".green(),
            format!("({}ms)", result.latency_ms).dimmed()
        ),
        PortStatus::Closed => println!(
            "  {} {} {}  {}",
            "·".dimmed(),
            result.host.dimmed(),
            format!(":{}", result.port).dimmed(),
            "closed".dimmed()
        ),
        PortStatus::Filtered => println!(
            "  {} {} {}  {}",
            "?".yellow(),
            result.host.dimmed(),
            format!(":{}", result.port).yellow(),
            "filtered".yellow()
        ),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if !cli.quiet {
        print_banner();
    }

    let ports = parse_ports(&cli.ports)?;
    let total_ports = ports.len();

    let profile = get_profile(&cli.profile).with_overrides(
        cli.concurrency,
        cli.rate,
        cli.timeout_ms,
        !cli.no_randomize,
    );

    if !cli.quiet {
        println!("{} {} {} {}",
            "Target:".dimmed(), cli.host.white().bold(),
            "│ Profile:".dimmed(), profile.name.bright_cyan()
        );
        println!("{} {} {} {}",
            "Ports:".dimmed(), total_ports.to_string().white(),
            "│ Concurrency:".dimmed(),
            profile.concurrency.to_string().white()
        );
        println!("{} {}\n",
            "Output:".dimmed(), cli.output.white()
        );
    }

    let writer = ResultWriter::new(&cli.output)?;
    let scanner = Scanner::new(profile);

    let (tx, mut rx) = mpsc::channel::<scanner::ScanResult>(1000);

    // Progress bar
    let pb = ProgressBar::new(total_ports as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.cyan} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("█▓░")
    );

    // Spawn scanner task
    let tcp = scanner.tcp();
    let host = cli.host.clone();
    let debug = cli.debug;
    let scan_handle = tokio::spawn(async move {
        tcp.scan_ports(&host, ports, debug, tx).await
    });

    // Collect results
    let mut open_count = 0u32;
    let quiet = cli.quiet;

    while let Some(result) = rx.recv().await {
        pb.inc(1);
        if result.status == PortStatus::Open {
            open_count += 1;
            pb.set_message(format!("{} open", open_count));
        }
        if !quiet || result.status == PortStatus::Open {
            pb.suspend(|| print_result(&result));
        }
        if let Err(e) = writer.write(&result) {
            pb.suspend(|| {
                eprintln!("{} Failed to write result: {}", "✗".red(), e)
            });
        }
    }

    scan_handle.await??;
    pb.finish_with_message(format!("Done. {} open ports found.", open_count));

    if !cli.quiet {
        println!("\n{} {} open port(s) found on {}",
            "✓".green().bold(),
            open_count.to_string().bright_green().bold(),
            cli.host.white()
        );
        println!("{} Results saved to {}",
            "✓".green().bold(),
            cli.output.bright_cyan()
        );
    }

    Ok(())
}
