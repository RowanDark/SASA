mod cli;
mod profiles;
mod scanner;
mod output;

use clap::Parser;
use cli::{Cli, ProfileArg};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use output::ResultWriter;
use scanner::{Scanner, PortStatus, resolve_host};
use tokio::sync::mpsc;

fn print_banner() {
    println!("{}", r#"
  ____    _    ____    _
 / ___|  / \  / ___|  / \
 \___ \ / _ \ \___ \ / _ \
  ___) / ___ \ ___) / ___ \
 |____/_/   \_\____/_/   \_\
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

fn service_name(port: u16) -> &'static str {
    match port {
        20 => "ftp-data", 21 => "ftp", 22 => "ssh", 23 => "telnet",
        25 => "smtp", 53 => "dns", 67 | 68 => "dhcp", 69 => "tftp",
        80 => "http", 110 => "pop3", 111 => "rpcbind", 119 => "nntp",
        123 => "ntp", 135 => "msrpc", 137 | 138 | 139 => "netbios",
        143 => "imap", 161 | 162 => "snmp", 389 => "ldap",
        443 => "https", 445 => "smb", 465 => "smtps", 514 => "syslog",
        515 => "lpd", 587 => "submission", 631 => "ipp", 636 => "ldaps",
        993 => "imaps", 995 => "pop3s", 1080 => "socks", 1194 => "openvpn",
        1433 => "mssql", 1521 => "oracle", 2049 => "nfs",
        2375 | 2376 => "docker", 3306 => "mysql", 3389 => "rdp",
        4444 => "metasploit", 4848 => "glassfish", 5432 => "postgres",
        5900 => "vnc", 5985 | 5986 => "winrm", 6379 => "redis",
        6443 => "k8s-api", 7077 => "spark", 8080 => "http-alt",
        8443 => "https-alt", 8888 => "jupyter", 9000 => "php-fpm",
        9090 => "prometheus", 9200 | 9300 => "elasticsearch",
        27017 | 27018 => "mongodb", 50070 => "hadoop",
        _ => "",
    }
}

fn render_summary_table(results: &[scanner::ScanResult]) {
    if results.is_empty() {
        return;
    }

    const PORT_W: usize = 7;
    const SVC_W: usize = 13;
    const LAT_W: usize = 10;
    const BAN_W: usize = 42;

    let top    = format!("┌{:─<PORT_W$}┬{:─<SVC_W$}┬{:─<LAT_W$}┬{:─<BAN_W$}┐", "", "", "", "");
    let mid    = format!("├{:─<PORT_W$}┼{:─<SVC_W$}┼{:─<LAT_W$}┼{:─<BAN_W$}┤", "", "", "", "");
    let bottom = format!("└{:─<PORT_W$}┴{:─<SVC_W$}┴{:─<LAT_W$}┴{:─<BAN_W$}┘", "", "", "", "");

    println!("\n{}", top.bright_cyan());
    println!("{}",
        format!("│{:>PORT_W$}│{:<SVC_W$}│{:<LAT_W$}│{:<BAN_W$}│",
            " PORT ", " SERVICE     ", " LATENCY  ", " BANNER                                   ")
        .bright_cyan().bold()
    );
    println!("{}", mid.bright_cyan());

    for r in results {
        let svc = service_name(r.port);
        let lat = format!("{}ms", r.latency_ms);
        let banner_raw = r.banner.as_deref().unwrap_or("");
        let banner = if banner_raw.chars().count() > 40 {
            let truncated: String = banner_raw.chars().take(39).collect();
            format!("{}…", truncated)
        } else {
            banner_raw.to_string()
        };
        println!("│{:>PORT_W$}│{:<SVC_W$}│{:<LAT_W$}│{:<BAN_W$}│",
            format!("{:>5} ", r.port),
            format!(" {:<12}", svc),
            format!(" {:<9}", lat),
            format!(" {:<41}", banner),
        );
    }

    println!("{}", bottom.bright_cyan());
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

    let resolved_ip = resolve_host(&cli.host).await?;

    let profile = get_profile(&cli.profile).with_overrides(
        cli.concurrency,
        cli.rate,
        cli.timeout_ms,
        !cli.no_randomize,
    );

    if !cli.quiet {
        let target_display = if resolved_ip != cli.host {
            format!("{} ({})", cli.host, resolved_ip)
        } else {
            cli.host.clone()
        };
        println!("{} {} {} {}",
            "Target:".dimmed(), target_display.white().bold(),
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
    let host = resolved_ip.clone();
    let debug = cli.debug;
    let scan_handle = tokio::spawn(async move {
        tcp.scan_ports(&host, ports, debug, tx).await
    });

    // Collect results
    let mut open_count = 0u32;
    let mut open_results: Vec<scanner::ScanResult> = Vec::new();
    let quiet = cli.quiet;

    while let Some(result) = rx.recv().await {
        pb.inc(1);
        if result.status == PortStatus::Open {
            open_count += 1;
            pb.set_message(format!("{} open", open_count));
            open_results.push(result.clone());
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
        open_results.sort_by_key(|r| r.port);
        render_summary_table(&open_results);

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
