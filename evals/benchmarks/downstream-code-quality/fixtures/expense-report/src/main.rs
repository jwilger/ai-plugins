use std::io::{self, Read};
use std::process::ExitCode;

fn parse_record(line: &str) -> Result<(&str, u64), String> {
    let Some((category, cents)) = line.split_once(',') else {
        return Err("record must have category,cents".to_owned());
    };
    if category.is_empty() || cents.contains(',') {
        return Err("record must have one non-empty category and one amount".to_owned());
    }
    let cents = cents
        .parse::<u64>()
        .map_err(|_| "amount must be an unsigned integer".to_owned())?;
    Ok((category, cents))
}

fn run() -> Result<String, String> {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if arguments.as_slice() != ["validate"] {
        return Err("usage: expense-report validate".to_owned());
    }

    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|error| format!("failed to read stdin: {error}"))?;
    let record_count = input
        .lines()
        .map(parse_record)
        .collect::<Result<Vec<_>, _>>()?
        .len();

    Ok(format!("valid,{record_count}\n"))
}

fn main() -> ExitCode {
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
