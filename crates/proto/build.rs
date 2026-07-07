use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

fn make_cmd_id_attr(line: &str) -> Option<String> {
    let marker = if line.contains("cmd_id") {
        "cmd_id"
    } else {
        "cmdid"
    };
    let cmd_id = line
        .split(marker)
        .nth(1)?
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse::<u32>()
        .ok()?;
    Some(format!("#[cmdid({cmd_id})]"))
}

fn output_stem(path: &Path) -> &str {
    let stem = path.file_stem().unwrap().to_str().unwrap();
    stem.strip_suffix("_pb").unwrap_or(stem)
}

fn implement_cmd_id(path: &Path) -> std::io::Result<()> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut output = Vec::new();

    let mut cmd_id_attr = None;
    for line in reader.lines() {
        let line = line?;
        let line_lower = line.to_lowercase();
        let is_cmd_id_comment = line_lower.trim_start().starts_with("///")
            && (line_lower.contains("cmdid") || line_lower.contains("cmd_id"));
        if is_cmd_id_comment {
            if let Some(attr) = make_cmd_id_attr(&line_lower) {
                cmd_id_attr = Some(attr);
            } else {
                output.push(line);
            }
        } else {
            output.push(line);
            if let Some(attr) = cmd_id_attr.take() {
                output.push(attr);
            }
        }
    }

    fs::write(path, output.join("\n").as_bytes())?;
    Ok(())
}

pub fn main() -> std::io::Result<()> {
    println!("cargo:rerun-if-changed=proto");

    const PROTO_DIR: &str = "proto";
    const OUT_DIR: &str = "out";

    fs::create_dir_all(OUT_DIR)?;

    let mut proto_files: Vec<_> = fs::read_dir(PROTO_DIR)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension()?.to_str()? == "proto" {
                Some(path)
            } else {
                None
            }
        })
        .collect();
    proto_files.sort();

    if !proto_files.is_empty() {
        prost_build::Config::new()
            .out_dir(OUT_DIR)
            .type_attribute(".", "#[derive(proto_derive::CmdID)]")
            .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
            .compile_protos(
                &proto_files.iter().map(|p| p.as_path()).collect::<Vec<_>>(),
                &[PROTO_DIR],
            )?;
    }

    for proto_file in &proto_files {
        let rust_file_name = format!("{}/{}.rs", OUT_DIR, output_stem(proto_file));
        implement_cmd_id(Path::new(&rust_file_name))?;
    }

    Ok(())
}
