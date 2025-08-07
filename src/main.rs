use sha256::try_digest;
use std::process::Command;

#[cfg(windows)]
fn is_running_as_admin() -> bool {
    use windows::Win32::UI::Shell::IsUserAnAdmin;
    unsafe { IsUserAnAdmin().as_bool() }
}

#[cfg(not(windows))]
fn is_running_as_admin() -> bool {
    true
}

struct Runtime {
    url: &'static str,
    file: &'static str,
    args: &'static [&'static str],
    description: &'static str,
}

const RUNTIMES: &[Runtime] = &[
    Runtime {
        url: "https://download.visualstudio.microsoft.com/download/pr/e8b0aac4-7f86-4a7b-9a9a-448aa2b0f116/99a4178751b799db3d059b4b22b4451e/windowsdesktop-runtime-7.0.18-win-x64.exe",
        file: "windowsdesktop-runtime-7.0.18-win-x64.exe",
        args: &["-s"],
        description: ".NET 7 runtime",
    },
    Runtime {
        url: "https://download.visualstudio.microsoft.com/download/pr/c1d08a81-6e65-4065-b606-ed1127a954d3/14fe55b8a73ebba2b05432b162ab3aa8/windowsdesktop-runtime-8.0.4-win-x64.exe",
        file: "windowsdesktop-runtime-8.0.4-win-x64.exe",
        args: &["-s"],
        description: ".NET 8 runtime",
    },
    Runtime {
        url: "https://aka.ms/vs/17/release/vc_redist.x64.exe",
        file: "vc_redist.x64.exe",
        args: &["/install", "/quiet", "/norestart"],
        description: "Visual C++ Redistributable",
    },
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if !is_running_as_admin() {
        println!("Admin privileges are required to install Aimmy.");
        std::io::stdin().read_line(&mut String::new())?;
        return Ok(());
    }

    let temp_dir = std::env::temp_dir();
    for rt in RUNTIMES {
        let path = temp_dir.join(rt.file);
        if !path.exists() {
            println!("Downloading {}...", rt.description);
            download_file(rt.url, path.to_str().unwrap())?;
        }
        println!("Installing {}...", rt.description);
        if !Command::new(&path).args(rt.args).status()?.success() {
            eprintln!("Failed to install {}", rt.description);
        }
    }

    let client = reqwest::blocking::Client::new();
    let release: serde_json::Value = client
        .get("https://api.github.com/repos/Babyhamsta/aimmy/releases/latest")
        .header("User-Agent", "aimmy-setup")
        .send()?
        .json()?;

    let version = release["tag_name"].as_str().unwrap_or("unknown");

    if std::fs::metadata("Aimmy").is_ok() {
        if std::fs::metadata("Aimmy/bin/version.txt").is_ok() {
            let installed_version = std::fs::read_to_string("Aimmy/bin/version.txt")?;
            if installed_version == version {
                println!("Aimmy is already installed.");
                return Ok(());
            } else {
                println!("Updating Aimmy from {} to {}", installed_version, version);
            }
        }
    }

    let asset = release["assets"]
        .as_array()
        .and_then(|a| a.first())
        .ok_or("No assets found")?;
    let download_url = asset["browser_download_url"]
        .as_str()
        .ok_or("Missing download URL")?;
    println!("Downloading Aimmy from {}", download_url);
    download_file(download_url, "aimmy.zip")?;

    let hash = try_digest("aimmy.zip")?;
    let expected_hash = asset["digest"]
        .as_str()
        .and_then(|d| d.split(':').last())
        .unwrap_or("");
    if hash != expected_hash {
        eprintln!("Hash mismatch: expected {}, got {}", expected_hash, hash);
        return Err("Hash mismatch".into());
    }

    println!("Extracting aimmy.zip");
    let mut archive = zip::ZipArchive::new(std::fs::File::open("aimmy.zip")?)?;
    archive.extract("Aimmy")?;
    std::fs::write("Aimmy/bin/version.txt", version)?;
    
    println!("Cleaning up...");
    std::fs::remove_file("aimmy.zip")?;
    for rt in RUNTIMES {
        let path = temp_dir.join(rt.file);
        let _ = std::fs::remove_file(path);
    }

    Command::new("explorer")
        .arg(std::fs::canonicalize("Aimmy")?)
        .status()?;

    Ok(())
}

fn download_file(url: &str, destination: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut response = reqwest::blocking::get(url)?;
    let mut file = std::fs::File::create(destination)?;
    std::io::copy(&mut response, &mut file)?;
    Ok(())
}
