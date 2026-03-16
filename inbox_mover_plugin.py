#!/usr/bin/env python3
"""
Inbox Mover Plugin (v0.0.2)
---------------------------
A utility to process, extract, and log ZIP files (and standard files) from a specific source folder.

🌱 ALPHA VERSION NOTICE
Welcome to the Inbox Mover Plugin! Please note that this is an early alpha release. 
While we've designed it to be as helpful and robust as possible, you should not 
implicitly trust it with critical or production data just yet. We warmly encourage 
you to test it thoroughly in a safe, backed-up environment to ensure it behaves 
exactly as you need it to. Happy testing!

USAGE AS A PLUGIN:
    from inbox_mover_plugin import im

    config = {
        "target_folder": "/path/to/extract/files",
        "processed_folder": "/path/to/move/completed/source",
        "receipt_folder": "/path/to/save/receipts",
        "conflict_resolution": "rename_existing",
        "post_processing": "move",
        "auto_unzip": True
    }

    result = im("/path/to/source_folder", config)
    print(result)

USAGE FROM COMMAND LINE:
    python inbox_mover_plugin.py --source "C:\\test\\source_folder" --config "C:\\test\\config.json"

CONFIG DICTIONARY / JSON STRUCTURE:
    {
        "target_folder": "str"       -> (Required) Where contents are extracted/copied.
        "processed_folder": "str"    -> (Optional) Where the source folder is moved if post_processing is 'move'.
        "receipt_folder": "str"      -> (Optional) Specific folder to place the extracted receipt.json.
        "conflict_resolution": "str" -> (Optional) 'overwrite' (default), 'keep_both', or 'rename_existing'.
        "post_processing": "str"     -> (Optional) 'leave' (default), 'delete', or 'move'.
        "auto_unzip": "bool"         -> (Optional) True to extract ZIPs, False to copy them as-is (default: True).
    }

EXAMPLE config.json:
    {
        "target_folder": "C:/data/extracted",
        "processed_folder": "C:/data/archived_zips",
        "receipt_folder": "C:/data/receipts",
        "conflict_resolution": "keep_both",
        "post_processing": "move",
        "auto_unzip": true
    }

OVERRIDES:
    If a `receipt.json` file is found inside the source ZIP, the plugin will read it. 
    If keys matching the config are found inside `receipt.json` (e.g., "target_folder"), 
    they will OVERRIDE the configuration passed to the plugin.
"""

import os
import sys
import json
import zipfile
import shutil
import datetime
import argparse
import getpass


