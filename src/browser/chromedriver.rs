use tokio::sync::mpsc::UnboundedSender;

pub async fn setup_chromedriver(
    tx: &UnboundedSender<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (platform_name, exe_name) = match std::env::consts::OS {
        "windows" => ("win64", "chromedriver.exe"),
        "macos" => {
            if std::env::consts::ARCH == "aarch64" {
                ("mac-arm64", "chromedriver")
            } else {
                ("mac-x64", "chromedriver")
            }
        }
        "linux" => ("linux64", "chromedriver"),
        _ => return Err("Unsupported OS".into()),
    };

    if std::fs::metadata(exe_name).is_ok() {
        return Ok(());
    }

    let _ = tx.send(format!("Downloading ChromeDriver for {}...", platform_name));
    let client = reqwest::Client::new();
    let res = client
        .get("https://googlechromelabs.github.io/chrome-for-testing/last-known-good-versions-with-downloads.json")
        .send().await?
        .json::<serde_json::Value>().await?;

    let url = res
        .get("channels")
        .and_then(|c| c.get("Stable"))
        .and_then(|c| c.get("downloads"))
        .and_then(|c| c.get("chromedriver"))
        .and_then(|c| c.as_array())
        .and_then(|arr| {
            arr.iter()
                .find(|x| x.get("platform").and_then(|p| p.as_str()) == Some(platform_name))
        })
        .and_then(|x| x.get("url"))
        .and_then(|u| u.as_str())
        .ok_or_else(|| {
            Box::<dyn std::error::Error>::from(
                "Failed to parse ChromeDriver download URL from JSON",
            )
        })?;

    let zip_bytes = client.get(url).send().await?.bytes().await?;
    tokio::fs::write("chromedriver.zip", &zip_bytes).await?;

    let _ = tx.send("Extracting ChromeDriver...".into());
    let output = tokio::process::Command::new("tar")
        .args(["-xf", "chromedriver.zip"])
        .output()
        .await?;

    if !output.status.success() {
        return Err("Failed to extract zip file.".into());
    }

    let folder_name = format!("chromedriver-{}", platform_name);
    let extracted_exe = format!("{}/{}", folder_name, exe_name);
    let _ = tokio::fs::rename(&extracted_exe, exe_name).await;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(mut perms) = std::fs::metadata(exe_name).map(|m| m.permissions()) {
            perms.set_mode(0o755);
            let _ = std::fs::set_permissions(exe_name, perms);
        }
    }

    let _ = tokio::fs::remove_file("chromedriver.zip").await;
    let _ = tokio::fs::remove_dir_all(&folder_name).await;

    let _ = tx.send("ChromeDriver installed".into());
    Ok(())
}
