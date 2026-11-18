//! `falco-core bench`: misura RAM, CPU e processi di Falco già in esecuzione (§10).
//! Uso: `falco-core bench [secondi] [etichetta-stato]` → JSON su stdout.

use std::time::Duration;

use serde_json::json;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

const NAMES: &[&str] = &["falco-core", "falco-ui", "sunshine", "moonlight"];

fn short(name: &str) -> String {
    name.trim_end_matches(".exe").to_lowercase()
}

pub fn run(args: &[String]) -> anyhow::Result<()> {
    let seconds: u64 = args.first().and_then(|s| s.parse().ok()).unwrap_or(60);
    let label = args.get(1).cloned().unwrap_or_else(|| "unspecified".into());
    let me = sysinfo::get_current_pid().ok();
    let mut sys = System::new();
    let refresh = ProcessRefreshKind::nothing().with_memory().with_cpu();

    let mut core_rss_max = 0u64;
    let mut core_cpu_sum = 0f32;
    let mut total_rss_max = 0u64;
    let mut samples = 0u32;
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, refresh);

    for _ in 0..seconds {
        std::thread::sleep(Duration::from_secs(1));
        sys.refresh_processes_specifics(ProcessesToUpdate::All, true, refresh);
        let mut total = 0u64;
        let procs = sys.processes();
        let name_of =
            |pid: &sysinfo::Pid| procs.get(pid).map(|p| short(&p.name().to_string_lossy()));
        // WebView2 conta solo se discende da falco-ui (altre app usano la stessa webview)
        let from_ui = |mut pid: Option<sysinfo::Pid>| {
            for _ in 0..4 {
                match pid {
                    Some(p) if name_of(&p).as_deref() == Some("falco-ui") => return true,
                    Some(p) => pid = procs.get(&p).and_then(|x| x.parent()),
                    None => return false,
                }
            }
            false
        };
        for (pid, p) in procs {
            if Some(*pid) == me {
                continue;
            }
            let name = short(&p.name().to_string_lossy());
            let webview = name == "msedgewebview2" && from_ui(p.parent());
            if !NAMES.contains(&name.as_str()) && !webview {
                continue;
            }
            total += p.memory();
            if name == "falco-core" {
                core_rss_max = core_rss_max.max(p.memory());
                core_cpu_sum += p.cpu_usage();
            }
        }
        total_rss_max = total_rss_max.max(total);
        samples += 1;
    }

    let mb = |b: u64| (b as f64 / 1_048_576.0 * 10.0).round() / 10.0;
    let report = json!({
        "state": label,
        "seconds": seconds,
        "core": {
            "rssMaxMb": mb(core_rss_max),
            "cpuAvgPercent": if samples > 0 { core_cpu_sum / samples as f32 } else { 0.0 },
        },
        "allFalcoProcessesRssMaxMb": mb(total_rss_max),
        "budget": { "coreRssMb": 15, "trayCpuPercent": 0.5 },
    });
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
