use serde_json::{ Value, from_str };
use std::fs::read_to_string;
use std::io::Error;
use std::path::Path;
use std::process::ExitCode;

fn read_package_lock(dir: &str) -> Result<String, Error> {
    read_to_string(Path::new(dir).join("package-lock.json"))
}

fn parse_json(data: String) -> Result<Value, Error> {
    let value: Value = from_str(&data)?;

    Ok(value)
}

fn validate_json(json: &Value) -> Vec<String> {
    json["packages"]
        .as_object()
        .and_then(|packages| {
            Some(
                packages
                    .iter()
                    .filter_map(|(k, v)| {
                        // empty string is self
                        if k.is_empty() {
                            return None;
                        }

                        // if it's not a node module, then it's possible a workspace
                        if !k.starts_with("node_modules") {
                            return None;
                        }

                        // symlink is fine
                        if v["link"] == Value::Bool(true) {
                            return None;
                        }

                        // missing integrity
                        // missing resolved
                        if v["integrity"] == Value::Null || v["resolved"] == Value::Null {
                            return Some(k.to_string());
                        }

                        None
                    })
                    .collect()
            )
        })
        .or_else(|| Some(Vec::new()))
        .unwrap()
}

fn main() -> ExitCode {
    let args = std::env::args().collect::<Vec<String>>();

    let cwd = if args.len() > 1 { &args[1] } else { "." };

    let package_read = read_package_lock(cwd);

    if let Err(_) = package_read {
        let path = Path::new(cwd).join("package-lock.json");

        println!("[ERROR] Could not read package-lock.json at {}", path.display());

        return ExitCode::FAILURE;
    }

    let data = package_read.unwrap();
    let parsed = parse_json(data);

    if let Err(e) = parsed {
        println!("[ERROR] {}", e);

        return ExitCode::FAILURE;
    }

    let v = parsed.unwrap();

    let missing = validate_json(&v);

    if missing.len() > 0 {
        if let Some(name) = &v["name"].as_str() {
            println!("[ERROR] [{}] package-lock.json is missing the following resolved/integrity fields:", name);
        } else {
            println!(
                "[ERROR] package-lock.json is missing the following resolved/integrity fields:"
            );
        }

        missing.iter().for_each(|m| {
            println!("    {}", m);
        });

        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    const LK2: &str = include_str!("./tests/good/lockfile2/package-lock.json");
    const LK2_WORK: &str = include_str!("./tests/good/lockfile2-workspaces/package-lock.json");
    const LK3: &str = include_str!("./tests/good/lockfile3/package-lock.json");
    const LK3_WORK: &str = include_str!("./tests/good/lockfile3-workspaces/package-lock.json");

    const GOOD_LOCKS: &[&str] = &[LK2, LK2_WORK, LK3, LK3_WORK];

    const BLK2: &str = include_str!("./tests/bad/lockfile2/package-lock.json");
    const BLK2_WORK: &str = include_str!("./tests/bad/lockfile2-workspaces/package-lock.json");
    const BLK3: &str = include_str!("./tests/bad/lockfile3/package-lock.json");
    const BLK3_WORK: &str = include_str!("./tests/bad/lockfile3-workspaces/package-lock.json");

    const BAD_LOCKS: &[&str] = &[BLK2, BLK2_WORK, BLK3, BLK3_WORK];

    #[test]
    fn test_good_json() {
        for lockfile in GOOD_LOCKS.iter() {
            let json = parse_json(lockfile.to_string()).unwrap();
            let missing = validate_json(&json);

            assert_eq!(dbg!(missing).len(), 0);
        }
    }

    #[test]
    fn test_bad_json() {
        for (i, lockfile) in BAD_LOCKS.iter().enumerate() {
            let json = parse_json(lockfile.to_string()).unwrap();
            let missing = validate_json(&json);

            assert_ne!(missing.len(), 0, "{}", i);
        }
    }
}
