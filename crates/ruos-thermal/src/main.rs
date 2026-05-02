//! `ruos-thermal` CLI — prints one snapshot of CPU thermal + cpufreq state.
//!
//! Iter 91 (ADR-174 first deliverable). No daemon mode, no Unix socket,
//! no clock writes. Just: walk sysfs, render output, exit 0.
//!
//! Usage:
//!
//!   ruos-thermal                  # default TSV output
//!   ruos-thermal --json           # NDJSON line for piped consumers
//!   ruos-thermal --prom           # Prometheus textfile-collector format
//!   ruos-thermal --version, -V    # print pkg-name + semver
//!   ruos-thermal --help, -h       # print this help and exit

use ruos_thermal::ThermalSensor;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut json = false;
    let mut prom = false;

    for a in &args {
        match a.as_str() {
            "--json" => json = true,
            "--prom" => prom = true,
            "--version" | "-V" => {
                println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
                return ExitCode::SUCCESS;
            }
            "--help" | "-h" => {
                print_help();
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("ruos-thermal: unknown arg: {}", other);
                eprintln!("(try --help)");
                return ExitCode::from(1);
            }
        }
    }
    if json && prom {
        eprintln!("ruos-thermal: --json and --prom are mutually exclusive");
        return ExitCode::from(1);
    }

    let sensor = ThermalSensor::system();
    let snap = match sensor.read() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("ruos-thermal: read failed: {}", e);
            return ExitCode::from(2);
        }
    };

    if json {
        // Hand-rolled JSON to stay zero-dep for the skeleton.
        print!("{{\"cpu_temps\":[");
        for (i, t) in snap.cpu_temps_celsius.iter().enumerate() {
            if i > 0 { print!(","); }
            print!("{{\"zone\":{},\"celsius\":{:.3}}}", t.zone, t.celsius);
        }
        print!("],\"cpu_policies\":[");
        for (i, p) in snap.cpu_policies.iter().enumerate() {
            if i > 0 { print!(","); }
            print!(
                "{{\"id\":{},\"cur_hz\":{},\"max_hz\":{},\"hw_max_hz\":{},\"governor\":{:?}}}",
                p.id, p.cur_hz, p.max_hz, p.hw_max_hz, p.governor
            );
        }
        print!("]");
        if let Some(m) = snap.max_cpu_celsius() {
            print!(",\"max_cpu_celsius\":{:.3}", m);
        }
        if let Some(m) = snap.mean_cpu_celsius() {
            print!(",\"mean_cpu_celsius\":{:.3}", m);
        }
        println!("}}");
    } else if prom {
        println!("# HELP ruos_thermal_cpu_temp_celsius Per-zone CPU temperature.");
        println!("# TYPE ruos_thermal_cpu_temp_celsius gauge");
        for t in &snap.cpu_temps_celsius {
            println!("ruos_thermal_cpu_temp_celsius{{zone=\"{}\"}} {:.3}", t.zone, t.celsius);
        }
        println!("# HELP ruos_thermal_cpu_freq_hz Per-policy current CPU frequency.");
        println!("# TYPE ruos_thermal_cpu_freq_hz gauge");
        for p in &snap.cpu_policies {
            println!("ruos_thermal_cpu_freq_hz{{policy=\"{}\"}} {}", p.id, p.cur_hz);
        }
        println!("# HELP ruos_thermal_cpu_max_freq_hz Per-policy configured max frequency.");
        println!("# TYPE ruos_thermal_cpu_max_freq_hz gauge");
        for p in &snap.cpu_policies {
            println!(
                "ruos_thermal_cpu_max_freq_hz{{policy=\"{}\",governor=\"{}\"}} {}",
                p.id, p.governor, p.max_hz
            );
        }
    } else {
        // Default TSV: one row per zone, one row per policy.
        println!("kind\tindex\tvalue\tunit\textra");
        for t in &snap.cpu_temps_celsius {
            println!("temp\t{}\t{:.3}\tcelsius\tzone", t.zone, t.celsius);
        }
        for p in &snap.cpu_policies {
            println!("freq\t{}\t{}\thz\tcur (max={} hw={} gov={})",
                     p.id, p.cur_hz, p.max_hz, p.hw_max_hz, p.governor);
        }
        if let Some(m) = snap.max_cpu_celsius() {
            println!("# max cpu temp: {:.1}°C", m);
        }
        if let Some(m) = snap.mean_cpu_celsius() {
            println!("# mean cpu temp: {:.1}°C", m);
        }
    }

    ExitCode::SUCCESS
}

fn print_help() {
    eprintln!(
        "ruos-thermal — Pi 5 thermal + cpufreq snapshot (ADR-174)

USAGE:
    ruos-thermal [OPTIONS]

OUTPUT (one of):
    (default)    TSV with one row per thermal zone + one per cpufreq policy
    --json       single NDJSON line — pipe into jq / log shippers
    --prom       Prometheus textfile-collector format (gauges + HELP/TYPE)

OPTIONS:
    --version, -V  Print pkg-name + semver, exit.
    --help, -h     Print this help and exit.

EXIT CODES:
    0   snapshot rendered cleanly
    1   bad CLI args
    2   sysfs read error (missing roots, permission denied, etc.)

This is the iter-91 read-only skeleton (ADR-174). The clock-write +
Unix-socket budget protocol + workload subscribers land iters 92-97.
"
    );
}
