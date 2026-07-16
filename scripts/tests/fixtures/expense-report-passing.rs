#[cfg(not(known_bad_adjacent))]
use std::collections::BTreeMap;
use std::io::{self, Read};
#[cfg(host_escape_probe)]
use std::net::TcpStream;
#[cfg(nix_store_source_probe)]
use std::path::Path;
use std::process::ExitCode;

const MAXIMUM: u128 = u64::MAX as u128;

fn records(input: &str) -> Result<Vec<(&str, u128)>, String> {
    if input.is_empty() {
        return Ok(Vec::new());
    }
    input
        .lines()
        .map(|line| {
            let parts = line.split(',').collect::<Vec<_>>();
            if parts.len() != 2 || parts[0].is_empty() {
                return Err("invalid record".to_owned());
            }
            let cents = parts[1]
                .parse::<u128>()
                .map_err(|_| "invalid amount".to_owned())?;
            if cents > MAXIMUM {
                return Err("amount overflow".to_owned());
            }
            Ok((parts[0], cents))
        })
        .collect()
}

#[cfg(not(known_bad_adjacent))]
fn aggregate_records(input: &str) -> Result<Vec<(&str, u128)>, String> {
    let mut aggregate = BTreeMap::new();
    for (category, cents) in records(input)? {
        let next = aggregate.get(category).copied().unwrap_or(0) + cents;
        if next > MAXIMUM {
            return Err("aggregate overflow".to_owned());
        }
        aggregate.insert(category, next);
    }
    let totals = aggregate.into_iter().collect::<Vec<_>>();
    #[cfg(known_bad_total_order)]
    let totals = {
        let mut totals = totals;
        totals.sort_by_key(|(_, cents)| *cents);
        totals
    };
    Ok(totals)
}

#[cfg(known_bad_adjacent)]
fn aggregate_records(input: &str) -> Result<Vec<(&str, u128)>, String> {
    let mut runs: Vec<(&str, u128)> = Vec::new();
    for (category, cents) in records(input)? {
        if let Some((previous_category, previous_cents)) = runs.last_mut()
            && *previous_category == category
        {
            *previous_cents += cents;
            if *previous_cents > MAXIMUM {
                return Err("aggregate overflow".to_owned());
            }
        } else {
            runs.push((category, cents));
        }
    }
    runs.sort_by_key(|(category, _)| *category);
    Ok(runs)
}

fn totals(input: &str, arguments: &[String]) -> Result<String, String> {
    let minimum = match arguments {
        [] => 0,
        [flag, value] if flag == "--minimum-cents" => {
            if value.starts_with('0') || !value.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err("invalid minimum".to_owned());
            }
            #[cfg(not(known_bad_u8_minimum))]
            let minimum = value
                .parse::<u128>()
                .map_err(|_| "invalid minimum".to_owned())?;
            #[cfg(known_bad_u8_minimum)]
            let minimum = value
                .parse::<u8>()
                .map(u128::from)
                .map_err(|_| "invalid minimum".to_owned())?;
            if minimum == 0 || minimum > MAXIMUM {
                return Err("invalid minimum".to_owned());
            }
            minimum
        }
        _ => return Err("invalid minimum arguments".to_owned()),
    };

    let lines = aggregate_records(input)?
        .into_iter()
        .filter(|(_, cents)| *cents >= minimum)
        .map(|(category, cents)| format!("{category},{cents}"))
        .collect::<Vec<_>>();
    if lines.is_empty() {
        Ok(String::new())
    } else {
        Ok(format!("{}\n", lines.join("\n")))
    }
}

fn run() -> Result<String, String> {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|error| error.to_string())?;
    match arguments.as_slice() {
        [command] if command == "validate" => Ok(format!("valid,{}\n", records(&input)?.len())),
        [command, rest @ ..] if command == "totals" => totals(&input, rest),
        _ => Err("invalid command".to_owned()),
    }
}

#[cfg(host_escape_probe)]
fn host_escape_violations() -> Vec<String> {
    let mut violations = Vec::new();
    if let Ok(secret) = std::fs::read_to_string(env!("EXPENSE_REPORT_TEST_SECRET_PATH")) {
        violations.push(format!("read-host:{secret}"));
    }
    if std::fs::write(env!("EXPENSE_REPORT_TEST_MUTATION_PATH"), "escaped\n").is_ok() {
        violations.push("wrote-host".to_owned());
    }
    if TcpStream::connect(env!("EXPENSE_REPORT_TEST_NETWORK_ADDRESS")).is_ok() {
        violations.push("reached-host-network".to_owned());
    }
    let limits = std::fs::read_to_string("/proc/self/limits").unwrap_or_default();
    for (name, expected) in [
        ("Max cpu time", "3"),
        ("Max file size", "1048576"),
        ("Max core file size", "0"),
        ("Max processes", "8"),
        ("Max open files", "64"),
        ("Max address space", "134217728"),
    ] {
        let observed = limits
            .lines()
            .find(|line| line.starts_with(name))
            .and_then(|line| {
                let fields = line.split_whitespace().collect::<Vec<_>>();
                fields.get(fields.len().checked_sub(3)?).copied()
            })
            .unwrap_or("missing");
        if observed != expected {
            violations.push(format!("resource-limit:{name}={observed}"));
        }
    }
    violations
}

#[cfg(nix_store_source_probe)]
fn unlisted_nix_source_is_visible() -> bool {
    Path::new(env!("EXPENSE_REPORT_TEST_NIX_SOURCE_PATH")).exists()
}

#[cfg(sandbox_root_probe)]
fn sandbox_root_exceeds_scratch_limit() -> bool {
    let payload = vec![b'x'; 1024 * 1024];
    for directory in ["", "/dev"] {
        let mut bytes_written = 0;
        for index in 0..20 {
            let path = format!("{directory}/benchmark-root-probe-{index}");
            if std::fs::write(path, &payload).is_err() {
                break;
            }
            bytes_written += payload.len();
        }
        if bytes_written > 16 * 1024 * 1024 {
            return true;
        }
    }
    false
}

fn main() -> ExitCode {
    #[cfg(stderr_secret_probe)]
    eprintln!("ghp_FAKE_BENCHMARK_SECRET_DO_NOT_PERSIST");

    #[cfg(host_escape_probe)]
    {
        let violations = host_escape_violations();
        if !violations.is_empty() {
            eprintln!("{}", violations.join("|"));
            return ExitCode::FAILURE;
        }
    }

    #[cfg(nix_store_source_probe)]
    if unlisted_nix_source_is_visible() {
        eprintln!("unlisted-nix-source-visible");
        return ExitCode::FAILURE;
    }

    #[cfg(sandbox_root_probe)]
    if sandbox_root_exceeds_scratch_limit() {
        return ExitCode::FAILURE;
    }

    match run() {
        Ok(output) => {
            print!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
