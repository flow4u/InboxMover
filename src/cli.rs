use std::path::Path;

use crate::core::InboxMoverCore;

/// Executes headless CLI batch or single-folder processing (matching `inbox_mover.py --cli`).
pub fn run_cli(
    mut core: InboxMoverCore,
    dir: Option<&str>,
    dir2: Option<&str>,
    single_folder: Option<&str>,
    workspace: Option<&str>,
    overrule: bool,
) -> Result<(), String> {
    if let Some(ws) = workspace {
        core.switch_workspace(ws);
    }
    if overrule {
        core.app_settings.overrule_receipt = true;
    }

    if let Some(folder_str) = single_folder {
        let p = Path::new(folder_str);
        if !p.is_dir() {
            return Err(format!("Specified folder '{}' does not exist.", folder_str));
        }
        let folder_data = core.inspect_transfer_folder(p);
        let (rule, src, _) = core.resolve_rule_for_folder(Some(&folder_data));
        println!(
            "[CLI] Processing '{}' (Config ID: {}, Rule Source: {})...",
            folder_data.folder_name, folder_data.permit_id, src
        );
        core.process_folder(
            &folder_data,
            &rule,
            Some(|cur, tot, name: &str| {
                println!("  [{cur}/{tot}] {name}");
            }),
            None::<fn(&str) -> Option<String>>,
        )?;
        println!("[CLI] Successfully processed '{}'.", folder_data.folder_name);
        return Ok(());
    }

    let d1 = dir.unwrap_or(&core.app_settings.search_folder_1).to_string();
    let d2 = dir2.unwrap_or(&core.app_settings.search_folder_2).to_string();

    if d1.trim().is_empty() && d2.trim().is_empty() {
        return Err(
            "No search directory provided. Pass --dir <PATH> or --folder <TRANSFER_PATH>."
                .to_string(),
        );
    }

    let mut folders = core.scan_search_directories(&d1, &d2);
    // Process oldest to newest
    folders.sort_by_key(|f| f.created_timestamp);

    let mut processed = 0;
    let mut skipped = 0;
    let mut failed = 0;

    for folder in &folders {
        let indicator = core.get_folder_status_indicator(folder);
        if indicator != "✓" {
            println!(
                "[CLI] Skipping '{}' (Status: {indicator})",
                folder.folder_name
            );
            skipped += 1;
            continue;
        }

        let (rule, src, _) = core.resolve_rule_for_folder(Some(folder));
        println!(
            "[CLI] Processing '{}' (Rule: {src} -> Target: '{}')...",
            folder.folder_name, rule.target_folder
        );
        match core.process_folder(
            folder,
            &rule,
            None::<fn(usize, usize, &str)>,
            None::<fn(&str) -> Option<String>>,
        ) {
            Ok(()) => {
                println!("  -> OK");
                processed += 1;
            }
            Err(e) => {
                eprintln!("  -> ERROR: {e}");
                failed += 1;
            }
        }
    }

    println!(
        "[CLI] Summary: {processed} processed, {skipped} skipped, {failed} failed (out of {} total).",
        folders.len()
    );

    if failed > 0 {
        Err(format!("{failed} folder(s) failed during CLI processing."))
    } else {
        Ok(())
    }
}