class InboxProcessor:
    def __init__(self, config):
        # Establish baseline config with defaults
        self.base_config = {
            "target_folder": config.get("target_folder"),
            "processed_folder": config.get("processed_folder"),
            "receipt_folder": config.get("receipt_folder"),
            "conflict_resolution": config.get("conflict_resolution", "overwrite"),
            "post_processing": config.get("post_processing", "leave"),
            "auto_unzip": config.get("auto_unzip", True)
        }
        self.actions_log = []

    def process(self, source_folder):
        """Main processing pipeline for a given source folder."""
        source_folder = os.path.abspath(source_folder)
        
        if not os.path.exists(source_folder):
            return self._fail(f"Source folder does not exist: {source_folder}")
        if not os.path.isdir(source_folder):
            return self._fail(f"Source path is not a directory: {source_folder}")

        # 1. Inspect the folder
        folder_data = self._inspect_folder(source_folder)
        if not folder_data["can_process"]:
            return self._fail("Folder is empty. Nothing to process.", folder_data)

        # 2. Merge config with any overrides from a discovered receipt.json
        active_config = self._merge_config(folder_data.get("receipt", {}))

        # 3. Validate configuration
        if not active_config.get("target_folder"):
            return self._fail("Target folder is not specified in config or receipt.", folder_data, active_config)
        
        if active_config["post_processing"] == "move" and not active_config.get("processed_folder"):
            return self._fail("Post processing is 'move' but 'processed_folder' is not specified.", folder_data, active_config)

        # 4. Execute file operations
        try:
            self._ensure_directories(active_config)
            self._process_files(folder_data, active_config)
            self._apply_post_processing(folder_data, active_config)
            
            # Write logs to the final destination of the folder/files
            self._write_local_log("SUCCESS", folder_data, active_config, "Successfully processed folder.")
            self._update_receipt_log(folder_data, active_config)

            return {
                "status": "SUCCESS",
                "message": "Processed successfully.",
                "actions": self.actions_log,
                "config_used": active_config
            }

        except Exception as e:
            self._write_local_log("ERROR", folder_data, active_config, str(e))
            return self._fail(f"Processing error: {str(e)}", folder_data, active_config)

    # --- Internal Helpers ---

    def _fail(self, message, folder_data=None, config=None):
        if folder_data:
            self._write_local_log("ERROR", folder_data, config or self.base_config, message)
        return {
            "status": "ERROR",
            "message": message,
            "actions": self.actions_log,
            "config_used": config or self.base_config
        }

    def _inspect_folder(self, folder_path):
        """Inspect folder for a valid zip containing receipt.json, or fallback to raw files."""
        data = {
            "folder_path": folder_path,
            "folder_name": os.path.basename(folder_path),
            "zip_path": None,
            "permitId": "DEFAULT",
            "receipt": None,
            "has_valid_zip": False,
            "can_process": False
        }
        
        for root, _, files in os.walk(folder_path):
            for file in files:
                if file.lower().endswith('.zip'):
                    zip_path = os.path.join(root, file)
                    zip_info = self._inspect_zip(zip_path)
                    if zip_info and zip_info.get("receipt"):
                        data["zip_path"] = zip_path
                        data["permitId"] = zip_info["permitId"]
                        data["receipt"] = zip_info["receipt"]
                        data["has_valid_zip"] = True
                        data["can_process"] = True
                        return data
        
        # Fallback: No valid zip with receipt found. Check if folder has files.
        for root, _, files in os.walk(folder_path):
            if files:
                data["can_process"] = True
                return data
                
        return data

    def _inspect_zip(self, zip_path):
        """Extract receipt.json from a zip memory stream without extracting files."""
        try:
            with zipfile.ZipFile(zip_path, 'r') as zf:
                receipt_filename = next((f for f in zf.namelist() if f.endswith('receipt.json')), None)
                if receipt_filename:
                    with zf.open(receipt_filename) as f:
                        content = f.read().decode('utf-8')
                        try:
                            receipt_json = json.loads(content)
                            return {
                                "receipt": receipt_json,
                                "permitId": receipt_json.get("permitId", "DEFAULT")
                            }
                        except json.JSONDecodeError:
                            pass
        except Exception:
            pass
        return None

    def _merge_config(self, receipt_data):
        """Override base config with any values found inside receipt.json."""
        merged = self.base_config.copy()
        if not receipt_data:
            return merged
            
        # Map potential receipt keys to plugin config keys
        override_mapping = {
            "target_folder": "target_folder",
            "processed_folder": "processed_folder",  # receipt key matches plugin key
            "process_folder": "processed_folder",    # legacy receipt key support
            "receipt_folder": "receipt_folder",
            "conflict_resolution": "conflict_resolution",
            "post_processing": "post_processing",
            "auto_unzip": "auto_unzip"
        }
        
        for receipt_key, config_key in override_mapping.items():
            if receipt_key in receipt_data:
                merged[config_key] = receipt_data[receipt_key]
                
        return merged

    def _ensure_directories(self, config):
        os.makedirs(config["target_folder"], exist_ok=True)
        if config.get("receipt_folder"):
            os.makedirs(config["receipt_folder"], exist_ok=True)
        if config["post_processing"] == "move" and config.get("processed_folder"):
            os.makedirs(config["processed_folder"], exist_ok=True)

    def _get_final_path(self, extracted_path, conflict_res):
        """Handle file naming collisions based on conflict resolution strategy."""
        if not os.path.exists(extracted_path):
            return extracted_path
            
        if conflict_res == 'overwrite':
            self.actions_log.append({
                "type": "conflict_resolved",
                "source": extracted_path,
                "message": "Existing file overwritten"
            })
            return extracted_path
            
        elif conflict_res == 'keep_both':
            base, ext = os.path.splitext(extracted_path)
            counter = 1
            while os.path.exists(f"{base} ({counter}){ext}"):
                counter += 1
            new_path = f"{base} ({counter}){ext}"
            self.actions_log.append({
                "type": "conflict_resolved",
                "source": extracted_path,
                "message": f"Kept both. Extracted file renamed to {os.path.basename(new_path)}"
            })
            return new_path
            
        elif conflict_res == 'rename_existing':
            timestamp = datetime.datetime.now().strftime("%y%m%d-%H%M%S")
            base, ext = os.path.splitext(extracted_path)
            dirname = os.path.dirname(extracted_path)
            filename = os.path.basename(extracted_path)
            renamed_path = os.path.join(dirname, f"{timestamp}_{filename}")
            
            if os.path.exists(renamed_path):
                counter = 1
                while os.path.exists(f"{renamed_path}_{counter}"):
                    counter += 1
                renamed_path = f"{renamed_path}_{counter}"
                
            os.rename(extracted_path, renamed_path)
            self.actions_log.append({
                "type": "conflict_resolved",
                "source": extracted_path,
                "message": f"Existing file renamed to {os.path.basename(renamed_path)}"
            })
            return extracted_path

        return extracted_path

    def _process_files(self, folder_data, config):
        """Extract valid ZIP or copy loose files depending on auto_unzip configuration."""
        target_folder = config["target_folder"]
        receipt_folder = config.get("receipt_folder")
        conflict_res = config["conflict_resolution"]
        auto_unzip = config.get("auto_unzip", True)

        def _copy_file(src_path, base_folder_path, folder_name):
            rel_path = os.path.relpath(src_path, base_folder_path)
            ext_path = os.path.join(target_folder, rel_path)
            os.makedirs(os.path.dirname(ext_path), exist_ok=True)
            final_path = self._get_final_path(ext_path, conflict_res)
            
            shutil.copy2(src_path, final_path)
            self.actions_log.append({
                "type": "copy",
                "source": f"{folder_name} -> {rel_path}",
                "destination": final_path
            })

        def _extract_zip(zip_path):
            zip_filename = os.path.basename(zip_path)
            with zipfile.ZipFile(zip_path, 'r') as zf:
                file_list = [f for f in zf.infolist() if not f.is_dir()]
                for zinfo in file_list:
                    original_name = zinfo.filename
                    is_absolute = original_name.startswith('/') or original_name.startswith('\\') or (len(original_name) >= 3 and original_name[1] == ':' and original_name[2] in ('/', '\\'))
                    
                    if original_name.lower().endswith('receipt.json'):
                        timestamp = datetime.datetime.now().strftime("%y%m%d-%H%M%S")
                        new_filename = f"{timestamp}-{os.path.basename(original_name)}"
                        if receipt_folder and os.path.isdir(receipt_folder):
                            ext_path = os.path.join(receipt_folder, new_filename)
                        elif is_absolute:
                            ext_path = os.path.join(os.path.dirname(original_name), new_filename)
                        else:
                            ext_path = os.path.join(target_folder, os.path.dirname(original_name.lstrip('/\\')), new_filename)
                    else:
                        if is_absolute:
                            ext_path = original_name
                        else:
                            safe_name = original_name.lstrip('/\\')
                            ext_path = os.path.join(target_folder, safe_name)
                            
                    os.makedirs(os.path.dirname(ext_path), exist_ok=True)
                    final_path = self._get_final_path(ext_path, conflict_res)

                    with zf.open(zinfo) as source, open(final_path, "wb") as target:
                        shutil.copyfileobj(source, target)
                        
                    self.actions_log.append({
                        "type": "extract",
                        "source": f"{zip_filename} -> {original_name}",
                        "destination": final_path
                    })

        # Logic fork: process zip or loose files
        if folder_data.get('has_valid_zip') and folder_data.get('zip_path'):
            if auto_unzip:
                _extract_zip(folder_data['zip_path'])
            else:
                folder_path = folder_data['folder_path']
                folder_name = folder_data['folder_name']
                _copy_file(folder_data['zip_path'], folder_path, folder_name)
        else:
            folder_path = folder_data['folder_path']
            folder_name = folder_data['folder_name']
            for root, _, files in os.walk(folder_path):
                for file in files:
                    src_path = os.path.join(root, file)
                    if file.lower().endswith('.zip') and auto_unzip:
                        _extract_zip(src_path)
                    else:
                        _copy_file(src_path, folder_path, folder_name)

    def _apply_post_processing(self, folder_data, config):
        """Apply leave, move, or delete actions to the source folder."""
        post_action = config["post_processing"]
        folder_path = folder_data['folder_path']
        
        if post_action == 'delete':
            if folder_path and os.path.isdir(folder_path):
                shutil.rmtree(folder_path)
                self.actions_log.append({
                    "type": "post_processing",
                    "source": folder_path,
                    "destination": "DELETED"
                })
        elif post_action == 'move':
            processed_folder = config["processed_folder"]
            folder_name = folder_data['folder_name']
            dest_path = os.path.join(processed_folder, folder_name)
            
            if os.path.exists(dest_path):
                counter = 1
                while os.path.exists(f"{dest_path}_{counter}"):
                    counter += 1
                dest_path = f"{dest_path}_{counter}"
                
            shutil.move(folder_path, dest_path)
            self.actions_log.append({
                "type": "post_processing",
                "source": folder_path,
                "destination": dest_path
            })

    def _write_local_log(self, status, folder_data, config, message=""):
        """Write a formatted log text file in the final location of the source folder."""
        post_action = config.get('post_processing', 'leave')
        target_local_dir = None
        
        if post_action == 'leave':
            target_local_dir = folder_data.get('folder_path')
        elif post_action == 'move':
            # Find the new destination path from actions log
            for act in self.actions_log:
                if act.get('type') == 'post_processing' and act.get('destination') not in ('DELETED', ''):
                    target_local_dir = act.get('destination')
                    break
        elif post_action == 'delete' and status != 'SUCCESS':
            target_local_dir = folder_data.get('folder_path')
            
        if not target_local_dir or not os.path.isdir(target_local_dir):
            return

        local_log_path = os.path.join(target_local_dir, "Inbox Process.log")
        ts = datetime.datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        user = getpass.getuser()
        permit_id = folder_data.get('permitId', 'Unknown')
        folder_name = folder_data.get('folder_name', 'Unknown')
        
        lines = [f"[{ts}] {status} | User: {user} | Config: {permit_id} | Folder: {folder_name}"]
        if message:
            lines.append(f"  Message: {message}")
        if self.actions_log:
            lines.append("  Actions:")
            for act in self.actions_log:
                a_type = str(act.get("type", "")).upper()
                a_src = act.get("source", "")
                a_dest = act.get("destination", "")
                a_msg = act.get("message", "")
                
                if a_type == "CONFLICT_RESOLVED":
                    lines.append(f"    - CONFLICT: {a_src} -> {a_msg}")
                else:
                    lines.append(f"    - {a_type}: {a_src} -> {a_dest}")
        lines.append("-" * 80)
        new_log_text = "\n".join(lines) + "\n\n"
        
        existing_content = ""
        if os.path.exists(local_log_path):
            try:
                with open(local_log_path, 'r', encoding='utf-8') as f:
                    existing_content = f.read()
            except Exception:
                pass
                
        try:
            with open(local_log_path, 'w', encoding='utf-8') as f:
                f.write(new_log_text + existing_content)
        except Exception as e:
            print(f"Failed to write local log to {local_log_path}: {e}")

    def _update_receipt_log(self, folder_data, config):
        """Append processing log to the extracted receipt.json if it exists."""
        log_entry = {
            "timestamp": datetime.datetime.now().isoformat(),
            "user": getpass.getuser(),
            "status": "SUCCESS",
            "folder_name": folder_data.get('folder_name', 'Unknown'),
            "config_id": folder_data.get('permitId', 'Unknown'),
            "files_processed": len([a for a in self.actions_log if a.get('type') in ('extract', 'copy')]),
            "config_applied": config,
            "actions": self.actions_log
        }

        for act in self.actions_log:
            dest = act.get("destination", "")
            if isinstance(dest, str) and dest.lower().endswith("receipt.json") and os.path.exists(dest):
                try:
                    with open(dest, 'r', encoding='utf-8') as f:
                        try:
                            receipt_data = json.load(f)
                        except json.JSONDecodeError:
                            receipt_data = {"_raw_file_unparsable": True}
                    
                    if "processing_logs" not in receipt_data:
                        receipt_data["processing_logs"] = []
                        
                    receipt_data["processing_logs"].append(log_entry)
                    
                    with open(dest, 'w', encoding='utf-8') as f:
                        json.dump(receipt_data, f, indent=4)
                except Exception as e:
                    print(f"Failed to update receipt.json with processing log: {e}")


