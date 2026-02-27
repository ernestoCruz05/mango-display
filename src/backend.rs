use regex::Regex;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq)]
pub struct OutputMode {
    pub width: i32,
    pub height: i32,
    pub refresh_rate: f32,
    pub current: bool,
    pub preferred: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Output {
    pub name: String,
    pub description: String,
    pub make: String,
    pub model: String,
    pub serial: String,
    pub physical_size: String,
    pub position: (i32, i32),
    pub scale: f32,
    pub transform: String,
    pub modes: Vec<OutputMode>,
    pub enabled: bool,
}

pub fn wlr_randr_get_outputs() -> Result<Vec<Output>, String> {
    let output = Command::new("wlr-randr")
        .output()
        .map_err(|e| format!("Failed to run wlr-randr: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_wlr_randr_output(&stdout)
}

pub fn parse_wlr_randr_output(output_str: &str) -> Result<Vec<Output>, String> {
    let mut outputs = Vec::new();
    let lines: Vec<&str> = output_str.lines().collect();

    let mut current_output: Option<Output> = None;
    let name_desc_regex = Regex::new(r#"^([^\s]+)\s+"(.*)""#).unwrap();
    let pos_regex = Regex::new(r#"^  Position:\s+(-?\d+),(-?\d+)"#).unwrap();
    let scale_regex = Regex::new(r#"^  Scale:\s+([0-9.]+)"#).unwrap();
    let transform_regex = Regex::new(r#"^  Transform:\s+(.*)"#).unwrap();
    let mode_regex = Regex::new(r#"^\s+(\d+)x(\d+) px, ([0-9.]+) Hz(?: \((.*)\))?"#).unwrap();
    let make_regex = Regex::new(r#"^  Make:\s+(.*)"#).unwrap();
    let model_regex = Regex::new(r#"^  Model:\s+(.*)"#).unwrap();
    let serial_regex = Regex::new(r#"^  Serial:\s+(.*)"#).unwrap();
    let phys_size_regex = Regex::new(r#"^  Physical size:\s+(.*)"#).unwrap();
    let enabled_regex = Regex::new(r#"^  Enabled:\s+(yes|no)"#).unwrap();

    let mut parsing_modes = false;

    for line in lines {
        if !line.starts_with(' ') {
            if let Some(out) = current_output.take() {
                if out.modes.is_empty() {
                    // huh
                }
                outputs.push(out);
            }
            if let Some(caps) = name_desc_regex.captures(line) {
                current_output = Some(Output {
                    name: caps.get(1).map_or("", |m| m.as_str()).to_string(),
                    description: caps.get(2).map_or("", |m| m.as_str()).to_string(),
                    make: String::new(),
                    model: String::new(),
                    serial: String::new(),
                    physical_size: String::new(),
                    position: (0, 0),
                    scale: 1.0,
                    transform: "normal".to_string(),
                    modes: Vec::new(),
                    enabled: true,
                });
                parsing_modes = false;
            }
        } else if let Some(out) = current_output.as_mut() {
            if let Some(caps) = enabled_regex.captures(line) {
                out.enabled = caps.get(1).unwrap().as_str() == "yes";
            } else if let Some(caps) = make_regex.captures(line) {
                out.make = caps.get(1).unwrap().as_str().to_string();
            } else if let Some(caps) = model_regex.captures(line) {
                out.model = caps.get(1).unwrap().as_str().to_string();
            } else if let Some(caps) = serial_regex.captures(line) {
                out.serial = caps.get(1).unwrap().as_str().to_string();
            } else if let Some(caps) = phys_size_regex.captures(line) {
                out.physical_size = caps.get(1).unwrap().as_str().to_string();
            } else if let Some(caps) = pos_regex.captures(line) {
                let x = i32::from_str(caps.get(1).unwrap().as_str()).unwrap_or(0);
                let y = i32::from_str(caps.get(2).unwrap().as_str()).unwrap_or(0);
                out.position = (x, y);
            } else if let Some(caps) = scale_regex.captures(line) {
                out.scale = f32::from_str(caps.get(1).unwrap().as_str()).unwrap_or(1.0);
            } else if let Some(caps) = transform_regex.captures(line) {
                out.transform = caps.get(1).unwrap().as_str().to_string();
            } else if line.trim() == "Modes:" {
                parsing_modes = true;
            } else if parsing_modes {
                if let Some(caps) = mode_regex.captures(line) {
                    let w = i32::from_str(caps.get(1).unwrap().as_str()).unwrap_or(0);
                    let h = i32::from_str(caps.get(2).unwrap().as_str()).unwrap_or(0);
                    let freq = f32::from_str(caps.get(3).unwrap().as_str()).unwrap_or(0.0);

                    let mut current = false;
                    let mut preferred = false;

                    if let Some(flags) = caps.get(4) {
                        let f_str = flags.as_str();
                        if f_str.contains("current") {
                            current = true;
                        }
                        if f_str.contains("preferred") {
                            preferred = true;
                        }
                    }

                    out.modes.push(OutputMode {
                        width: w,
                        height: h,
                        refresh_rate: freq,
                        current,
                        preferred,
                    });
                }
            }
        }
    }
    if let Some(out) = current_output.take() {
        outputs.push(out);
    }

    Ok(outputs)
}

pub fn wlr_randr_apply(outputs: &[Output]) -> Result<(), String> {
    let mut cmd = Command::new("wlr-randr");

    for out in outputs {
        cmd.arg("--output").arg(&out.name);
        if out.enabled {
            cmd.arg("--on");
            cmd.arg("--pos")
                .arg(format!("{},{}", out.position.0, out.position.1));
            cmd.arg("--scale").arg(format!("{:.6}", out.scale));
            cmd.arg("--transform").arg(&out.transform);

            if let Some(current_mode) = out.modes.iter().find(|m| m.current) {
                cmd.arg("--mode").arg(format!(
                    "{}x{}@{:.3}",
                    current_mode.width, current_mode.height, current_mode.refresh_rate
                ));
            }
        } else {
            cmd.arg("--off");
        }
    }

    let status = cmd
        .status()
        .map_err(|e| format!("Failed to run wlr-randr: {}", e))?;
    if !status.success() {
        return Err("wlr-randr exited with non-zero status".to_string());
    }
    Ok(())
}

pub fn wlr_randr_save(
    outputs: &[Output],
    settings: &crate::settings::AppSettings,
) -> Result<(), String> {
    let mut script = String::from("# Generated by mango-display\n\n");

    for out in outputs {
        if out.enabled {
            let rr = match out.transform.as_str() {
                "normal" => 0,
                "90" => 1,
                "180" => 2,
                "270" => 3,
                "flipped" => 4,
                "flipped-90" => 5,
                "flipped-180" => 6,
                "flipped-270" => 7,
                _ => 0,
            };

            let mut w = 0;
            let mut h = 0;
            let mut r = 0.0;

            if let Some(current_mode) = out.modes.iter().find(|m| m.current) {
                w = current_mode.width;
                h = current_mode.height;
                r = current_mode.refresh_rate;
            }

            script.push_str(&format!(
                "monitorrule=name:{},width:{},height:{},refresh:{:.6},x:{},y:{},scale:{:.6},rr:{}\n",
                out.name, w, h, r, out.position.0, out.position.1, out.scale, rr
            ));
        } else {
            // This is just a placeholder as currently (according to the https://mangowc.vercel.app/docs/configuration/monitors)
            // there is no way to disable a monitor
            // TODO: Update this if there is a way to disable a monitor
        }
    }

    let expand_path = |p: &str| -> PathBuf {
        if p.starts_with("~/") {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("/"))
                .join(p.strip_prefix("~/").unwrap())
        } else {
            PathBuf::from(p)
        }
    };

    let monitors_path = expand_path(&settings.monitors_conf_path);
    let bak_path = expand_path(&settings.monitors_bak_path);
    let config_path = expand_path(&settings.config_conf_path);

    // --- First-time backup: snapshot original monitorrule lines with source tracking ---
    // We scan config.conf AND every file it sources, tracking which file each rule came from.
    // The .bak file is JSON so we can restore rules back to their original locations.
    if !bak_path.exists() {
        if let Some(parent) = bak_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let collect_monitorrules = |path: &PathBuf| -> Vec<String> {
            fs::read_to_string(path)
                .unwrap_or_default()
                .lines()
                .filter(|l| {
                    let t = l.trim();
                    t.starts_with("monitorrule=") || t.starts_with("monitorrule =")
                })
                .map(|l| l.to_string())
                .collect()
        };

        let resolve_source = |raw: &str| -> PathBuf {
            if raw.starts_with("~/") || raw.starts_with('/') {
                expand_path(raw)
            } else {
                config_path
                    .parent()
                    .unwrap_or_else(|| std::path::Path::new("/"))
                    .join(raw)
            }
        };

        // Compact the path back to a tilde form for portability
        let to_portable = |p: &PathBuf| -> String {
            if let Some(home) = dirs::home_dir() {
                if let Ok(suffix) = p.strip_prefix(&home) {
                    return format!("~/{}", suffix.display());
                }
            }
            p.display().to_string()
        };

        let mut backup_entries: Vec<serde_json::Value> = Vec::new();

        if config_path.exists() {
            // Rules directly in config.conf
            let direct_rules = collect_monitorrules(&config_path);
            if !direct_rules.is_empty() {
                backup_entries.push(serde_json::json!({
                    "source_file": to_portable(&config_path),
                    "rules": direct_rules,
                }));
            }

            // Also follow any source= lines
            if let Ok(content) = fs::read_to_string(&config_path) {
                for line in content.lines() {
                    let t = line.trim();
                    let sourced_path_str = if t.starts_with("source=") {
                        Some(t.trim_start_matches("source=").trim())
                    } else if t.starts_with("source =") {
                        Some(t.trim_start_matches("source =").trim())
                    } else {
                        None
                    };

                    if let Some(raw) = sourced_path_str {
                        let sourced = resolve_source(raw);
                        let rules = collect_monitorrules(&sourced);
                        if !rules.is_empty() {
                            backup_entries.push(serde_json::json!({
                                "source_file": to_portable(&sourced),
                                "rules": rules,
                            }));
                        }
                    }
                }
            }
        }

        let backup_json = serde_json::json!({ "entries": backup_entries });
        fs::write(&bak_path, serde_json::to_string_pretty(&backup_json).unwrap_or_default())
            .map_err(|e| format!("Failed to write monitors.bak: {}", e))?;
    }

    if let Some(parent) = monitors_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create monitors dir: {}", e))?;
    }

    fs::write(&monitors_path, script)
        .map_err(|e| format!("Failed to write monitors config: {}", e))?;

    if settings.auto_append_source {
        // Use the tilde path from settings (portable, good for dotfiles)
        let source_line_tilde = format!("source={}", settings.monitors_conf_path);
        // Also recognise the expanded form in case it was written by an older version
        let source_line_abs = format!("source={}", monitors_path.display());
        let source_line_abs_spaced = format!("source = {}", monitors_path.display());
        let needs_source = if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .map_err(|e| format!("Failed to read config file: {}", e))?;
            !content.contains(&source_line_tilde)
                && !content.contains(&source_line_abs)
                && !content.contains(&source_line_abs_spaced)
        } else {
            true
        };

        if needs_source {
            use std::io::Write;
            if let Some(parent) = config_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&config_path)
                .map_err(|e| format!("Failed to open config file: {}", e))?;

            writeln!(file, "\n{}", source_line_tilde)
                .map_err(|e| format!("Failed to write config file source override: {}", e))?;
        }
    }

    Ok(())
}

pub fn wlr_randr_restore_default(settings: &crate::settings::AppSettings) -> Result<(), String> {
    let expand_path = |p: &str| -> PathBuf {
        if p.starts_with("~/") {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("/"))
                .join(p.strip_prefix("~/").unwrap())
        } else {
            PathBuf::from(p)
        }
    };

    let config_path = expand_path(&settings.config_conf_path);
    let monitors_path = expand_path(&settings.monitors_conf_path);
    let bak_path = expand_path(&settings.monitors_bak_path);

    // Parse the JSON backup
    let backup: serde_json::Value = if bak_path.exists() {
        let raw = fs::read_to_string(&bak_path)
            .map_err(|e| format!("Failed to read monitors.bak: {}", e))?;
        serde_json::from_str(&raw).unwrap_or(serde_json::json!({ "entries": [] }))
    } else {
        serde_json::json!({ "entries": [] })
    };

    let entries = backup["entries"].as_array().cloned().unwrap_or_default();

    // Helper to strip all monitorrule lines from a file's content
    let strip_monitorrules = |content: &str| -> String {
        content
            .lines()
            .filter(|l| {
                let t = l.trim();
                !t.starts_with("monitorrule=") && !t.starts_with("monitorrule =")
            })
            .map(|l| format!("{}\n", l))
            .collect()
    };

    // Step 1: Clean config.conf — remove monitorrule lines AND the source= line mango added
    if config_path.exists() {
        let source_line_tilde = format!("source={}", settings.monitors_conf_path);
        let source_line_abs = format!("source={}", monitors_path.display());
        let source_line_abs_spaced = format!("source = {}", monitors_path.display());

        let content = fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read config.conf: {}", e))?;

        let cleaned: String = content
            .lines()
            .filter(|l| {
                let t = l.trim();
                !t.starts_with("monitorrule=")
                    && !t.starts_with("monitorrule =")
                    && t != source_line_tilde.as_str()
                    && t != source_line_abs.as_str()
                    && t != source_line_abs_spaced.as_str()
            })
            .map(|l| format!("{}\n", l))
            .collect();

        fs::write(&config_path, cleaned)
            .map_err(|e| format!("Failed to write config.conf: {}", e))?;
    }

    // Step 2: For each backup entry, restore rules into their original source file
    for entry in &entries {
        if let (Some(source_file), Some(rules)) =
            (entry["source_file"].as_str(), entry["rules"].as_array())
        {
            let target_path = expand_path(source_file);
            let rules_block: String = rules
                .iter()
                .filter_map(|r| r.as_str())
                .map(|r| format!("{}\n", r))
                .collect();

            if rules_block.is_empty() {
                continue;
            }

            if target_path.exists() {
                // Strip any existing monitorrule lines first, then append originals
                let content = fs::read_to_string(&target_path).unwrap_or_default();
                let cleaned = strip_monitorrules(&content);
                let restored = format!("{}\n{}", cleaned.trim_end(), rules_block);
                fs::write(&target_path, restored)
                    .map_err(|e| format!("Failed to restore rules to {}: {}", source_file, e))?;
            } else {
                // The file doesn't exist anymore, write it fresh
                if let Some(parent) = target_path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                fs::write(&target_path, rules_block)
                    .map_err(|e| format!("Failed to create {}: {}", source_file, e))?;
            }
        }
    }

    // Step 3: Delete monitors.conf (only if it wasn't in the backup — i.e., mango created it)
    let monitors_was_backed_up = entries.iter().any(|e| {
        e["source_file"]
            .as_str()
            .map(|s| expand_path(s) == monitors_path)
            .unwrap_or(false)
    });
    if monitors_path.exists() && !monitors_was_backed_up {
        fs::remove_file(&monitors_path)
            .map_err(|e| format!("Failed to delete monitors.conf: {}", e))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_wlr_randr() {
        let sample = r#"eDP-1 "Unknown Unknown Unknown"
  Make: Unknown
  Model: Unknown
  Serial: Unknown
  Physical size: 340x190 mm
  Enabled: yes
  Position: 0,0
  Scale: 1.000000
  Transform: normal
  Modes:
    1920x1080 px, 60.000000 Hz (preferred, current)
DP-1 "Acer Acer KG271 C 28243AAB48T0"
  Make: Acer
  Model: Acer KG271 C
  Serial: 28243AAB48T0
  Physical size: 600x340 mm
  Enabled: no
  Position: 1920,0
  Scale: 1.500000
  Transform: 90
  Modes:
    1920x1080 px, 144.000000 Hz (preferred)
    1920x1080 px, 60.000000 Hz
"#;
        let outputs = parse_wlr_randr_output(sample).expect("Failed to parse");
        assert_eq!(outputs.len(), 2);

        let out1 = &outputs[0];
        assert_eq!(out1.name, "eDP-1");
        assert_eq!(out1.enabled, true);
        assert_eq!(out1.position, (0, 0));
        assert_eq!(out1.scale, 1.0);
        assert_eq!(out1.transform, "normal");
        assert_eq!(out1.modes.len(), 1);
        assert_eq!(out1.modes[0].width, 1920);
        assert_eq!(out1.modes[0].refresh_rate, 60.0);
        assert!(out1.modes[0].current);
        assert!(out1.modes[0].preferred);

        let out2 = &outputs[1];
        assert_eq!(out2.name, "DP-1");
        assert_eq!(out2.make, "Acer");
        assert_eq!(out2.enabled, false);
        assert_eq!(out2.position, (1920, 0));
        assert_eq!(out2.scale, 1.5);
        assert_eq!(out2.transform, "90");
        assert_eq!(out2.modes.len(), 2);
        assert_eq!(out2.modes[0].width, 1920);
        assert_eq!(out2.modes[0].refresh_rate, 144.0);
        assert!(out2.modes[0].preferred);
        assert!(!out2.modes[0].current);
        assert!(!out2.modes[1].current);
    }
}
