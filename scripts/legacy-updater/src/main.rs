use std::{error::Error, path::PathBuf};
use tauri_plugin_updater::UpdaterExt;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 5 { return Err("usage: <fixture-root> <old-executable> <feed-url> <fixture-public-key> <current-version>".into()); }
    let root = PathBuf::from(&args[0]).canonicalize()?;
    let executable = PathBuf::from(&args[1]).canonicalize()?;
    if !executable.starts_with(&root) || std::fs::read_to_string(root.join(".legacy-update-fixture"))? != "disposable-test-only" {
        return Err("An explicitly disposable fixture is required".into());
    }
    if !args[2].starts_with("http://127.0.0.1:") { return Err("Only a local fixture feed is allowed".into()); }
    let mut context = tauri::test::mock_context(tauri::test::noop_assets());
    context.package_info_mut().version = args[4].parse()?;
    context.config_mut().plugins.0.insert("updater".into(), serde_json::json!({
        "pubkey": args[3], "dangerousInsecureTransportProtocol": true,
        "windows": { "installMode": "passive" }
    }));
    let app = tauri::test::mock_builder()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .build(context)?;
    let updater = app.updater_builder().executable_path(executable)
        .endpoints(vec![args[2].parse()?])?.no_proxy().build()?;
    tauri::async_runtime::block_on(async {
        let update = updater.check().await?.ok_or("No compatible upgrade")?;
        update.download_and_install(|_, _| {}, || {}).await?;
        Ok::<_, Box<dyn Error>>(())
    })?;
    println!("legacy-updater-installed");
    Ok(())
}
