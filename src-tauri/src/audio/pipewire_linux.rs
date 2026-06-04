use std::process::Command;

pub fn default_monitor_target() -> Result<String, String> {
    if let Some(default_sink) = command_stdout("pactl", &["get-default-sink"])? {
        let sink = default_sink.trim();
        if !sink.is_empty() {
            let monitor_name = format!("{}.monitor", sink);
            if let Some(serial) = source_serial(&monitor_name)? {
                return Ok(serial);
            }
            return Ok(monitor_name);
        }
    }

    listed_source_names()?
        .into_iter()
        .find(|name| name.ends_with(".monitor"))
        .ok_or_else(|| {
            "No PipeWire/Pulse monitor source found. Make sure an audio output device is available."
                .to_string()
        })
}

pub fn default_input_target() -> Result<String, String> {
    if let Some(default_source) = command_stdout("pactl", &["get-default-source"])? {
        let source = default_source.trim();
        if !source.is_empty() && !source.ends_with(".monitor") {
            if let Some(serial) = source_serial(source)? {
                return Ok(serial);
            }
            return Ok(source.to_string());
        }
    }

    listed_source_names()?
        .into_iter()
        .find(|name| !name.ends_with(".monitor"))
        .ok_or_else(|| {
            "No PipeWire/Pulse microphone source found. Connect a microphone and set a default input device."
                .to_string()
        })
}

fn listed_source_names() -> Result<Vec<String>, String> {
    let sources = command_stdout("pactl", &["list", "short", "sources"])?
        .ok_or("pactl returned no sources".to_string())?;

    Ok(sources
        .lines()
        .filter_map(|line| line.split_whitespace().nth(1))
        .map(str::to_string)
        .collect())
}

fn source_serial(source_name: &str) -> Result<Option<String>, String> {
    let sources = command_stdout("pactl", &["list", "sources"])?
        .ok_or("pactl returned no source details".to_string())?;

    let mut in_target = false;
    for line in sources.lines() {
        let trimmed = line.trim();

        if let Some(name) = trimmed.strip_prefix("Name: ") {
            in_target = name == source_name;
            continue;
        }

        if !in_target {
            continue;
        }

        if let Some(serial) = trimmed.strip_prefix("object.serial = ") {
            return Ok(Some(serial.trim_matches('"').to_string()));
        }
    }

    Ok(None)
}

fn command_stdout(program: &str, args: &[&str]) -> Result<Option<String>, String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| format!("Failed to run {}: {}", program, e))?;

    if !output.status.success() {
        return Ok(None);
    }

    String::from_utf8(output.stdout)
        .map(Some)
        .map_err(|e| format!("{} returned non-UTF8 output: {}", program, e))
}