def im(source_folder, config):
    """
    Main plugin entry point.
    
    :param source_folder: A string path to the directory to process.
    :param config: A dictionary containing the configuration keys.
                   Required: 'target_folder'
                   Optional: 'processed_folder', 'receipt_folder', 
                             'conflict_resolution', 'post_processing',
                             'auto_unzip'
    :return: A dictionary containing processing status and action logs.
    """
    processor = InboxProcessor(config)
    return processor.process(source_folder)


# --------------------------------------------------------------------------- #
# CLI IMPLEMENTATION
# --------------------------------------------------------------------------- #

if __name__ == '__main__':
    parser = argparse.ArgumentParser(
        description="Inbox Mover Plugin CLI",
        formatter_class=argparse.RawTextHelpFormatter,
        epilog="""
Example Usage:
  python inbox_mover_plugin.py --source "C:/transfer-123" --config "C:/config.json"
  
Config JSON Structure:
  {
      "target_folder": "C:/output/extracted",
      "processed_folder": "C:/output/processed_zips",
      "receipt_folder": "C:/output/receipts",
      "conflict_resolution": "rename_existing",
      "post_processing": "move",
      "auto_unzip": true
  }
"""
    )
    parser.add_argument('-s', '--source', required=True, help='Path to the source folder containing the zip/files.')
    parser.add_argument('-c', '--config', required=True, help='Path to the JSON configuration file.')

    args = parser.parse_args()

    # Load configuration from file
    if not os.path.exists(args.config):
        print(f"Error: Config file not found at {args.config}")
        sys.exit(1)
        
    try:
        with open(args.config, 'r', encoding='utf-8') as f:
            plugin_config = json.load(f)
    except json.JSONDecodeError as e:
        print(f"Error parsing config JSON file: {e}")
        sys.exit(1)

    print(f"Processing Source: {args.source}")
    print("...")

    # Run the plugin
    result = im(args.source, plugin_config)

    # Output results
    if result["status"] == "SUCCESS":
        print(f"SUCCESS: {result['message']}")
        print(f"Files extracted/copied: {len([a for a in result['actions'] if a['type'] in ['extract', 'copy']])}")
    else:
        print(f"ERROR: {result['message']}")
        
    # Optional: print detailed actions if required
    for action in result.get("actions", []):
        print(f"  - {action['type'].upper()}: {action.get('source')} -> {action.get('destination', action.get('message'))}")