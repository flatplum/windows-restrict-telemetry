//    This file is the entrypoint for the windows-restrict-telemetry program.
//    Copyright (C) 2026  flatplum
//
//    This program is free software: you can redistribute it and/or modify
//    it under the terms of the GNU Affero General Public License as
//    published by the Free Software Foundation, either version 3 of the
//    License, or (at your option) any later version.
//
//    This program is distributed in the hope that it will be useful,
//    but WITHOUT ANY WARRANTY; without even the implied warranty of
//    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
//    GNU Affero General Public License for more details.
//
//    You should have received a copy of the GNU Affero General Public License
//    along with this program.  If not, see <https://www.gnu.org/licenses/>.

use reqwest::blocking::get;
use zip::read::ZipArchive;
use tempfile::tempdir;
use std::{fs, io::{self, Write}, error::Error, collections::HashMap, process::Command};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct LGPOConfig {
    default: HashMap<String, Vec<String>>,
}

fn download_file(url: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let response = get(url)?;

    if !response.status().is_success() {
        return Err(format!("Failed to download file. HTTP Status: {}", response.status()).into());
    }

    let bytes = response.bytes()?;
    Ok(bytes.to_vec())
}

fn get_single_file_from_zip(zip_data: &[u8], filename: &str, output_path: &str) -> Result<(), Box<dyn Error>> {
    let cursor = io::Cursor::new(zip_data);
    let mut archive = ZipArchive::new(cursor)?;

    let mut file = archive.by_name(filename)?;
    let mut output_file = fs::File::create(output_path)?;

    io::copy(&mut file, &mut output_file)?;

    return Ok(());
}

fn main() -> Result<(), Box<dyn Error>> {
    let tools_dir = tempdir()?;
    let gpo_dir = tempdir()?;

    let machine_path = gpo_dir.path().join("Machine");
    let user_path = gpo_dir.path().join("User");

    let lgpo_url = r"https://download.microsoft.com/download/8/5/c/85c25433-a1b0-4ffa-9429-7e023e7da8d8/LGPO.zip";
    let lgpo_name = "LGPO.exe";
    let lgpo_path = tools_dir.path().join(lgpo_name);

    let exclude = vec!["1", "3", "18.4", "24", "24.1", "29", "29.99"];

    // Create necessary dirs
    fs::create_dir(&machine_path)?;
    fs::create_dir(&user_path)?;

    // Extract LGPO to Tools/LGPO.exe
    get_single_file_from_zip(
        &download_file(lgpo_url)?, 
        "LGPO_30/LGPO.exe", 
        match lgpo_path.to_str() {
            Some(x) => x,
            None => {
                return Err("Could not get path of LGPO.exe".into());
            }
        }
    )?;

    // Parse yaml
    let lgpo_config_str = String::from_utf8_lossy(include_bytes!("lgpo.yaml"));
    let lgpo_config = match serde_saphyr::from_str::<LGPOConfig>(&lgpo_config_str) {
        Ok(x) => x.default,
        Err(e) => {
            return Err(e.into())
        }
    };
    let lgpo_config_filtered: Vec<String> = lgpo_config.into_iter()
        .filter(|(k, _)| !exclude.contains(&k.as_str()))
        .flat_map(|(_, v)| v)
        .collect();

    // Generate .pol files
    let machinetxt_path = &machine_path.join("machine.txt");
    let usertxt_path = &user_path.join("user.txt");
    let machinetxt_file = fs::File::create(&machinetxt_path)?;
    let usertxt_file = fs::File::create(&usertxt_path)?;

    for entry in lgpo_config_filtered {
        if entry.starts_with("Computer") {
            writeln!(&machinetxt_file, "{}", entry)?;
        } else if entry.starts_with("User") {
            writeln!(&usertxt_file, "{}", entry)?;
        } else {
            eprintln!("Could not parse LGPO entry")
        }
    };

    drop(usertxt_file);
    drop(machinetxt_file);

    Command::new(&lgpo_path)
        .arg("/r")
        .arg(machinetxt_path)
        .arg("/w")
        .arg(&machine_path.join("registry.pol"))
        .output()?;

    Command::new(&lgpo_path)
        .arg("/r")
        .arg(usertxt_path)
        .arg("/w")
        .arg(&user_path.join("registry.pol"))
        .output()?;

    // Apply policies
    Command::new("powershell")
        .arg("-NoExit") // This is necessary for some reason
        .arg("-Command")
        .arg("Start-Process") 
        .arg(&lgpo_path.display().to_string()) 
        .arg("-ArgumentList")
        .arg(format!("'/g {}'", &gpo_dir.path().display())) 
        .arg("-Verb") 
        .arg("RunAs") 
        .output()?;

    Ok(())
}
