#!/usr/bin/env python3
"""
Inbox Mover v0.17.3
A utility to process and extract zip files containing a receipt*.json,
with both a Material-inspired GUI and a CLI mode.
Runs entirely on standard Python libraries.
"""

import os
import sys
import json
import zipfile
import shutil
import datetime
import argparse
import threading
import subprocess
import getpass
import re
import fnmatch
import webbrowser

# Tkinter imports
import tkinter as tk
from tkinter import ttk, filedialog, messagebox, simpledialog

VERSION = "0.17.3"

# --------------------------------------------------------------------------- #
# HELPERS
# --------------------------------------------------------------------------- #

class ToolTip:
    """A simple tooltip class for Tkinter widgets."""
    def __init__(self, widget, text, is_dark_mode=True):
        self.widget = widget
        self.text = text
        self.is_dark_mode = is_dark_mode
        self.tip_window = None
        self.widget.bind("<Enter>", self.show_tip)
        self.widget.bind("<Leave>", self.hide_tip)

    def show_tip(self, event=None):
        if self.tip_window or not self.text:
            return
        x, y, _cx, cy = self.widget.bbox("insert")
        x = x + self.widget.winfo_rootx() + 25
        y = y + cy + self.widget.winfo_rooty() + 25
        self.tip_window = tw = tk.Toplevel(self.widget)
        tw.wm_overrideredirect(True)
        tw.wm_geometry(f"+{x}+{y}")
        tw.attributes("-topmost", True)
        
        # Theme adaptive colors
        bg = "#333333" if self.is_dark_mode else "#ffffe0"
        fg = "#ffffff" if self.is_dark_mode else "#000000"
        
        label = tk.Label(tw, text=self.text, justify=tk.LEFT,
                         background=bg, foreground=fg, relief=tk.SOLID, borderwidth=1,
                         font=("Segoe UI", "9", "normal"), padx=4, pady=2)
        label.pack(ipadx=1)

    def hide_tip(self, event=None):
        tw = self.tip_window
        self.tip_window = None
        if tw:
            tw.destroy()

# --------------------------------------------------------------------------- #
# CORE LOGIC
# --------------------------------------------------------------------------- #

class InboxMoverCore:
    def __init__(self):
        self.local_config_dir = "permit_configs"
        if not os.path.exists(self.local_config_dir):
            os.makedirs(self.local_config_dir, exist_ok=True)

        # Caching layer for performance
        self._config_cache = {}
        self._patterns_cache = {}

        settings = self.load_app_settings()
        self.use_global = settings.get("use_global", False)
        self.global_dir = settings.get("global_dir", "")

        self.set_workspace()

    def translate_path(self, path):
        """
        Translates paths between Windows and Linux environments for specific known mounts.
        Windows z:\ <-> Linux /mnt/data/
        Windows i:\ <-> Linux /mnt/inbox/
        """
        if not path or not isinstance(path, str):
            return path
        
        # Normalize slashes for internal check
        p_check = path.replace('\\', '/')
        
        if sys.platform == "win32":
            # Translate Linux -> Windows
            if p_check.startswith("/mnt/data/"):
                suffix = path[10:].replace('/', '\\')
                return f"z:\\{suffix}"
            elif p_check.startswith("/mnt/inbox/"):
                suffix = path[11:].replace('/', '\\')
                return f"i:\\{suffix}"
            elif p_check == "/mnt/data":
                return "z:\\"
            elif p_check == "/mnt/inbox":
                return "i:\\"
        else:
            # Translate Windows -> Linux
            if p_check.lower().startswith("z:/"):
                suffix = path[3:].replace('\\', '/')
                return f"/mnt/data/{suffix}"
            elif p_check.lower().startswith("i:/"):
                suffix = path[3:].replace('\\', '/')
                return f"/mnt/inbox/{suffix}"
            elif p_check.lower() in ("z:", "z:/"):
                return "/mnt/data/"
            elif p_check.lower() in ("i:", "i:/"):
                return "/mnt/inbox/"
                
        return path

    def set_workspace(self):
        """Configure the active directory and reload cache."""
        if self.use_global and self.global_dir:
            self.config_dir = self.global_dir
        else:
            self.config_dir = self.local_config_dir

        self.ensure_config_dir()
        self.log_file = os.path.join(self.config_dir, "process_log.jsonl")
        self.reload_cache()

    def reload_cache(self):
        """Load all configurations and patterns into memory for high-speed access."""
        self._config_cache = {}
        self._patterns_cache = {}
        
        if not os.path.exists(self.config_dir):
            return

        # Load Patterns
        patterns_path = os.path.join(self.config_dir, "patterns.json")
        if os.path.exists(patterns_path):
            try:
                with open(patterns_path, 'r', encoding='utf-8') as f:
                    self._patterns_cache = json.load(f)
            except Exception:
                self._patterns_cache = {}

        # Load all individual permit configs
        try:
            for f in os.listdir(self.config_dir):
                if f.endswith('.json') and f not in ('app_settings.json', 'patterns.json'):
                    permit_id = f[:-5]
                    path = os.path.join(self.config_dir, f)
                    try:
                        with open(path, 'r', encoding='utf-8') as file:
                            self._config_cache[permit_id] = json.load(file)
                    except Exception:
                        pass
        except Exception:
            pass

    def ensure_config_dir(self):
        """Create the config directory and base files if they don't exist."""
        if not os.path.exists(self.config_dir):
            os.makedirs(self.config_dir, exist_ok=True)
            
        patterns_file = os.path.join(self.config_dir, "patterns.json")
        if not os.path.exists(patterns_file):
            with open(patterns_file, 'w', encoding='utf-8') as f:
                json.dump({}, f, indent=4)

    def load_app_settings(self):
        """App settings are always stored locally with OS-aware defaults."""
        settings_path = os.path.join(self.local_config_dir, "app_settings.json")
        
        # OS-Specific first-time defaults
        if sys.platform == "win32":
            def_sf1, def_sf2 = "i:\\", "z:\\inbox"
        else:
            def_sf1, def_sf2 = "/mnt/inbox/", "/mnt/data/"
            
        settings = {"dark_mode": True, "font_size": 11, "window_geometry": "1120x950", 
                    "search_folder_1": def_sf1, "search_folder_2": def_sf2, 
                    "use_global": False, "global_dir": ""}
        
        if os.path.exists(settings_path):
            try:
                with open(settings_path, 'r', encoding='utf-8') as f:
                    loaded = json.load(f)
                    settings.update(loaded)
            except Exception:
                pass
        
        # Apply path translation to existing search folders
        settings["search_folder_1"] = self.translate_path(settings.get("search_folder_1", ""))
        settings["search_folder_2"] = self.translate_path(settings.get("search_folder_2", ""))
        return settings

    def save_app_settings(self, settings):
        settings_path = os.path.join(self.local_config_dir, "app_settings.json")
        with open(settings_path, 'w', encoding='utf-8') as f:
            json.dump(settings, f, indent=4)

    def find_transfer_folders(self, search_folders):
        """Find transfer folders efficiently."""
        folders_data = []
        seen_paths = set()
        
        if isinstance(search_folders, str):
            search_folders = [search_folders]

        for search_folder in search_folders:
            if not search_folder or not os.path.isdir(search_folder):
                continue

            try:
                # Use listdir for speed on network shares
                items = os.listdir(search_folder)
            except Exception:
                continue

            for item in items:
                if item.lower().startswith('transfer-'):
                    item_path = os.path.join(search_folder, item)
                    if item_path not in seen_paths and os.path.isdir(item_path):
                        seen_paths.add(item_path)
                        folder_data = self.inspect_transfer_folder(item_path)
                        folders_data.append(folder_data)
        
        folders_data.sort(key=lambda x: x['folder_name'], reverse=True)
        return folders_data

    def inspect_transfer_folder(self, folder_path):
        """Inspect a transfer folder for a valid zip or a loose receipt*.json."""
        folder_name = os.path.basename(folder_path)
        data = {
            "folder_path": folder_path,
            "folder_name": folder_name,
            "zip_path": None,
            "permitId": "DEFAULT",
            "receipt": None,
            "receipt_raw": "",
            "has_valid_zip": False,
            "can_process": False,
            "has_log": False,
            "file_list": []
        }
        
        # Check for presence of logs
        if os.path.exists(os.path.join(folder_path, "Process.log")) or \
           os.path.exists(os.path.join(folder_path, "Inbox Process.log")):
            data["has_log"] = True

        valid_zip_found = False
        file_list = []
        loose_receipt_data = None
        loose_receipt_raw = ""
        has_loose_receipt = False
        
        # 1. Gather all files and prioritize finding a loose receipt*.json
        for root, _, files in os.walk(folder_path):
            for file in files:
                rel_file = os.path.relpath(os.path.join(root, file), folder_path)
                file_list.append(rel_file)
                
                # Use glob pattern for receipt files
                if fnmatch.fnmatch(file.lower(), 'receipt*.json'):
                    has_loose_receipt = True
                    receipt_path = os.path.join(root, file)
                    try:
                        with open(receipt_path, 'r', encoding='utf-8') as f:
                            raw_content = f.read()
                            # If we find multiple, we'll append them for viewing, 
                            # but pick the first valid one for configuration.
                            if not loose_receipt_raw:
                                loose_receipt_raw = raw_content
                            else:
                                loose_receipt_raw += f"\n\n--- OTHER RECEIPT: {file} ---\n" + raw_content
                            
                        if raw_content.strip() and not loose_receipt_data:
                            try:
                                loose_receipt_data = json.loads(raw_content)
                            except json.JSONDecodeError as e:
                                warning_msg = f"[WARNING: {file} is not a valid JSON file.]\n[Error details: {e}]\n\n"
                                loose_receipt_raw = warning_msg + loose_receipt_raw
                    except Exception as e:
                        print(f"Error reading {file}: {e}")
                        loose_receipt_raw = f"[Error reading {file}: {e}]"
        
        data["file_list"] = file_list
        
        # 2. Check for ZIP files and apply logic
        for root, _, files in os.walk(folder_path):
            for file in files:
                if file.lower().endswith('.zip') and not valid_zip_found:
                    zip_path = os.path.join(root, file)
                    data["zip_path"] = zip_path
                    data["has_valid_zip"] = True
                    valid_zip_found = True
                    
                    if has_loose_receipt:
                        data["receipt"] = loose_receipt_data
                        data["receipt_raw"] = loose_receipt_raw if loose_receipt_raw.strip() else "<receipt files are empty>"
                        data["permitId"] = loose_receipt_data.get("permitId", "DEFAULT") if loose_receipt_data else "DEFAULT"
                    else:
                        zip_info = self.inspect_zip(zip_path)
                        if zip_info and zip_info.get("receipt_raw"):
                            data["permitId"] = zip_info["permitId"]
                            data["receipt"] = zip_info["receipt"]
                            data["receipt_raw"] = zip_info["receipt_raw"]
                    break
        
        if not valid_zip_found:
            if has_loose_receipt:
                data["receipt"] = loose_receipt_data
                data["receipt_raw"] = loose_receipt_raw if loose_receipt_raw.strip() else "<receipt files are empty>"
                data["permitId"] = loose_receipt_data.get("permitId", "DEFAULT") if loose_receipt_data else "DEFAULT"
                data["can_process"] = True
            elif not file_list:
                data["receipt_raw"] = ""
                data["can_process"] = False
            else:
                data["receipt_raw"] = ""
                data["can_process"] = True
        else:
            data["can_process"] = True
                
        return data

    def inspect_zip(self, zip_path):
        """Read a zip file to extract receipt*.json without full extraction."""
        data = {
            "permitId": "DEFAULT",
            "receipt": None,
            "receipt_raw": ""
        }
        try:
            with zipfile.ZipFile(zip_path, 'r') as zf:
                # Use glob pattern search within ZIP
                receipt_filenames = [f for f in zf.namelist() if fnmatch.fnmatch(os.path.basename(f).lower(), 'receipt*.json')]
                
                if receipt_filenames:
                    combined_raw = ""
                    first_valid_json = None
                    
                    for receipt_filename in receipt_filenames:
                        try:
                            with zf.open(receipt_filename) as f:
                                content = f.read().decode('utf-8')
                                if not combined_raw:
                                    combined_raw = content
                                else:
                                    combined_raw += f"\n\n--- OTHER RECEIPT IN ZIP: {os.path.basename(receipt_filename)} ---\n" + content
                                
                                if not first_valid_json:
                                    try:
                                        first_valid_json = json.loads(content)
                                    except json.JSONDecodeError:
                                        pass
                        except RuntimeError:
                            combined_raw += f"\n<{os.path.basename(receipt_filename)} is password protected>"
                    
                    data["receipt_raw"] = combined_raw
                    if first_valid_json:
                        data["receipt"] = first_valid_json
                        data["permitId"] = first_valid_json.get("permitId", "DEFAULT")
                return data
        except Exception:
            return None

    def load_config(self, permit_id):
        return self._config_cache.get(permit_id)

    def get_all_configs(self):
        return self._config_cache

    def save_config(self, permit_id, config_data):
        if not permit_id:
            raise ValueError("Cannot save configuration without a Config ID.")
        config_path = os.path.join(self.config_dir, f"{permit_id}.json")
        with open(config_path, 'w', encoding='utf-8') as f:
            json.dump(config_data, f, indent=4)
        self._config_cache[permit_id] = config_data

    def delete_config(self, permit_id):
        if not permit_id:
            return
        config_path = os.path.join(self.config_dir, f"{permit_id}.json")
        if os.path.exists(config_path):
            os.remove(config_path)
        if permit_id in self._config_cache:
            del self._config_cache[permit_id]

    def load_patterns(self):
        return self._patterns_cache

    def save_pattern(self, pattern, config_data):
        self._patterns_cache[pattern] = config_data
        patterns_path = os.path.join(self.config_dir, "patterns.json")
        with open(patterns_path, 'w', encoding='utf-8') as f:
            json.dump(self._patterns_cache, f, indent=4)

    def delete_pattern(self, pattern):
        if pattern in self._patterns_cache:
            del self._patterns_cache[pattern]
            patterns_path = os.path.join(self.config_dir, "patterns.json")
            with open(patterns_path, 'w', encoding='utf-8') as f:
                json.dump(self._patterns_cache, f, indent=4)

    def write_log(self, status, folder_data, config, actions, message=""):
        log_entry = {
            "timestamp": datetime.datetime.now().isoformat(),
            "user": getpass.getuser(),
            "status": status,
            "folder_name": folder_data.get('folder_name', 'Unknown'),
            "config_id": folder_data.get('permitId', 'Unknown'),
            "files_processed": len([a for a in actions if a.get('type') in ('extract', 'copy')]),
            "config_applied": config,
            "actions": actions,
            "message": message
        }
        try:
            with open(self.log_file, 'a', encoding='utf-8') as f:
                f.write(json.dumps(log_entry) + '\n')
        except Exception:
            pass
            
        post_action = config.get('post_action', 'leave') if config else 'leave'
        target_local_dir = None
        
        if post_action == 'leave':
            target_local_dir = folder_data.get('folder_path')
        elif post_action == 'move':
            for act in actions:
                if act.get('type') == 'post_processing' and act.get('destination') not in ('DELETED', ''):
                    target_local_dir = act.get('destination')
                    break
            if not target_local_dir:
                target_local_dir = folder_data.get('folder_path')
        elif post_action == 'delete' and status != 'SUCCESS':
            target_local_dir = folder_data.get('folder_path')
            
        if target_local_dir and os.path.isdir(target_local_dir):
            local_log_path = os.path.join(target_local_dir, "Process.log")
            
            ts = log_entry["timestamp"].replace("T", " ")[:19]
            lines = [f"[{ts}] | User: {log_entry['user']} | Config: {log_entry['config_id']} | Folder: {log_entry['folder_name']}"]
            if message:
                lines.append(f"  Message: {message}")
                if actions:
                    lines.append("  Actions:")
                    for act in actions:
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
            except Exception:
                pass

        for act in actions:
            dest = act.get("destination", "")
            if isinstance(dest, str) and dest.lower().endswith(".json") and fnmatch.fnmatch(os.path.basename(dest).lower(), 'receipt*.json') and os.path.exists(dest):
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
                except Exception:
                    pass

    def safe_copy(self, src, dst):
        """Robust copy that handles 'Operation not permitted' on restrictive mounts."""
        try:
            # Try preserving all metadata
            shutil.copy2(src, dst)
        except (OSError, PermissionError) as e:
            # Fallback: copy data only if utime or ownership preservation fails
            # This handles [Errno 1] on Linux network mounts
            if e.errno == 1 or "permitted" in str(e).lower():
                try:
                    # shutil.copyfile is the most basic; no metadata touch
                    shutil.copyfile(src, dst)
                except Exception:
                    raise e
            else:
                raise e

    def safe_rename(self, src, dst):
        """Robust rename that handles permission issues by falling back to copy+delete."""
        try:
            os.rename(src, dst)
        except (OSError, PermissionError) as e:
            if e.errno == 1 or "permitted" in str(e).lower():
                self.safe_copy(src, dst)
                try:
                    if os.path.isdir(src):
                        shutil.rmtree(src)
                    else:
                        os.remove(src)
                except Exception:
                    pass # Original remains but copy succeeded
            else:
                raise e

    def process_zip(self, folder_data, config, progress_callback=None, password_callback=None):
        target_folder = config.get('target_folder')
        conflict_action = config.get('conflict_action', 'overwrite')
        post_action = config.get('post_action', 'leave')
        target_zip_folder = config.get('target_zip_folder')
        receipt_folder = config.get('receipt_folder')
        auto_extract = config.get('auto_extract', True)
        
        actions_log = []

        if not target_folder:
            error_msg = "Target folder is not specified."
            self.write_log("ERROR", folder_data, config, actions_log, error_msg)
            raise ValueError(error_msg)

        if not os.path.exists(target_folder):
            try:
                os.makedirs(target_folder, exist_ok=True)
            except Exception as e:
                error_msg = f"Failed to create Target folder '{target_folder}': {e}"
                self.write_log("ERROR", folder_data, config, actions_log, error_msg)
                raise ValueError(error_msg)

        if receipt_folder and not os.path.exists(receipt_folder):
            try:
                os.makedirs(receipt_folder, exist_ok=True)
            except Exception as e:
                error_msg = f"Failed to create Receipt folder '{receipt_folder}': {e}"
                self.write_log("ERROR", folder_data, config, actions_log, error_msg)
                raise ValueError(error_msg)
                
        if post_action == 'move':
            if not target_zip_folder:
                error_msg = "Processed Folder is not specified but post action is 'move'."
                self.write_log("ERROR", folder_data, config, actions_log, error_msg)
                raise ValueError(error_msg)
            if not os.path.exists(target_zip_folder):
                try:
                    os.makedirs(target_zip_folder, exist_ok=True)
                except Exception as e:
                    error_msg = f"Failed to create Processed folder '{target_zip_folder}': {e}"
                    self.write_log("ERROR", folder_data, config, actions_log, error_msg)
                    raise ValueError(error_msg)

        def get_final_path(extracted_path):
            if os.path.exists(extracted_path):
                if conflict_action == 'overwrite':
                    actions_log.append({
                        "type": "conflict_resolved",
                        "source": extracted_path,
                        "message": "Existing file overwritten"
                    })
                elif conflict_action == 'keep_both':
                    base, ext = os.path.splitext(extracted_path)
                    counter = 1
                    while os.path.exists(f"{base} ({counter}){ext}"):
                        counter += 1
                    new_path = f"{base} ({counter}){ext}"
                    actions_log.append({
                        "type": "conflict_resolved",
                        "source": extracted_path,
                        "message": f"Kept both. Extracted file renamed to {os.path.basename(new_path)}"
                    })
                    extracted_path = new_path
                elif conflict_action == 'rename_existing':
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
                    
                    self.safe_rename(extracted_path, renamed_path)
                    actions_log.append({
                        "type": "conflict_resolved",
                        "source": extracted_path,
                        "message": f"Existing file renamed to {os.path.basename(renamed_path)}"
                    })
            return extracted_path

        def extract_zip_file(zip_path):
            zip_filename = os.path.basename(zip_path)
            pwd = None
            with zipfile.ZipFile(zip_path, 'r') as zf:
                file_list = [f for f in zf.infolist() if not f.is_dir()]
                total = len(file_list)
                for i, zinfo in enumerate(file_list):
                    original_name = zinfo.filename
                    
                    if progress_callback:
                        progress_callback(i + 1, total, original_name)
                        
                    safe_name = original_name.lstrip('/\\')
                    parts = safe_name.replace('\\', '/').split('/')
                    
                    if len(parts) > 0 and parts[0].lower().startswith('transfer-'):
                        if len(parts) == 1:
                            continue
                        safe_name = '/'.join(parts[1:])
                    
                    is_absolute = original_name.startswith('/') or original_name.startswith('\\') or (len(original_name) >= 3 and original_name[1] == ':' and original_name[2] in ('/', '\\'))
                    
                    # Wildcard match for receipt*.json in ZIP
                    if fnmatch.fnmatch(os.path.basename(original_name).lower(), 'receipt*.json'):
                        timestamp = datetime.datetime.now().strftime("%y%m%d-%H%M%S")
                        new_filename = f"{timestamp}-{os.path.basename(safe_name)}"
                        if receipt_folder and os.path.isdir(receipt_folder):
                            ext_path = os.path.join(receipt_folder, new_filename)
                        elif is_absolute:
                            ext_path = os.path.join(os.path.dirname(original_name), new_filename)
                        else:
                            ext_path = os.path.join(target_folder, os.path.dirname(safe_name), new_filename)
                    else:
                        if is_absolute:
                            ext_path = original_name
                        else:
                            ext_path = os.path.join(target_folder, safe_name)
                            
                    os.makedirs(os.path.dirname(ext_path), exist_ok=True)
                    final_path = get_final_path(ext_path)

                    while True:
                        try:
                            with zf.open(zinfo, pwd=pwd) as source, open(final_path, "wb") as target:
                                shutil.copyfileobj(source, target)
                            break
                        except (RuntimeError, OSError) as e:
                            err_str = str(e).lower()
                            if 'password' in err_str or 'encrypted' in err_str:
                                if password_callback:
                                    pwd_str = password_callback(zip_filename)
                                    if pwd_str is None:
                                        raise ValueError(f"Password entry cancelled by user. Extraction of {zip_filename} aborted.")
                                    pwd = pwd_str.encode('utf-8')
                                else:
                                    raise ValueError(f"Password required for {zip_filename} but no password provided.")
                            else:
                                raise e

                    actions_log.append({
                        "type": "extract",
                        "source": f"{zip_filename} -> {original_name}",
                        "destination": final_path
                    })

        try:
            folder_path = folder_data.get('folder_path')
            folder_name = folder_data.get('folder_name')

            for root, _, files in os.walk(folder_path):
                for file in files:
                    src_path = os.path.join(root, file)
                    if file.lower() in ('process.log', 'inbox process.log'):
                        continue
                        
                    if auto_extract and file.lower().endswith('.zip'):
                        extract_zip_file(src_path)
                    else:
                        rel_path = os.path.relpath(src_path, folder_path)
                        # Wildcard match for loose receipt*.json
                        if fnmatch.fnmatch(file.lower(), 'receipt*.json'):
                            timestamp = datetime.datetime.now().strftime("%y%m%d-%H%M%S")
                            new_filename = f"{timestamp}-{file}"
                            if receipt_folder and os.path.isdir(receipt_folder):
                                ext_path = os.path.join(receipt_folder, new_filename)
                            else:
                                ext_path = os.path.join(target_folder, os.path.dirname(rel_path), new_filename)
                        else:
                            ext_path = os.path.join(target_folder, rel_path)
                            
                        os.makedirs(os.path.dirname(ext_path), exist_ok=True)
                        final_path = get_final_path(ext_path)
                        self.safe_copy(src_path, final_path)
                        
                        actions_log.append({
                            "type": "copy",
                            "source": f"{folder_name} -> {rel_path}",
                            "destination": final_path
                        })

            if post_action == 'delete':
                folder_path = folder_data.get('folder_path')
                if folder_path and os.path.isdir(folder_path):
                    shutil.rmtree(folder_path)
                    actions_log.append({
                        "type": "post_processing",
                        "source": folder_path,
                        "destination": "DELETED"
                    })
            elif post_action == 'move':
                folder_path = folder_data.get('folder_path')
                if not folder_path or not os.path.isdir(folder_path):
                    raise ValueError(f"Source folder '{folder_path}' is invalid or missing.")
                    
                folder_name = folder_data.get('folder_name')
                dest_path = os.path.join(target_zip_folder, folder_name)
                
                if os.path.exists(dest_path):
                    counter = 1
                    while os.path.exists(f"{dest_path}_{counter}"):
                        counter += 1
                    dest_path = f"{dest_path}_{counter}"
                
                # Robust move for restrictive mounts
                try:
                    shutil.move(folder_path, dest_path)
                except (OSError, PermissionError) as e:
                    if e.errno == 1 or "permitted" in str(e).lower():
                        # Manual move fallback
                        os.makedirs(dest_path, exist_ok=True)
                        for item in os.listdir(folder_path):
                            s = os.path.join(folder_path, item)
                            d = os.path.join(dest_path, item)
                            self.safe_rename(s, d)
                        shutil.rmtree(folder_path)
                    else:
                        raise e

                actions_log.append({
                    "type": "post_processing",
                    "source": folder_path,
                    "destination": dest_path
                })

            self.write_log("SUCCESS", folder_data, config, actions_log, "Successfully processed folder.")

        except Exception as e:
            self.write_log("ERROR", folder_data, config, actions_log, str(e))
            raise e

# --------------------------------------------------------------------------- #
# GUI APPLICATION
# --------------------------------------------------------------------------- #

class InboxMoverGUI:
    def __init__(self, root):
        self.root = root
        self.root.title(f"Inbox Mover v{VERSION}")
        self.root.minsize(1400, 750)
        
        self.core = InboxMoverCore()
        settings = self.core.load_app_settings()
        self.is_dark_mode = settings.get("dark_mode", True)
        self.base_font_size = settings.get("font_size", 10)
        
        window_geometry = settings.get("window_geometry", "1550x850")
        self.root.geometry(window_geometry)
        self.root.protocol("WM_DELETE_WINDOW", self.on_closing)
        
        self.folders_data = []
        self.current_index = -1

        self.workspace_mode_var = tk.StringVar(value="global" if self.core.use_global else "local")
        self.global_dir_var = tk.StringVar(value=self.core.global_dir)

        sf1 = settings.get("search_folder_1", "")
        sf2 = settings.get("search_folder_2", "")
        
        self.search_folder_1_var = tk.StringVar(value=sf1)
        self.search_folder_2_var = tk.StringVar(value=sf2)
        
        self.target_folder_var = tk.StringVar()
        self.target_zip_folder_var = tk.StringVar()
        self.receipt_folder_var = tk.StringVar()
        
        self.conflict_action_var = tk.StringVar(value="overwrite")
        self.post_action_var = tk.StringVar(value="leave")
        self.active_pattern_var = tk.StringVar(value="")
        self.auto_extract_var = tk.BooleanVar(value=True)
        
        self.conflict_display_var = tk.StringVar()
        self.post_display_var = tk.StringVar()
        
        self.conflict_map = {
            "Overwrite existing file": "overwrite", 
            "Keep both (add number)": "keep_both", 
            "Rename existing with timestamp": "rename_existing"
        }
        self.post_map = {
            "Leave files in place": "leave", 
            "Delete original folder": "delete", 
            "Move to Processed folder": "move"
        }
        self.conflict_reverse_map = {v: k for k, v in self.conflict_map.items()}
        self.post_reverse_map = {v: k for k, v in self.post_map.items()}
        
        self.conflict_display_var.trace_add("write", lambda *a: self.conflict_action_var.set(self.conflict_map.get(self.conflict_display_var.get(), "overwrite")))
        self.post_display_var.trace_add("write", lambda *a: self.post_action_var.set(self.post_map.get(self.post_display_var.get(), "leave")))
        
        self.inbox_name_var = tk.StringVar(value="")
        self.zip_name_var = tk.StringVar(value="No Transfer Folders Found")
        self.permit_id_var = tk.StringVar(value="")
        self.last_processed_var = tk.StringVar(value="")
        self.nav_count_var = tk.StringVar(value="0 Folders")

        self.target_folder_var.trace_add("write", self.check_unsaved_changes)
        self.target_zip_folder_var.trace_add("write", self.check_unsaved_changes)
        self.receipt_folder_var.trace_add("write", self.check_unsaved_changes)
        self.conflict_action_var.trace_add("write", self.check_unsaved_changes)
        self.post_action_var.trace_add("write", self.check_unsaved_changes)
        self.active_pattern_var.trace_add("write", self.check_unsaved_changes)
        self.auto_extract_var.trace_add("write", self.check_unsaved_changes)

        self.setup_ui()
        self.apply_theme()
        self.apply_fonts()
        self.bind_keys()
        
        if self.search_folder_1_var.get() or self.search_folder_2_var.get():
            self.on_search_folder_changed(startup=True)
            
        self.root.after(50, self.show_welcome_splash)

    def setup_ui(self):
        self.main_frame = ttk.Frame(self.root, padding=15)
        self.main_frame.pack(fill=tk.BOTH, expand=True)

        header_frame = ttk.Frame(self.main_frame)
        header_frame.pack(fill=tk.X, pady=(0, 10))
        
        title_frame = ttk.Frame(header_frame)
        title_frame.pack(side=tk.LEFT)
        self.lbl_title = ttk.Label(title_frame, text="Inbox Mover", style="AppTitle.TLabel")
        self.lbl_title.pack(side=tk.LEFT)
        self.lbl_version = ttk.Label(title_frame, text=f"v{VERSION}", style="CardDim.TLabel")
        self.lbl_version.pack(side=tk.LEFT, padx=(10, 0), anchor=tk.S)
        
        tools_frame = ttk.Frame(header_frame)
        tools_frame.pack(side=tk.RIGHT, anchor=tk.S)
        
        self.theme_btn = ttk.Button(tools_frame, text="☀", width=3, command=self.toggle_theme, takefocus=0)
        self.theme_btn.pack(side=tk.RIGHT)
        ToolTip(self.theme_btn, "Toggle Dark/Light Theme", self.is_dark_mode)

        self.btn_help = ttk.Button(tools_frame, text="?", width=3, command=self.show_help, takefocus=0)
        self.btn_help.pack(side=tk.RIGHT, padx=5)
        ToolTip(self.btn_help, "Open Manual", self.is_dark_mode)
        
        log_menu = ttk.Frame(tools_frame)
        log_menu.pack(side=tk.RIGHT, padx=10)
        ttk.Button(log_menu, text="📂 Log Folder", command=self.open_log_folder, takefocus=0).pack(side=tk.RIGHT, padx=2)
        ttk.Button(log_menu, text="📄 View Log", command=self.view_log, takefocus=0).pack(side=tk.RIGHT, padx=2)
        ttk.Button(log_menu, text="🗑 Clear Log", command=self.clear_log, takefocus=0).pack(side=tk.RIGHT, padx=2)
        
        font_menu = ttk.Frame(tools_frame)
        font_menu.pack(side=tk.RIGHT, padx=10)
        ttk.Button(font_menu, text="A+", width=3, command=self.increase_font, takefocus=0).pack(side=tk.RIGHT, padx=1)
        ttk.Button(font_menu, text="Reset View", command=self.reset_view, takefocus=0).pack(side=tk.RIGHT, padx=1)
        ttk.Button(font_menu, text="A-", width=3, command=self.decrease_font, takefocus=0).pack(side=tk.RIGHT, padx=1)

        self.paned = ttk.PanedWindow(self.main_frame, orient=tk.HORIZONTAL)
        self.paned.pack(fill=tk.BOTH, expand=True, pady=(5, 0))

        self.left_panel = ttk.Frame(self.paned, style="Card.TFrame", padding=15)
        self.paned.add(self.left_panel, weight=35)

        ttk.Label(self.left_panel, text="SETTINGS WORKSPACE", style="SectionHeader.TLabel").pack(anchor=tk.W, pady=(0, 8))
        ws_grid = ttk.Frame(self.left_panel, style="Card.TFrame")
        ws_grid.pack(fill=tk.X, pady=(0, 15))
        
        ttk.Radiobutton(ws_grid, text="Personal (Local)", variable=self.workspace_mode_var, value="local", command=self.apply_workspace, style="Card.TRadiobutton").pack(anchor=tk.W, pady=2)
        
        team_frame = ttk.Frame(ws_grid, style="Card.TFrame")
        team_frame.pack(fill=tk.X, pady=2)
        ttk.Radiobutton(team_frame, text="Team Shared:", variable=self.workspace_mode_var, value="global", command=self.apply_workspace, style="Card.TRadiobutton").pack(side=tk.LEFT)
        
        dir_frame = ttk.Frame(ws_grid, style="Card.TFrame")
        dir_frame.pack(fill=tk.X, pady=(2, 0))
        self.entry_global_dir = ttk.Entry(dir_frame, textvariable=self.global_dir_var, width=5, state=tk.NORMAL if self.workspace_mode_var.get() == "global" else tk.DISABLED)
        self.entry_global_dir.pack(side=tk.LEFT, fill=tk.X, expand=True)
        self.btn_browse_global = ttk.Button(dir_frame, text="...", width=3, command=self.browse_global_dir, state=tk.NORMAL if self.workspace_mode_var.get() == "global" else tk.DISABLED)
        self.btn_browse_global.pack(side=tk.RIGHT, padx=(5, 0))

        ttk.Separator(self.left_panel, orient='horizontal').pack(fill=tk.X, pady=10)

        ttk.Label(self.left_panel, text="SCAN LOCATIONS", style="SectionHeader.TLabel").pack(anchor=tk.W, pady=(0, 8))
        src_frame = ttk.Frame(self.left_panel, style="Card.TFrame")
        src_frame.pack(fill=tk.X, pady=(0, 15))
        self.create_folder_input(src_frame, "Search 1:", self.search_folder_1_var, self.on_search_folder_changed)
        self.create_folder_input(src_frame, "Search 2:", self.search_folder_2_var, self.on_search_folder_changed)
        
        ttk.Separator(self.left_panel, orient='horizontal').pack(fill=tk.X, pady=10)

        q_header = ttk.Frame(self.left_panel, style="Card.TFrame")
        q_header.pack(fill=tk.X, pady=(0, 5))
        ttk.Label(q_header, text="PENDING QUEUE", style="SectionHeader.TLabel").pack(side=tk.LEFT)
        self.lbl_nav_count = ttk.Label(q_header, textvariable=self.nav_count_var, style="CardDim.TLabel")
        self.lbl_nav_count.pack(side=tk.LEFT, padx=10)
        self.btn_refresh = ttk.Button(q_header, text="↻ Refresh", command=lambda: self.on_search_folder_changed(is_manual_refresh=True), takefocus=0)
        self.btn_refresh.pack(side=tk.RIGHT)

        list_frame = ttk.Frame(self.left_panel, style="Card.TFrame")
        list_frame.pack(fill=tk.BOTH, expand=True)
        font_family = "Segoe UI" if sys.platform == "win32" else "Helvetica"
        self.queue_listbox = tk.Listbox(list_frame, font=(font_family, self.base_font_size), selectbackground="#2563eb", activestyle="none", highlightthickness=0, borderwidth=0)
        self.queue_listbox.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
        scrollbar = ttk.Scrollbar(list_frame, orient=tk.VERTICAL, command=self.queue_listbox.yview)
        self.queue_listbox.configure(yscrollcommand=scrollbar.set)
        scrollbar.pack(side=tk.RIGHT, fill=tk.Y)
        self.queue_listbox.bind('<<ListboxSelect>>', self.on_queue_select)

        self.right_panel = ttk.Frame(self.paned, style="Card.TFrame")
        self.paned.add(self.right_panel, weight=65)
        
        self.placeholder_frame = ttk.Frame(self.right_panel, style="Card.TFrame")
        ttk.Label(self.placeholder_frame, text="Select a folder from the queue to review and process.", style="CardDim.TLabel").pack(expand=True)
        
        self.detail_container = ttk.Frame(self.right_panel, style="Card.TFrame", padding=25)
        
        header_row = ttk.Frame(self.detail_container, style="Card.TFrame")
        header_row.pack(fill=tk.X, pady=(0, 5))
        self.lbl_zip_name = ttk.Label(header_row, textvariable=self.zip_name_var, style="CardTitle.TLabel")
        self.lbl_zip_name.pack(side=tk.LEFT, fill=tk.X, expand=True)
        
        utils_frame = ttk.Frame(header_row, style="Card.TFrame")
        utils_frame.pack(side=tk.RIGHT)
        self.btn_open_local_log = ttk.Button(utils_frame, text="📄 View Log", command=self.open_local_log, takefocus=0)
        self.btn_open_local_log.pack(side=tk.RIGHT)

        badge_row = ttk.Frame(self.detail_container, style="Card.TFrame")
        badge_row.pack(fill=tk.X, pady=(0, 15))
        self.lbl_permit_id = ttk.Label(badge_row, textvariable=self.permit_id_var, style="Badge.TLabel")
        self.lbl_permit_id.pack(side=tk.LEFT, padx=(0, 10))
        self.lbl_inbox_name = ttk.Label(badge_row, textvariable=self.inbox_name_var, style="CardDim.TLabel")
        self.lbl_inbox_name.pack(side=tk.LEFT, padx=(0, 10))
        self.lbl_last_processed = ttk.Label(badge_row, textvariable=self.last_processed_var, style="CardAccent.TLabel")
        self.lbl_last_processed.pack(side=tk.LEFT)
        
        ttk.Separator(self.detail_container, orient='horizontal').pack(fill=tk.X, pady=(5, 15))

        self.form_paned = ttk.PanedWindow(self.detail_container, orient=tk.HORIZONTAL)
        self.form_paned.pack(fill=tk.X, pady=(0, 15))
        
        dest_col = ttk.Frame(self.form_paned, style="Card.TFrame")
        self.form_paned.add(dest_col, weight=1)
        dest_inner = ttk.Frame(dest_col, style="Card.TFrame")
        dest_inner.pack(fill=tk.BOTH, expand=True, padx=(0, 10))
        ttk.Label(dest_inner, text="DESTINATIONS", style="SectionHeader.TLabel").pack(anchor=tk.W, pady=(0, 10))
        self.create_folder_input(dest_inner, "Target:", self.target_folder_var)
        self.create_folder_input(dest_inner, "Processed:", self.target_zip_folder_var)
        self.create_folder_input(dest_inner, "Receipt:", self.receipt_folder_var)

        rules_col = ttk.Frame(self.form_paned, style="Card.TFrame")
        rules_col.pack(fill=tk.BOTH, expand=True)
        self.form_paned.add(rules_col, weight=1)
        rules_inner = ttk.Frame(rules_col, style="Card.TFrame")
        rules_inner.pack(fill=tk.BOTH, expand=True, padx=(10, 0))
        ttk.Label(rules_inner, text="PROCESSING RULES", style="SectionHeader.TLabel").pack(anchor=tk.W, pady=(0, 10))
        
        pat_row = ttk.Frame(rules_inner, style="Card.TFrame")
        pat_row.pack(fill=tk.X, pady=2)
        ttk.Label(pat_row, text="Pattern Match:", width=14, style="Card.TLabel").pack(side=tk.LEFT)
        self.entry_pattern = ttk.Entry(pat_row, textvariable=self.active_pattern_var, width=5)
        self.entry_pattern.pack(side=tk.LEFT, fill=tk.X, expand=True)
        
        conf_row = ttk.Frame(rules_inner, style="Card.TFrame")
        conf_row.pack(fill=tk.X, pady=4)
        ttk.Label(conf_row, text="Conflict Action:", width=14, style="Card.TLabel").pack(side=tk.LEFT)
        c_cb = ttk.Combobox(conf_row, textvariable=self.conflict_display_var, values=list(self.conflict_map.keys()), state="readonly", width=5)
        c_cb.pack(side=tk.LEFT, fill=tk.X, expand=True)

        post_row = ttk.Frame(rules_inner, style="Card.TFrame")
        post_row.pack(fill=tk.X, pady=4)
        ttk.Label(post_row, text="Post Action:", width=14, style="Card.TLabel").pack(side=tk.LEFT)
        p_cb = ttk.Combobox(post_row, textvariable=self.post_display_var, values=list(self.post_map.keys()), state="readonly", width=5)
        p_cb.pack(side=tk.LEFT, fill=tk.X, expand=True)

        chk_row = ttk.Frame(rules_inner, style="Card.TFrame")
        chk_row.pack(fill=tk.X, pady=4)
        ttk.Label(chk_row, text="", width=14, style="Card.TLabel").pack(side=tk.LEFT)
        ttk.Checkbutton(chk_row, text="Auto-extract ZIP contents", variable=self.auto_extract_var, style="Card.TCheckbutton", takefocus=0).pack(side=tk.LEFT)
        
        receipt_row = ttk.Frame(rules_inner, style="Card.TFrame")
        receipt_row.pack(fill=tk.X, pady=10)
        ttk.Label(receipt_row, text="", width=14, style="Card.TLabel").pack(side=tk.LEFT)
        self.btn_create_receipt = ttk.Button(receipt_row, text="Create Receipt", command=self.create_receipt_file, takefocus=0)
        self.btn_create_receipt.pack(side=tk.LEFT)
        ToolTip(self.btn_create_receipt, "Export current settings to a receipt*.json file", self.is_dark_mode)

        ttk.Separator(self.detail_container, orient='horizontal').pack(fill=tk.X, pady=(5, 10))

        ttk.Label(self.detail_container, text="FILE INSPECTOR", style="SectionHeader.TLabel").pack(anchor=tk.W)
        self.receipt_text = tk.Text(self.detail_container, wrap=tk.WORD, state=tk.DISABLED, relief="flat", highlightthickness=1, height=8, takefocus=0)
        self.receipt_text.pack(side=tk.TOP, fill=tk.BOTH, expand=True, pady=(10, 15))
        scroll_txt = ttk.Scrollbar(self.detail_container, orient=tk.VERTICAL, command=self.receipt_text.yview)
        self.receipt_text.configure(yscrollcommand=scroll_txt.set)

        card_actions = ttk.Frame(self.detail_container, style="Card.TFrame")
        card_actions.pack(side=tk.BOTTOM, fill=tk.X)
        
        rule_actions_group = ttk.Frame(card_actions, style="Card.TFrame")
        rule_actions_group.pack(side=tk.LEFT)
        
        rule_header_row = ttk.Frame(rule_actions_group, style="Card.TFrame")
        rule_header_row.pack(fill=tk.X, padx=2)
        ttk.Label(rule_header_row, text="RULES", font=("Segoe UI", 7, "bold"), foreground="#757575").pack(side=tk.LEFT)
        
        self.lbl_active_ws = ttk.Label(rule_header_row, text="", font=("Segoe UI", 7, "bold"), padding=(4, 0))
        self.lbl_active_ws.pack(side=tk.LEFT, padx=(5, 0))
        
        rule_btns = ttk.Frame(rule_actions_group, style="Card.TFrame")
        rule_btns.pack(fill=tk.X)

        self.btn_ws_reload = ttk.Button(rule_btns, text="↻", width=3, command=self.apply_workspace, takefocus=0)
        self.btn_ws_reload.pack(side=tk.LEFT, padx=(0, 5))
        ToolTip(self.btn_ws_reload, "Reload configurations from disk", self.is_dark_mode)

        self.btn_manage_configs = ttk.Button(rule_btns, text="⚙ Manage", command=self.open_manage_configs, takefocus=0)
        self.btn_manage_configs.pack(side=tk.LEFT, padx=(0, 5))
        
        self.btn_delete_config = ttk.Button(rule_btns, text="🗑 Delete", command=self.delete_selected_rule, takefocus=0, state=tk.DISABLED)
        self.btn_delete_config.pack(side=tk.LEFT, padx=(0, 5))
        
        self.btn_save_config = ttk.Button(rule_btns, text="💾 Save", command=self.save_permit_config, takefocus=0)
        self.btn_save_config.pack(side=tk.LEFT)

        ttk.Frame(card_actions, style="Card.TFrame").pack(side=tk.LEFT, fill=tk.X, expand=True)

        folder_actions_group = ttk.Frame(card_actions, style="Card.TFrame")
        folder_actions_group.pack(side=tk.RIGHT)
        
        ttk.Label(folder_actions_group, text="FOLDER", font=("Segoe UI", 7, "bold"), foreground="#757575").pack(anchor=tk.W, padx=2)
        folder_btns = ttk.Frame(folder_actions_group, style="Card.TFrame")
        folder_btns.pack(fill=tk.X)
        
        self.btn_open_folder = ttk.Button(folder_btns, text="📂 Open", style="Open.TButton", command=self.open_current_folder)
        self.btn_open_folder.pack(side=tk.LEFT, padx=(0, 10))

        self.btn_delete_folder = ttk.Button(folder_btns, text="🗑 Delete", style="Delete.TButton", command=self.delete_current_folder)
        self.btn_delete_folder.pack(side=tk.LEFT, padx=(0, 10))
        
        self.btn_process = ttk.Button(folder_btns, text="Process", style="Process.TButton", command=self.process_current_zip)
        self.btn_process.pack(side=tk.LEFT, ipady=4, ipadx=10)

        # Keyboard Loop
        for btn in [self.btn_open_folder, self.btn_delete_folder, self.btn_process, self.btn_save_config, self.btn_manage_configs, self.btn_open_local_log, self.btn_create_receipt]:
            btn.bind('<Return>', lambda e, b=btn: self.invoke_btn(b))

        self.btn_open_folder.bind('<FocusIn>', lambda e: self.refresh_btn_text(self.btn_open_folder))
        self.btn_open_folder.bind('<FocusOut>', lambda e: self.refresh_btn_text(self.btn_open_folder))
        self.btn_delete_folder.bind('<FocusIn>', lambda e: self.refresh_btn_text(self.btn_delete_folder))
        self.btn_delete_folder.bind('<FocusOut>', lambda e: self.refresh_btn_text(self.btn_delete_folder))
        self.btn_process.bind('<FocusIn>', lambda e: self.refresh_btn_text(self.btn_process))
        self.btn_process.bind('<FocusOut>', lambda e: self.refresh_btn_text(self.btn_process))
        self.btn_save_config.bind('<FocusIn>', lambda e: self.refresh_btn_text(self.btn_save_config))
        self.btn_save_config.bind('<FocusOut>', lambda e: self.refresh_btn_text(self.btn_save_config))
        self.btn_manage_configs.bind('<FocusIn>', lambda e: self.refresh_btn_text(self.btn_manage_configs))
        self.btn_manage_configs.bind('<FocusOut>', lambda e: self.refresh_btn_text(self.btn_manage_configs))
        self.btn_open_local_log.bind('<FocusIn>', lambda e: self.refresh_btn_text(self.btn_open_local_log))
        self.btn_open_local_log.bind('<FocusOut>', lambda e: self.refresh_btn_text(self.btn_open_local_log))
        self.btn_create_receipt.bind('<FocusIn>', lambda e: self.refresh_btn_text(self.btn_create_receipt))
        self.btn_create_receipt.bind('<FocusOut>', lambda e: self.refresh_btn_text(self.btn_create_receipt))

        self.placeholder_frame.pack(fill=tk.BOTH, expand=True)
        self.apply_workspace()

    def show_status_popup(self, title, message):
        bg_col = "#1e1e1e" if self.is_dark_mode else "#ffffff"
        d = tk.Toplevel(self.root)
        d.overrideredirect(True)
        d.attributes("-topmost", True)
        d.transient(self.root)
        d.grab_set()
        
        w, h = 420, 180
        rw, rh = self.root.winfo_width(), self.root.winfo_height()
        rx, ry = self.root.winfo_rootx(), self.root.winfo_rooty()
        x, y = rx + (rw // 2) - (w // 2), ry + (rh // 2) - (h // 2)
        d.geometry(f"{w}x{h}+{x}+{y}")
        d.configure(bg=bg_col)
        
        c = tk.Frame(d, bg=bg_col, highlightbackground="#2563eb", highlightthickness=2)
        c.pack(fill=tk.BOTH, expand=True)
        i = ttk.Frame(c, style="Card.TFrame", padding=30)
        i.pack(fill=tk.BOTH, expand=True)
        
        lbl_title = ttk.Label(i, text=title, font=("Segoe UI", 11, "bold"))
        lbl_title.pack(pady=(0, 10))
        
        lbl_msg = ttk.Label(i, text=message, justify=tk.CENTER, wraplength=360)
        lbl_msg.pack(expand=True)
        
        d.lift()
        d.focus_force()
        self.root.update()
        return d, lbl_msg

    def show_no_new_folders_popup(self):
        bg_col = "#333333" if self.is_dark_mode else "#323232"
        fg_col = "#ffffff"
        
        popup = tk.Toplevel(self.root)
        popup.overrideredirect(True)
        popup.attributes("-topmost", True)
        
        w, h = 350, 60
        bx = self.btn_refresh.winfo_rootx()
        by = self.btn_refresh.winfo_rooty()
        bw = self.btn_refresh.winfo_width()
        
        x = bx + (bw // 2) - (w // 2)
        y = by - h - 10
        
        popup.geometry(f"{w}x{h}+{x}+{y}")
        popup.configure(bg=bg_col)
        
        frame = tk.Frame(popup, bg=bg_col, highlightbackground="#555555", highlightthickness=1)
        frame.pack(fill=tk.BOTH, expand=True)
        
        font_family = "Segoe UI" if sys.platform == "win32" else "Helvetica"
        lbl = tk.Label(frame, text="No new TRANSFER-folders have been found.", 
                       bg=bg_col, fg=fg_col, font=(font_family, 10), pady=10)
        lbl.pack(expand=True, fill=tk.BOTH)
        
        def dismiss(e=None):
            if popup.winfo_exists():
                popup.destroy()
                
        popup.bind("<Button-1>", dismiss)
        lbl.bind("<Button-1>", dismiss)
        frame.bind("<Button-1>", dismiss)
        
        self.root.after(3500, dismiss)

    def animate_refresh_button(self, frame=0):
        if not getattr(self, '_anim_running', False):
            return
        frames = ["Refreshing.", "Refreshing..", "Refreshing..."]
        if self.btn_refresh.winfo_exists():
            self.btn_refresh.config(text=f"↻ {frames[frame % len(frames)]}")
            self.root.after(400, self.animate_refresh_button, frame + 1)

    def show_welcome_splash(self):
        self.root.update()
        splash = tk.Toplevel(self.root)
        splash.overrideredirect(True)
        splash.attributes("-topmost", True)
        splash.transient(self.root)
        splash.grab_set()
        
        w, h = 625, 500
        rw, rh = self.root.winfo_width(), self.root.winfo_height()
        rx, ry = self.root.winfo_rootx(), self.root.winfo_rooty()
        x, y = rx + (rw // 2) - (w // 2), ry + (rh // 2) - (h // 2)
        splash.geometry(f"{w}x{h}+{x}+{y}")
        
        is_dark = self.is_dark_mode
        bg_color = "#1e1e1e" if is_dark else "#ffffff"
        key_bg, key_fg = ("#333333", "#90caf9") if is_dark else ("#f0f2f5", "#1976d2")
        border_color = "#90caf9" if is_dark else "#1976d2"

        splash.configure(bg=bg_color) 
        container = tk.Frame(splash, bg=bg_color, highlightbackground=border_color, highlightcolor=border_color, highlightthickness=2)
        container.pack(fill=tk.BOTH, expand=True)
        main_frame = ttk.Frame(container, style="Card.TFrame", padding=30)
        main_frame.pack(fill=tk.BOTH, expand=True)

        font_family = "Segoe UI" if sys.platform == "win32" else "Helvetica"
        ttk.Label(main_frame, text="Welcome to Inbox Mover", font=(font_family, self.base_font_size + 8, "bold")).pack(pady=(0, 5))
        
        url_str = "https://github.com/flow4u/InboxMover"
        url_lbl = tk.Label(main_frame, text=url_str, fg="#60a5fa" if is_dark else "#2563eb", bg=bg_color, font=(font_family, self.base_font_size, "underline"), cursor="hand2")
        url_lbl.pack(pady=(0, 15))
        url_lbl.bind("<Button-1>", lambda e: webbrowser.open_new(url_str))

        ttk.Label(main_frame, text="Speed up your workflow using these keyboard shortcuts:", style="CardDim.TLabel").pack(pady=(0, 20))
        grid = ttk.Frame(main_frame, style="Card.TFrame")
        grid.pack(fill=tk.X, padx=10)

        def add_shortcut(row, keys, desc):
            f = ttk.Frame(grid, style="Card.TFrame")
            f.grid(row=row, column=0, sticky=tk.E, pady=10, padx=(0, 20))
            for k in keys:
                lbl = tk.Label(f, text=k, bg=key_bg, fg=key_fg, font=("Courier", self.base_font_size + 1, "bold"), padx=10, pady=4, relief="flat")
                lbl.pack(side=tk.LEFT, padx=3)
            ttk.Label(grid, text=desc, font=(font_family, self.base_font_size + 1)).grid(row=row, column=1, sticky=tk.W, pady=10)

        add_shortcut(0, ["↑", "↓", "←", "→"], "Cycle through pending folders")
        add_shortcut(1, ["Tab", "Shift+Tab"], "Switch focus between main action buttons (Open/Delete/Process)")
        add_shortcut(2, ["Enter"], "Execute the highlighted action")

        bottom_frame = ttk.Frame(main_frame, style="Card.TFrame")
        bottom_frame.pack(side=tk.BOTTOM, fill=tk.X, pady=(20, 0))
        tk.Label(bottom_frame, text="Press Enter to close", font=(font_family, self.base_font_size + 1, "bold"), fg="#10b981" if is_dark else "#059669", bg=bg_color).pack(side=tk.LEFT, anchor=tk.S, pady=(10, 0))

        def close_splash(event=None):
            if splash.winfo_exists():
                splash.grab_release()
                splash.destroy()
            self.root.focus_force()
            target = self.btn_process if str(self.btn_process.cget('state')) == 'normal' else self.btn_open_folder
            self.focus_btn(target)

        btn_ok = ttk.Button(bottom_frame, text="Close", command=close_splash, style="Accent.TButton", padding=(15, 6))
        btn_ok.pack(side=tk.RIGHT)
        splash.focus_force()
        btn_ok.focus_set()
        splash.lift()
        splash.bind('<Return>', close_splash)
        splash.bind('<Escape>', close_splash)

    def create_folder_input(self, parent, label_text, str_var, callback=None):
        frame = ttk.Frame(parent, style="Card.TFrame")
        frame.pack(fill=tk.X, pady=3)
        ttk.Label(frame, text=label_text, width=10, style="Card.TLabel").pack(side=tk.LEFT)
        entry = ttk.Entry(frame, textvariable=str_var, takefocus=0, width=5)
        entry.pack(side=tk.LEFT, fill=tk.X, expand=True, padx=5)
        
        btn_frame = ttk.Frame(frame, style="Card.TFrame")
        btn_frame.pack(side=tk.RIGHT)
        
        def browse():
            folder = filedialog.askdirectory(parent=parent.winfo_toplevel())
            if folder:
                str_var.set(folder)
                if callback: callback()
                    
        def open_dir():
            path = str_var.get()
            if not path or not os.path.isdir(path):
                messagebox.showwarning("Warning", "The specified folder does not exist.", parent=parent.winfo_toplevel())
                return
            if sys.platform == "win32": os.startfile(path)
            elif sys.platform == "darwin": subprocess.call(["open", path])
            else: subprocess.call(["xdg-open", path])

        ttk.Button(btn_frame, text="Browse", width=7, command=browse, takefocus=0).pack(side=tk.LEFT, padx=(0, 2))
        ttk.Button(btn_frame, text="Open", width=5, command=open_dir, takefocus=0).pack(side=tk.LEFT)
        
        if callback:
            entry.bind('<FocusOut>', lambda e: callback())
            entry.bind('<Return>', lambda e: callback())

    def apply_fonts(self):
        style = ttk.Style()
        base = self.base_font_size
        font_family = "Segoe UI" if sys.platform == "win32" else "Helvetica"
        style.configure(".", font=(font_family, base))
        style.configure("TButton", font=(font_family, base))
        style.configure("AppTitle.TLabel", font=(font_family, base + 8, "bold"))
        style.configure("SectionHeader.TLabel", font=(font_family, base, "bold"))
        style.configure("CardTitle.TLabel", font=(font_family, base + 6, "bold"))
        style.configure("CardDim.TLabel", font=(font_family, base))
        style.configure("CardAccent.TLabel", font=(font_family, base, "italic"))
        style.configure("Badge.TLabel", font=(font_family, base, "bold"))
        style.configure("Process.TButton", font=(font_family, base + 2, "bold"))
        self.receipt_text.configure(font=("Courier", base))
        self.receipt_text.tag_configure("warning", font=("Courier", base, "bold"))
        self.queue_listbox.configure(font=(font_family, base))

    def increase_font(self):
        if self.base_font_size < 24:
            self.base_font_size += 1
            self.apply_fonts()
            self.save_settings()

    def decrease_font(self):
        if self.base_font_size > 8:
            self.base_font_size -= 1
            self.apply_fonts()
            self.save_settings()

    def reset_view(self):
        self.base_font_size = 10
        try:
            if self.root.attributes('-zoomed'): self.root.attributes('-zoomed', False)
        except Exception: pass
        try: self.root.state('normal')
        except Exception: pass
        
        sw, sh = self.root.winfo_screenwidth(), self.root.winfo_screenheight()
        tw, th = max(1300, min(1500, sw - 50)), max(750, min(850, sh - 100))
        self.root.minsize(tw, th)
        self.root.maxsize(tw, th)
        self.root.geometry(f"{tw}x{th}")
        self.root.update_idletasks()
        self.root.minsize(1300, 750)
        self.root.maxsize(sw, sh)
        try: self.paned.sashpos(0, int(tw * 0.35))
        except Exception: pass
        try: self.form_paned.sashpos(0, int((tw * 0.65) * 0.5))
        except Exception: pass
        self.apply_fonts()
        self.save_settings()

    def apply_theme(self):
        style = ttk.Style(self.root)
        style.theme_use('clam')
        if self.is_dark_mode:
            bg, card, fg, dim, acc = "#121212", "#1e1e1e", "#e0e0e0", "#9e9e9e", "#90caf9"
            btn, active, eb, border = "#333333", "#424242", "#2c2c2c", "#424242"
            pbg, pact = "#388e3c", "#4caf50"
            badge_bg, badge_fg = "#37474f", "#81d4fa"
            personal_bg, personal_fg = "#2e7d32", "#ffffff"
            team_bg, team_fg = "#1565c0", "#ffffff"
        else:
            bg, card, fg, dim, acc = "#f0f2f5", "#ffffff", "#212121", "#757575", "#1976d2"
            btn, active, eb, border = "#e0e0e0", "#bdbdbd", "#ffffff", "#bdbdbd"
            pbg, pact = "#388e3c", "#2e7d32"
            badge_bg, badge_fg = "#e3f2fd", "#0277bd"
            personal_bg, personal_fg = "#c8e6c9", "#1b5e20"
            team_bg, team_fg = "#bbdefb", "#0d47a1"
            
        self.root.configure(bg=bg)
        self.theme_btn.config(text="☀" if self.is_dark_mode else "☾")
        style.configure(".", background=bg, foreground=fg, fieldbackground=eb)
        style.configure("TFrame", background=bg)
        style.configure("TSeparator", background=border)
        style.configure("TButton", background=btn, foreground=fg, padding=5, borderwidth=0)
        style.map("TButton", background=[('active', active), ('focus', active), ('disabled', bg)], foreground=[('disabled', dim)])
        
        style.configure("Open.TButton", background=btn, foreground=fg, padding=5, borderwidth=0)
        style.map("Open.TButton", background=[('active', acc), ('focus', acc), ('disabled', bg)], foreground=[('active', "#ffffff"), ('focus', "#ffffff"), ('disabled', dim)])
        
        style.configure("Accent.TButton", background="#f57c00", foreground="#ffffff", padding=5, borderwidth=0)
        style.map("Accent.TButton", background=[('active', "#ff9800"), ('focus', "#ff9800")])
        style.configure("Process.TButton", background=pbg, foreground="#ffffff", borderwidth=0)
        style.map("Process.TButton", background=[('active', pact), ('focus', pact), ('disabled', btn)], foreground=[('disabled', dim)])
        dbgh = "#d32f2f" if self.is_dark_mode else "#c62828"
        style.configure("Delete.TButton", background=btn, foreground=fg, padding=5, borderwidth=0)
        style.map("Delete.TButton", background=[('active', dbgh), ('focus', dbgh), ('disabled', bg)], foreground=[('active', "#ffffff"), ('focus', "#ffffff"), ('disabled', dim)])
        style.configure("Card.TFrame", background=card)
        style.configure("Card.TLabel", background=card, foreground=fg)
        style.configure("AppTitle.TLabel", background=bg, foreground=fg)
        style.configure("SectionHeader.TLabel", background=card, foreground=dim)
        style.configure("CardTitle.TLabel", background=card, foreground=fg)
        style.configure("CardDim.TLabel", background=card, foreground=dim)
        style.configure("CardAccent.TLabel", background=card, foreground=acc)
        style.configure("Badge.TLabel", background=eb if self.is_dark_mode else "#e3f2fd", foreground=acc, padding=(6, 2))
        
        style.configure("WSLocal.TLabel", background=personal_bg, foreground=personal_fg, padding=(4, 0))
        style.configure("WSGlobal.TLabel", background=team_bg, foreground=team_fg, padding=(4, 0))
        
        style.configure("Card.TRadiobutton", background=card, foreground=fg)
        style.map("Card.TRadiobutton", background=[('active', card)])
        style.configure("Card.TCheckbutton", background=card, foreground=fg)
        style.map("Card.TCheckbutton", background=[('active', card)])
        style.configure("TEntry", fieldbackground=eb, foreground=fg, bordercolor=border, padding=4)
        style.configure("TCombobox", fieldbackground=eb, background=btn, foreground=fg, arrowcolor=fg, bordercolor=border, padding=3)
        style.map("TCombobox", fieldbackground=[('readonly', eb)], foreground=[('readonly', fg)], selectbackground=[('readonly', acc)], selectforeground=[('readonly', '#ffffff')])
        
        # Define warning tag colors
        warn_fg = "#ef4444" if self.is_dark_mode else "#c62828"
        self.receipt_text.configure(bg=card if self.is_dark_mode else "#f9f9f9", fg=fg, insertbackground=fg, highlightbackground=border)
        self.receipt_text.tag_configure("warning", foreground=warn_fg, font=("Courier", self.base_font_size, "bold"))
        
        if hasattr(self, 'queue_listbox'):
            self.queue_listbox.configure(bg=eb if self.is_dark_mode else "#ffffff", fg=fg, selectbackground=acc, selectforeground="#000000" if self.is_dark_mode else "#ffffff")

    def toggle_theme(self):
        self.is_dark_mode = not self.is_dark_mode
        self.apply_theme()
        self.apply_fonts() 
        self.save_settings()
        self.apply_workspace() # Refresh workspace badge colors

    def save_settings(self):
        self.core.save_app_settings({"dark_mode": self.is_dark_mode, "font_size": self.base_font_size, "window_geometry": self.root.geometry(), "search_folder_1": self.search_folder_1_var.get(), "search_folder_2": self.search_folder_2_var.get(), "use_global": self.core.use_global, "global_dir": self.core.global_dir})

    def on_closing(self):
        """Save settings and close the app."""
        self.save_settings()
        self.root.destroy()
        
    def open_log_folder(self):
        d = os.path.abspath(self.core.config_dir)
        if not os.path.exists(d): os.makedirs(d)
        if sys.platform == "win32": os.startfile(d)
        else: subprocess.call(["open" if sys.platform == "darwin" else "xdg-open", d])

    def view_log(self):
        if not os.path.exists(self.core.log_file):
            messagebox.showinfo("Log Empty", "No log file has been created yet.", parent=self.root)
            return
        
        bg_col = "#1e1e1e" if self.is_dark_mode else "#f9fafb"
        fg_col = "#ffffff" if self.is_dark_mode else "#000000"
        border_col = "#444444" if self.is_dark_mode else "#d1d5db"
        
        w = tk.Toplevel(self.root)
        w.title("Processing Log")
        w.geometry("900x650")
        w.configure(bg=bg_col)
        w.attributes("-topmost", True)
        w.transient(self.root)
        w.grab_set()
        
        f = ttk.Frame(w, padding="15", style="Card.TFrame")
        f.pack(fill=tk.BOTH, expand=True)
        txt = tk.Text(f, wrap=tk.WORD, font=("Courier", self.base_font_size), relief="flat", highlightthickness=1)
        txt.configure(bg=bg_col, fg=fg_col, insertbackground=fg_col, highlightbackground=border_col)
        txt.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
        
        sb = ttk.Scrollbar(f, orient=tk.VERTICAL, command=txt.yview)
        txt.configure(yscrollcommand=sb.set)
        sb.pack(side=tk.RIGHT, fill=tk.Y)
        
        txt.tag_configure("success", foreground="#10b981")
        txt.tag_configure("error", foreground="#ef4444")
        try:
            with open(self.core.log_file, 'r', encoding='utf-8') as file:
                lines = file.readlines()
            if not lines: txt.insert(tk.END, "Log file is currently empty.")
            else:
                for line in reversed(lines):
                    if not line.strip(): continue
                    try:
                        e = json.loads(line)
                        ts, status, folder, cfg = e.get("timestamp", "")[:19], e.get("status", "UNKNOWN"), e.get("folder_name", ""), e.get("config_id", "")
                        h = f"[{ts}] {status} | Config: {cfg} | Folder: {folder}\n"
                        txt.insert(tk.END, h, "success" if status == "SUCCESS" else ("error" if status == "ERROR" else ""))
                        if e.get("message"): txt.insert(tk.END, f"  Message: {e['message']}\n")
                        txt.insert(tk.END, "-" * 80 + "\n\n")
                    except Exception: pass
        except Exception: pass
        txt.config(state=tk.DISABLED)
        w.lift()
        w.focus_force()
        btn_close = ttk.Button(w, text="Close Documentation", command=w.destroy, width=25)
        btn_close.pack(pady=(10, 15))
        btn_close.bind('<Return>', lambda e: btn_close.invoke())

    def open_local_log(self):
        if self.current_index < 0 or not self.folders_data: return
        folder_path = self.folders_data[self.current_index]['folder_path']
        
        # Check for both filename variants
        p1 = os.path.join(folder_path, "Process.log")
        p2 = os.path.join(folder_path, "Inbox Process.log")
        
        active_log = p1 if os.path.exists(p1) else (p2 if os.path.exists(p2) else None)
        
        if not active_log:
            return
        
        bg_col = "#1e1e1e" if self.is_dark_mode else "#f9fafb"
        fg_col = "#ffffff" if self.is_dark_mode else "#000000"
        border_col = "#444444" if self.is_dark_mode else "#d1d5db"
        
        w = tk.Toplevel(self.root)
        w.title(f"Folder Process Log - {os.path.basename(active_log)}")
        w.geometry("1000x700")
        w.configure(bg=bg_col)
        w.attributes("-topmost", True)
        w.transient(self.root)
        w.grab_set()
        
        f = ttk.Frame(w, padding="15", style="Card.TFrame")
        f.pack(fill=tk.BOTH, expand=True)
        
        # Header in popup
        header = ttk.Frame(f, style="Card.TFrame")
        header.pack(fill=tk.X, pady=(0, 10))
        ttk.Label(header, text="Local Audit Trail", font=("Segoe UI", 12, "bold")).pack(side=tk.LEFT)
        ttk.Label(header, text=f"File: {active_log}", style="CardDim.TLabel").pack(side=tk.RIGHT)
        
        txt = tk.Text(f, wrap=tk.WORD, font=("Courier", self.base_font_size), relief="flat", highlightthickness=1)
        txt.configure(bg=bg_col, fg=fg_col, insertbackground=fg_col, highlightbackground=border_col)
        txt.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
        
        sb = ttk.Scrollbar(f, orient=tk.VERTICAL, command=txt.yview)
        txt.configure(yscrollcommand=sb.set)
        sb.pack(side=tk.RIGHT, fill=tk.Y)
        
        # Enhanced syntax highlighting for log readability
        txt.tag_configure("success", foreground="#10b981", font=("Courier", self.base_font_size, "bold"))
        txt.tag_configure("error", foreground="#ef4444", font=("Courier", self.base_font_size, "bold"))
        txt.tag_configure("conflict", foreground="#f59e0b")
        txt.tag_configure("action", foreground="#3b82f6")
        txt.tag_configure("dim", foreground="#6b7280")

        try:
            with open(active_log, 'r', encoding='utf-8') as file:
                lines = file.readlines()
            for line in lines:
                tag = ""
                uline = line.upper()
                if "SUCCESS" in uline: tag = "success"
                elif "ERROR" in uline: tag = "error"
                elif "CONFLICT" in uline: tag = "conflict"
                elif any(x in uline for x in ("EXTRACT:", "COPY:")): tag = "action"
                elif line.startswith("-") or line.startswith("["): tag = "dim"
                
                txt.insert(tk.END, line, tag)
        except Exception as e:
            txt.insert(tk.END, f"Error reading log file: {e}")
            
        txt.config(state=tk.DISABLED)
        w.lift()
        w.focus_force()
        
        footer = ttk.Frame(w, padding=(0, 0, 0, 15), style="Card.TFrame")
        footer.pack(fill=tk.X)
        btn_close = ttk.Button(footer, text="Close Audit", command=w.destroy, width=25)
        btn_close.pack()
        btn_close.bind('<Return>', lambda e: btn_close.invoke())
        w.bind('<Escape>', lambda e: w.destroy())

    def clear_log(self):
        if not os.path.exists(self.core.log_file): return
        if messagebox.askyesno("Confirm Clear", "Delete all processing logs?", parent=self.root):
            open(self.core.log_file, 'w').close()

    def show_help(self):
        bg_col = "#1e1e1e" if self.is_dark_mode else "#f9fafb"
        fg_col = "#cccccc" if self.is_dark_mode else "#212121"
        border_col = "#444444" if self.is_dark_mode else "#d1d5db"
        code_bg = "#333333" if self.is_dark_mode else "#eeeeee"
        header_fg = "#90caf9" if self.is_dark_mode else "#1976d2"
        
        w = tk.Toplevel(self.root)
        w.title("Inbox Mover Documentation")
        w.geometry("950x850")
        w.configure(bg=bg_col)
        w.attributes("-topmost", True)
        w.transient(self.root)
        w.grab_set()
        
        f = ttk.Frame(w, padding="20", style="Card.TFrame")
        f.pack(fill=tk.BOTH, expand=True)
        txt = tk.Text(f, wrap=tk.WORD, relief="flat", highlightthickness=1, padx=20, pady=20)
        txt.configure(bg=bg_col, fg=fg_col, highlightbackground=border_col)
        txt.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
        
        sb = ttk.Scrollbar(f, orient=tk.VERTICAL, command=txt.yview)
        txt.configure(yscrollcommand=sb.set)
        sb.pack(side=tk.RIGHT, fill=tk.Y)
        
        font_family = "Segoe UI" if sys.platform == "win32" else "Helvetica"
        txt.tag_configure("h1", font=(font_family, self.base_font_size + 10, "bold"), foreground=header_fg, spacing1=20, spacing3=12)
        txt.tag_configure("h2", font=(font_family, self.base_font_size + 4, "bold"), foreground=header_fg, spacing1=15, spacing3=8)
        txt.tag_configure("bold", font=(font_family, self.base_font_size, "bold"))
        txt.tag_configure("code", font=("Courier", self.base_font_size), background=code_bg)
        txt.tag_configure("bullet", lmargin1=25, lmargin2=40)
        
        doc = f"""# Inbox Mover v{VERSION} Manual

Inbox Mover processes project folders (starting with `transfer-`) by extracting contents to specified targets based on rules and descriptors.

# ⌨️ Keyboard Support
* **Arrows:** Cycle through pending folders.
* **Tab / Shift-Tab:** Switch focus between action buttons.
* **Enter:** Activate highlighted button (marked with ► arrows).
* **Esc:** Close any popup window.

# Core Logic

## 1. The `receipt.json` File
Inside ZIP or transfer root.
Example:
`{{`
`  "target_folder": "C:/Incoming",`
`  "process_folder": "C:/Archive",`
`  "conflict_resolution": "rename_existing",`
`  "post_processing": "move",`
`  "auto_extract": true`
`}}`

## 2. Valid Receipt Options
* **Conflict Resolution:** `overwrite`, `keep_both`, `rename_existing`.
* **Post Processing:** `leave`, `delete`, `move`.

## 3. Creating Receipts
Use **Create Receipt** button to generate a file based on UI settings.
* Files must be named `receipt*.json` (e.g. `receipt_upload.json`).
* These files automate routing when placed in transfer folders.
* Cross-platform note: Paths starting with `i:`, `z:`, or `/mnt` are considered cross-platform. "Local" paths (like `C:/` or `/home/`) trigger a Pattern Matching fallback if used on the wrong OS.

# Advanced Routing

## Auto-Match Patterns
Route by filename if `permitId` is missing. Enter a glob like `*.dwg`.

# Logging
Audit trails stored in workspace (`process_log.jsonl`) and individual folders (`Process.log`).
"""
        for line in doc.split('\n'):
            if line.startswith('# '): txt.insert(tk.END, line[2:] + "\n", "h1")
            elif line.startswith('## '): txt.insert(tk.END, line[3:] + "\n", "h2")
            elif line.startswith('* '): txt.insert(tk.END, "  • ", "bullet"); self._insert_styled_text(txt, line[2:] + "\n")
            else: self._insert_styled_text(txt, line + "\n")
        txt.config(state=tk.DISABLED); w.lift(); w.focus_force(); btn_close = ttk.Button(w, text="Close Manual", command=w.destroy, width=25)
        btn_close.pack(pady=(10, 15)); btn_close.bind('<Return>', lambda e: btn_close.invoke())

    def _insert_styled_text(self, text_widget, text):
        parts = re.split(r'(\*\*.*?\*\*|`.*?`)', text)
        for part in parts:
            if part.startswith('**') and part.endswith('**'): text_widget.insert(tk.END, part[2:-2], "bold")
            elif part.startswith('`') and part.endswith('`'): text_widget.insert(tk.END, part[1:-1], "code")
            else: text_widget.insert(tk.END, part)

    def bind_keys(self):
        self.root.bind("<Left>", self.prev_zip); self.root.bind("<Right>", self.next_zip); self.root.bind("<Up>", self.prev_zip); self.root.bind("<Down>", self.next_zip)

    def browse_global_dir(self):
        f = filedialog.askdirectory(title="Shared Settings Folder", parent=self.root)
        if f:
            if os.path.basename(f) != "permit_configs": f = os.path.join(f, "permit_configs")
            self.global_dir_var.set(f); self.workspace_mode_var.set("global"); self.apply_workspace()

    def apply_workspace(self, *args):
        nm, ng = self.workspace_mode_var.get(), self.global_dir_var.get().strip()
        old_mode = "global" if self.core.use_global else "local"
        if nm == old_mode and self.core.global_dir == ng:
            return

        self.entry_global_dir.config(state=tk.NORMAL if nm == "global" else tk.DISABLED)
        self.btn_browse_global.config(state=tk.NORMAL if nm == "global" else tk.DISABLED)
        self.core.use_global = (nm == "global")
        self.core.global_dir = ng
        self.core.set_workspace()
        self.save_settings()
        
        if nm == "global":
            self.lbl_active_ws.config(text="TEAM SHARED", style="WSGlobal.TLabel")
        else:
            self.lbl_active_ws.config(text="PERSONAL", style="WSLocal.TLabel")

        if nm == "global" and ng and os.path.isdir(ng) and (old_mode == "local"):
            self.prompt_and_merge_configs(ng)
            
        self.on_search_folder_changed()

    def prompt_and_merge_configs(self, gdir):
        ldir = self.core.local_config_dir
        lc = [f for f in os.listdir(ldir) if f.endswith('.json') and f not in ('app_settings.json', 'patterns.json', 'DEFAULT.json')]
        if not lc: return
        if messagebox.askyesno("Merge", f"Copy {len(lc)} local rules to shared workspace?", parent=self.root):
            for f in lc:
                d = os.path.join(gdir, f)
                if not os.path.exists(d): shutil.copy2(os.path.join(ldir, f), d)
            self.core.reload_cache()

    def on_search_folder_changed(self, startup=False, maintain_selection=False, is_manual_refresh=False):
        f1, f2 = self.search_folder_1_var.get(), self.search_folder_2_var.get()
        s = []
        if f1 and os.path.isdir(f1): s.append(f1)
        if f2 and os.path.isdir(f2): s.append(f2)
        
        old_idx = self.current_index
        old_paths = {d['folder_path'] for d in getattr(self, 'folders_data', [])}

        if is_manual_refresh:
            self._anim_running = True
            self.btn_refresh.config(state=tk.DISABLED)
            self.animate_refresh_button()

        def worker():
            if s:
                new_data = self.core.find_transfer_folders(s)
            else:
                new_data = []
            self.root.after(0, finish_update, new_data)

        def finish_update(new_data):
            if is_manual_refresh:
                self._anim_running = False
                if self.btn_refresh.winfo_exists():
                    self.btn_refresh.config(state=tk.NORMAL, text="↻ Refresh")
                    
            self.folders_data = new_data
            self.queue_listbox.delete(0, tk.END)
            if s:
                for d in self.folders_data: 
                    status_icon = "✓ " if d['can_process'] else "✗ "
                    log_icon = "(L) " if d.get('has_log') else ""
                    self.queue_listbox.insert(tk.END, status_icon + log_icon + d['folder_name'])
                    
                    if not d['can_process']:
                        self.queue_listbox.itemconfig(tk.END, foreground='#ef4444')
            
                if self.folders_data:
                    if maintain_selection and old_idx >= 0:
                        self.current_index = max(0, min(old_idx, len(self.folders_data) - 1))
                    else:
                        self.current_index = 0
                    
                    self.queue_listbox.selection_set(self.current_index)
                    self.nav_count_var.set(f"{len(self.folders_data)} Folders")
                    self.update_display()
                else:
                    self.current_index = -1
                    self.nav_count_var.set("0 Folders")
                    self.clear_zip_display()
            else:
                self.current_index = -1
                self.nav_count_var.set("0 Folders")
                self.clear_zip_display()
            
            self.root.focus_force()
            
            if is_manual_refresh:
                new_paths = {d['folder_path'] for d in self.folders_data}
                if not (new_paths - old_paths):
                    self.show_no_new_folders_popup()

        if is_manual_refresh:
            threading.Thread(target=worker, daemon=True).start()
        else:
            if s:
                new_data = self.core.find_transfer_folders(s)
            else:
                new_data = []
            finish_update(new_data)

    def on_queue_select(self, event):
        sel = self.queue_listbox.curselection()
        if sel:
            self.current_index = sel[0]
            self.update_display()

    def clear_zip_display(self):
        self.detail_container.pack_forget()
        self.placeholder_frame.pack(fill=tk.BOTH, expand=True)
        self.update_nav_buttons()

    def update_display(self):
        if not self.folders_data or self.current_index < 0:
            self.clear_zip_display()
            return
        self.placeholder_frame.pack_forget()
        self.detail_container.pack(fill=tk.BOTH, expand=True, padx=20, pady=20)
        self.queue_listbox.selection_clear(0, tk.END)
        self.queue_listbox.selection_set(self.current_index)
        self.queue_listbox.see(self.current_index)
        cd = self.folders_data[self.current_index]
        self.zip_name_var.set(cd['folder_name'])
        self.inbox_name_var.set(f"Inbox: {os.path.dirname(cd['folder_path'])}")
        self.permit_id_var.set(f"Config: {cd['permitId']}")
        
        p1 = os.path.join(cd['folder_path'], "Process.log")
        p2 = os.path.join(cd['folder_path'], "Inbox Process.log")
        active_local_log = p1 if os.path.exists(p1) else (p2 if os.path.exists(p2) else None)
        
        if active_local_log:
            try:
                with open(active_local_log, 'r', encoding='utf-8') as f: 
                    self.last_processed_var.set("Latest: " + f.readline().split(' | Config: ')[0])
            except Exception: pass
            self.btn_open_local_log.config(state=tk.NORMAL)
        else: 
            self.last_processed_var.set("")
            self.btn_open_local_log.config(state=tk.DISABLED)
        
        self.target_folder_var.set(""); self.target_zip_folder_var.set(""); self.receipt_folder_var.set("")
        self.active_pattern_var.set(""); self.auto_extract_var.set(True)
        
        self._apply_config_mapping(self.core.load_config("DEFAULT"))
        pattern_cfg_found = None
        for p, cfg in self.core.load_patterns().items():
            if any(fnmatch.fnmatch(f, p) for f in cd['file_list']):
                pattern_cfg_found = (p, cfg)
                break
        
        if cd['permitId'] != "DEFAULT":
            self._apply_config_mapping(self.core.load_config(cd['permitId']))
        elif pattern_cfg_found:
            self.active_pattern_var.set(pattern_cfg_found[0])
            self._apply_config_mapping(pattern_cfg_found[1])
            
        r = cd.get('receipt') or {}
        receipt_warning = ""
        use_receipt = True
        
        if r:
            is_win = (sys.platform == "win32")
            def is_incompatible_path(p):
                if not p: return False
                p_low = p.lower()
                if p_low.startswith(('i:', 'z:', '/mnt')): return False
                
                looks_like_linux = p_low.startswith('/')
                looks_like_windows = (len(p_low) >= 2 and p_low[0].isalpha() and p_low[1] == ':')
                
                if is_win and looks_like_linux: return True
                if not is_win and looks_like_windows: return True
                return False

            if is_incompatible_path(r.get('target_folder', '')) or is_incompatible_path(r.get('process_folder', '')):
                if pattern_cfg_found:
                    use_receipt = False
                    receipt_warning = "[WARNING: receipt*.json bypassed because it contains local destinations incompatible with this OS. Stored Pattern Matching rule is being used instead.]"
                    self.active_pattern_var.set(pattern_cfg_found[0])
                    self._apply_config_mapping(pattern_cfg_found[1])
                else:
                    receipt_warning = "[WARNING: receipt*.json contains local destinations incompatible with this OS. No Pattern Matching fallback found.]"

        if use_receipt and r:
            if r.get('target_folder'): self.target_folder_var.set(self.core.translate_path(r['target_folder']))
            if r.get('process_folder'): self.target_zip_folder_var.set(self.core.translate_path(r['process_folder']))
            if r.get('receipt_folder'): self.receipt_folder_var.set(self.core.translate_path(r['receipt_folder']))
            if r.get('conflict_resolution'): self.conflict_action_var.set(r['conflict_resolution'])
            if r.get('post_processing'): self.post_action_var.set(r['post_processing'])
            if 'auto_extract' in r: self.auto_extract_var.set(r['auto_extract'])
        
        self.conflict_display_var.set(self.conflict_reverse_map.get(self.conflict_action_var.get(), "Overwrite existing file"))
        self.post_display_var.set(self.post_reverse_map.get(self.post_action_var.get(), "Leave files in place"))
        
        inspector_text = f"FILES:\n{'-'*20}\n" + "\n".join(sorted(cd['file_list'])) + "\n\nRECEIPT:\n" + (cd['receipt_raw'] or "None")
        if receipt_warning:
            inspector_text = f"{receipt_warning}\n{'-'*80}\n" + inspector_text
            
        self.set_receipt_text(inspector_text)
        self.update_nav_buttons()
        self.check_unsaved_changes()

        if cd['can_process']:
            self.btn_process.focus_set()
        else:
            self.btn_delete_folder.focus_set()
        
        for btn in [self.btn_process, self.btn_delete_folder, self.btn_open_folder, self.btn_save_config, self.btn_manage_configs, self.btn_open_local_log]:
            self.refresh_btn_text(btn)

    def _apply_config_mapping(self, cfg):
        if not cfg: return
        for k, v in [('target_folder', self.target_folder_var), 
                    ('target_zip_folder', self.target_zip_folder_var), 
                    ('receipt_folder', self.receipt_folder_var)]:
            if cfg.get(k):
                v.set(self.core.translate_path(cfg[k]))
        
        if cfg.get('conflict_action'): self.conflict_action_var.set(cfg['conflict_action'])
        if cfg.get('post_action'): self.post_action_var.set(cfg['post_action'])
        if 'auto_extract' in cfg: self.auto_extract_var.set(cfg['auto_extract'])

    def set_receipt_text(self, text):
        self.receipt_text.config(state=tk.NORMAL)
        self.receipt_text.delete(1.0, tk.END)
        self.receipt_text.insert(tk.END, text)
        if "[WARNING" in text:
            start_search = "1.0"
            while True:
                start = self.receipt_text.search("[WARNING", start_search, tk.END)
                if not start: break
                end = self.receipt_text.search("]", start, tk.END)
                if not end: break
                self.receipt_text.tag_add("warning", start, f"{end} + 1c")
                start_search = f"{end} + 1c"
        self.receipt_text.config(state=tk.DISABLED)

    def update_nav_buttons(self):
        has = len(self.folders_data) > 0
        cp = has and self.current_index >= 0 and self.folders_data[self.current_index]['can_process']
        self.btn_process.config(state=tk.NORMAL if cp else tk.DISABLED)
        self.btn_save_config.config(state=tk.NORMAL if cp else tk.DISABLED)
        self.btn_delete_folder.config(state=tk.NORMAL if has else tk.DISABLED)
        self.btn_open_folder.config(state=tk.NORMAL if has else tk.DISABLED)
        for b in [self.btn_process, self.btn_save_config, self.btn_delete_folder, self.btn_open_folder, self.btn_manage_configs, self.btn_open_local_log]:
            self.refresh_btn_text(b)

    def check_unsaved_changes(self, *args):
        if self.current_index < 0: return
        cd = self.folders_data[self.current_index]
        cur = {"target_folder": self.target_folder_var.get(), "target_zip_folder": self.target_zip_folder_var.get(), "receipt_folder": self.receipt_folder_var.get(), "conflict_action": self.conflict_action_var.get(), "post_action": self.post_action_var.get(), "auto_extract": self.auto_extract_var.get()}
        ap = self.active_pattern_var.get().strip()
        saved = self.core.load_patterns().get(ap) if ap else self.core.load_config(cd['permitId'])
        
        if saved:
            translated_saved = {
                "target_folder": self.core.translate_path(saved.get("target_folder", "")),
                "target_zip_folder": self.core.translate_path(saved.get("target_zip_folder", "")),
                "receipt_folder": self.core.translate_path(saved.get("receipt_folder", "")),
                "conflict_action": saved.get("conflict_action", "overwrite"),
                "post_action": saved.get("post_action", "leave"),
                "auto_extract": saved.get("auto_extract", True)
            }
        else:
            translated_saved = {"target_folder": "", "target_zip_folder": "", "receipt_folder": "", "conflict_action": "overwrite", "post_action": "leave", "auto_extract": True}

        unsaved = (cur != translated_saved)
        self.btn_save_config.config(style="Accent.TButton" if unsaved else "TButton", text="💾 Save *" if unsaved else "💾 Save")
        self.refresh_btn_text(self.btn_save_config)

    def prev_zip(self, e=None):
        if self.root.focus_get() != self.queue_listbox and self.current_index > 0:
            self.current_index -= 1; self.update_display()

    def next_zip(self, e=None):
        if self.root.focus_get() != self.queue_listbox and self.current_index < len(self.folders_data) - 1:
            self.current_index += 1; self.update_display()

    def save_permit_config(self):
        if self.current_index < 0: return
        cd = self.folders_data[self.current_index]
        config_data = {"target_folder": self.target_folder_var.get(), "target_zip_folder": self.target_zip_folder_var.get(), "receipt_folder": self.receipt_folder_var.get(), "conflict_action": self.conflict_action_var.get(), "post_action": self.post_action_var.get(), "auto_extract": self.auto_extract_var.get()}
        pattern = self.active_pattern_var.get().strip()
        try:
            if pattern:
                self.core.save_pattern(pattern, config_data)
                messagebox.showinfo("Success", f"Pattern Rule '{pattern}' saved.", parent=self.root)
            else:
                permit_id = cd.get('permitId', 'DEFAULT')
                self.core.save_config(permit_id, config_data)
                messagebox.showinfo("Success", f"Config Rule for '{permit_id}' saved.", parent=self.root)
            self.check_unsaved_changes()
        except Exception as e:
            messagebox.showerror("Error", f"Failed to save rule: {e}", parent=self.root)

    def delete_selected_rule(self):
        ap = self.active_pattern_var.get().strip()
        if ap: self.delete_current_pattern()
        else: self.delete_current_config()

    def delete_current_config(self):
        if self.current_index < 0: return
        pid = self.folders_data[self.current_index]['permitId']
        if pid and pid != "DEFAULT" and messagebox.askyesno("Delete", f"Delete rule for {pid}?", parent=self.root):
            self.core.delete_config(pid); self.update_display()

    def delete_current_pattern(self):
        ap = self.active_pattern_var.get().strip()
        if ap and messagebox.askyesno("Delete", f"Delete pattern {ap}?", parent=self.root):
            self.core.delete_pattern(ap); self.active_pattern_var.set(""); self.update_display()

    def delete_current_folder(self):
        """Delete folder in a background thread with a status popup."""
        if self.current_index < 0: return
        cd = self.folders_data[self.current_index]
        if self.ask_custom_delete_confirmation(cd['folder_name']):
            popup, lbl = self.show_status_popup("Action in Progress", f"Permanently deleting folder:\n{cd['folder_name']}...")
            def worker():
                try:
                    shutil.rmtree(cd['folder_path'])
                    self.root.after(0, lambda: [popup.destroy(), self.on_search_folder_changed(maintain_selection=True), self.root.focus_force()])
                except Exception as e:
                    self.root.after(0, lambda: [popup.destroy(), messagebox.showerror("Error", str(e), parent=self.root), self.root.focus_force()])
            threading.Thread(target=worker, daemon=True).start()

    def open_current_folder(self):
        if self.current_index < 0: return
        path = self.folders_data[self.current_index]['folder_path']
        if os.path.isdir(path):
            if sys.platform == "win32": os.startfile(path)
            else: subprocess.call(["open" if sys.platform == "darwin" else "xdg-open", path])

    def ask_custom_delete_confirmation(self, name):
        self.root.update()
        bg_col = "#1e1e1e" if self.is_dark_mode else "#ffffff"
        d = tk.Toplevel(self.root)
        d.overrideredirect(True)
        d.attributes("-topmost", True)
        d.transient(self.root)
        d.grab_set()
        
        w, h = 500, 220
        d.geometry(f"{w}x{h}+{self.root.winfo_rootx()+(self.root.winfo_width()//2)-(w//2)}+{self.root.winfo_rooty()+(self.root.winfo_height()//2)-(h//2)}")
        d.configure(bg=bg_col)
        c = tk.Frame(d, bg=bg_col, highlightbackground="#e53935", highlightthickness=2); c.pack(fill=tk.BOTH, expand=True)
        i = ttk.Frame(c, style="Card.TFrame", padding=20); i.pack(fill=tk.BOTH, expand=True)
        ttk.Label(i, text="PERMANENT DELETION", font=("Segoe UI", 12, "bold"), foreground="#e53935").pack(pady=(0, 10))
        ttk.Label(i, text=f"Delete folder: {name}?", justify=tk.CENTER).pack(pady=5)
        bf = ttk.Frame(i, style="Card.TFrame"); bf.pack(side=tk.BOTTOM, fill=tk.X, pady=(15, 0))
        res = [False]
        def set_r(v): res[0] = v; d.grab_release(); d.destroy()
        def up(b, t):
            if d.winfo_exists(): b.config(text=f"► {t} ◄" if d.focus_get()==b else t)
        bc = ttk.Button(bf, text="Cancel", command=lambda: set_r(False)); bd = ttk.Button(bf, text="Delete", style="Delete.TButton", command=lambda: set_r(True)); bc.pack(side=tk.LEFT, expand=True, padx=5); bd.pack(side=tk.LEFT, expand=True, padx=5)
        
        bc.bind('<Return>', lambda e: bc.invoke())
        bd.bind('<Return>', lambda e: bd.invoke())
        bc.bind('<FocusIn>', lambda e: up(bc, "Cancel")); bc.bind('<FocusOut>', lambda e: up(bc, "Cancel"))
        bd.bind('<FocusIn>', lambda e: up(bd, "Delete")); bd.bind('<FocusOut>', lambda e: up(bd, "Delete"))
        
        # Arrow key navigation for modal buttons
        d.bind('<Left>', lambda e: bc.focus_set())
        d.bind('<Right>', lambda e: bd.focus_set())
        
        d.lift(); d.focus_force(); bc.focus_set(); up(bc, "Cancel"); up(bd, "Delete")
        d.bind('<Return>', lambda e: d.focus_get().invoke())
        d.bind('<Escape>', lambda e: set_r(False))
        self.root.wait_window(d); self.root.focus_force() 
        return res[0]

    def open_manage_configs(self):
        # Initial refresh
        self.core.reload_cache()
        
        bg_col = "#1e1e1e" if self.is_dark_mode else "#f0f2f5"
        fg_col = "#e0e0e0" if self.is_dark_mode else "#212121"
        list_bg = "#2c2c2c" if self.is_dark_mode else "#ffffff"
        
        w = tk.Toplevel(self.root)
        w.title("Manage Rules")
        w.geometry("1050x750")
        w.configure(bg=bg_col)
        w.attributes("-topmost", True)
        w.transient(self.root)
        w.grab_set()
        
        # Header Area with Switcher
        header_area = ttk.Frame(w, padding=(15, 15, 15, 0), style="Card.TFrame")
        header_area.pack(fill=tk.X)
        
        ttk.Label(header_area, text="Editing Rules Workspace:", font=("Segoe UI", 10, "bold")).pack(side=tk.LEFT)
        
        pop_ws_var = tk.StringVar(value=self.workspace_mode_var.get())
        
        def on_popup_ws_switch():
            new_mode = pop_ws_var.get()
            # Sync main UI variable (triggers apply_workspace)
            self.workspace_mode_var.set(new_mode)
            # update path label in popup
            update_popup_path_label()
            refresh_lists()

        # Workspace Toggle inside popup
        rb_p = ttk.Radiobutton(header_area, text="Personal (Local)", variable=pop_ws_var, value="local", command=on_popup_ws_switch, style="Card.TRadiobutton")
        rb_p.pack(side=tk.LEFT, padx=(15, 10))
        rb_t = ttk.Radiobutton(header_area, text="Team Shared", variable=pop_ws_var, value="global", command=on_popup_ws_switch, style="Card.TRadiobutton")
        rb_t.pack(side=tk.LEFT, padx=10)
        
        lbl_path = ttk.Label(header_area, text="", style="CardDim.TLabel")
        lbl_path.pack(side=tk.RIGHT)
        
        def update_popup_path_label():
            if pop_ws_var.get() == "global":
                lbl_path.config(text=f"Location: {self.core.global_dir}")
            else:
                lbl_path.config(text="Location: Local Application Data")
        
        update_popup_path_label()

        mp = ttk.PanedWindow(w, orient=tk.HORIZONTAL); mp.pack(fill=tk.BOTH, expand=True, padx=15, pady=15)
        lf = ttk.Frame(mp, style="Card.TFrame", padding=10); mp.add(lf, weight=1)
        ttk.Label(lf, text="Config IDs", style="Header.TLabel").pack(anchor=tk.W)
        self.config_listbox = tk.Listbox(lf, font=("Segoe UI", self.base_font_size), bg=list_bg, fg=fg_col, selectbackground="#2563eb", activestyle="none", highlightthickness=0)
        self.config_listbox.pack(fill=tk.BOTH, expand=True)
        ttk.Separator(lf, orient='horizontal').pack(fill=tk.X, pady=10)
        ttk.Label(lf, text="Patterns", style="Header.TLabel").pack(anchor=tk.W)
        self.pattern_listbox = tk.Listbox(lf, font=("Segoe UI", self.base_font_size), bg=list_bg, fg=fg_col, selectbackground="#2563eb", activestyle="none", highlightthickness=0)
        self.pattern_listbox.pack(fill=tk.BOTH, expand=True)
        rf = ttk.Frame(mp, style="Card.TFrame", padding=20); mp.add(rf, weight=3)
        
        mid, mt, mz, mr, mc, mpv, ma = tk.StringVar(), tk.StringVar(), tk.StringVar(), tk.StringVar(), tk.StringVar(value="Overwrite existing file"), tk.StringVar(value="Leave files in place"), tk.BooleanVar(value=True)
        at = tk.StringVar(value="config")
        
        def make_r(p, l, v, d=True):
            r = ttk.Frame(p, style="Card.TFrame"); r.pack(fill=tk.X, pady=5)
            lbl = ttk.Label(r, text=l, width=20, style="Card.TLabel"); lbl.pack(side=tk.LEFT)
            tk.Entry(r, textvariable=v, bg=list_bg, fg=fg_col, insertbackground=fg_col, borderwidth=1).pack(side=tk.LEFT, fill=tk.X, expand=True, padx=(0, 5))
            if d: 
                def rule_browse():
                    folder = filedialog.askdirectory(parent=w)
                    if folder: v.set(folder)
                ttk.Button(r, text="Browse", command=rule_browse).pack(side=tk.LEFT)
            return lbl

        ilbl = make_r(rf, "ID / Pattern:", mid, False)
        make_r(rf, "Target:", mt); make_r(rf, "Processed:", mz); make_r(rf, "Receipt:", mr)
        
        def refresh_lists():
            self.config_listbox.delete(0, tk.END)
            for p in sorted(self.core.get_all_configs().keys()):
                self.config_listbox.insert(tk.END, p)
            self.pattern_listbox.delete(0, tk.END)
            for p in sorted(self.core.load_patterns().keys()):
                self.pattern_listbox.insert(tk.END, p)
            # Clear entries
            mid.set(""); mt.set(""); mz.set(""); mr.set("")

        def load_sel(e, mode):
            at.set(mode)
            ilbl.config(text="Config ID:" if mode=="config" else "Pattern:")
            lb = self.config_listbox if mode=="config" else self.pattern_listbox
            if not lb.curselection(): return
            (self.pattern_listbox if mode=="config" else self.config_listbox).selection_clear(0, tk.END)
            p = lb.get(lb.curselection()[0])
            c = (self.core.get_all_configs() if mode=="config" else self.core.load_patterns()).get(p, {})
            mid.set(p)
            mt.set(self.core.translate_path(c.get('target_folder','')))
            mz.set(self.core.translate_path(c.get('target_zip_folder','')))
            mr.set(self.core.translate_path(c.get('receipt_folder','')))
            
        self.config_listbox.bind('<<ListboxSelect>>', lambda e: load_sel(e, "config"))
        self.pattern_listbox.bind('<<ListboxSelect>>', lambda e: load_sel(e, "pattern"))
        
        # Footer Action Bar
        footer = ttk.Frame(w, padding=(15, 0, 15, 15), style="Card.TFrame")
        footer.pack(fill=tk.X, side=tk.BOTTOM)
        
        def sv():
            p = mid.get().strip()
            if not p: return
            c = {"target_folder": mt.get(), "target_zip_folder": mz.get(), "receipt_folder": mr.get(), "conflict_action": self.conflict_map.get(mc.get(),"overwrite"), "post_action": self.post_map.get(mpv.get(),"leave"), "auto_extract": ma.get()}
            if at.get()=="config": self.core.save_config(p, c)
            else: self.core.save_pattern(p, c)
            refresh_lists()

        btn_close = ttk.Button(footer, text="Close", command=w.destroy, width=15)
        btn_close.pack(side=tk.RIGHT, padx=5)
        
        btn_save = ttk.Button(footer, text="Save Rule", command=sv, width=15, style="Accent.TButton")
        btn_save.pack(side=tk.RIGHT, padx=5)
        
        w.bind('<Escape>', lambda e: w.destroy())
        refresh_lists()
        self.root.wait_window(w)
        self.root.focus_force()

    def create_receipt_file(self):
        """Allow user to create and save a new receipt*.json file based on current GUI state."""
        receipt_data = {
            "target_folder": self.target_folder_var.get(),
            "process_folder": self.target_zip_folder_var.get(),
            "receipt_folder": self.receipt_folder_var.get(),
            "conflict_resolution": self.conflict_action_var.get(),
            "post_processing": self.post_action_var.get(),
            "auto_extract": self.auto_extract_var.get()
        }
        
        # Determine default directory (z:\ or Linux equivalent)
        default_dir = self.core.translate_path("z:/")
        if not os.path.isdir(default_dir):
            default_dir = os.path.expanduser("~")

        while True:
            file_path = filedialog.asksaveasfilename(
                parent=self.root,
                title="Create Receipt File",
                initialdir=default_dir,
                initialfile="receipt.json",
                defaultextension=".json",
                filetypes=[("JSON files", "*.json")]
            )

            if not file_path:
                return # User cancelled

            filename = os.path.basename(file_path).lower()
            if fnmatch.fnmatch(filename, "receipt*.json"):
                try:
                    with open(file_path, 'w', encoding='utf-8') as f:
                        json.dump(receipt_data, f, indent=4)
                    messagebox.showinfo("Success", f"Receipt file created:\n{file_path}", parent=self.root)
                    return
                except Exception as e:
                    messagebox.showerror("Error", f"Failed to save file: {e}", parent=self.root)
                    return
            else:
                messagebox.showwarning("Invalid Name", "The filename must begin with 'receipt' and end with '.json'\n(e.g., receipt_test.json)", parent=self.root)

    def process_current_zip(self):
        """Process ZIP contents in a background thread with a live status popup."""
        if self.current_index < 0: return
        c = {"target_folder": self.target_folder_var.get(), "target_zip_folder": self.target_zip_folder_var.get(), "receipt_folder": self.receipt_folder_var.get(), "conflict_action": self.conflict_action_var.get(), "post_action": self.post_action_var.get(), "auto_extract": self.auto_extract_var.get()}
        if not c['target_folder']: return
        
        folder_name = self.folders_data[self.current_index]['folder_name']
        popup, lbl_msg = self.show_status_popup("Action in Progress", f"Extracting contents for:\n{folder_name}...")
        self.btn_process.config(state=tk.DISABLED, text="Processing...")
        
        def update_status(current, total, filename):
            msg = f"Processing {current}/{total} files...\nCurrently: {filename}"
            self.root.after(0, lambda: lbl_msg.config(text=msg))

        def worker():
            try:
                self.core.process_zip(
                    self.folders_data[self.current_index], 
                    c, 
                    progress_callback=update_status,
                    password_callback=lambda z: simpledialog.askstring("PWD", f"PWD for {z}:", show='*', parent=self.root)
                )
                self.root.after(0, lambda: [popup.destroy(), self.on_process_success(), self.root.focus_force()])
            except Exception as e:
                self.root.after(0, lambda: [popup.destroy(), messagebox.showerror("Error", str(e), parent=self.root), self.btn_process.config(text="Process", state=tk.NORMAL), self.root.focus_force()])
        
        threading.Thread(target=worker, daemon=True).start()

    def on_process_success(self):
        messagebox.showinfo("Done", "Success!", parent=self.root)
        self.btn_process.config(text="Process")
        self.on_search_folder_changed(maintain_selection=True)
        self.root.focus_force()

    def focus_btn(self, b):
        if str(b.cget('state')) == 'normal': b.focus_set()
        return 'break'

    def invoke_btn(self, b):
        if str(b.cget('state')) == 'normal': b.invoke()
        return 'break'

    def refresh_btn_text(self, b):
        if not hasattr(b, 'cget'): return
        bt = b.cget('text').replace('► ', '').replace(' ◄', '')
        if b == self.btn_process: base = "Process" if "Processing" not in bt else "Processing..."
        elif b == self.btn_delete_folder: base = "🗑 Delete"
        elif b == self.btn_open_folder: base = "📂 Open"
        elif b == self.btn_save_config: base = "💾 Save *" if "*" in bt else "💾 Save"
        elif b == self.btn_manage_configs: base = "⚙ Manage"
        elif b == self.btn_open_local_log: base = "📄 View Log"
        elif b == self.btn_create_receipt: base = "Create Receipt"
        else: base = bt
        
        # Wrapped focus check to prevent KeyError on system dialog names
        try:
            is_focused = (self.root.focus_get() == b)
        except Exception:
            is_focused = False
            
        b.config(text=f"► {base} ◄" if is_focused else base)

def run_cli():
    p = argparse.ArgumentParser(description="CLI")
    p.add_argument('-s', '--search-folders', nargs='+', required=True)
    p.add_argument('-t', '--target-folder', required=True)
    args = p.parse_args()
    c = InboxMoverCore()
    
    translated_search = [c.translate_path(s) for s in args.search_folders]
    f = c.find_transfer_folders(translated_search)
    
    for d in f:
        cfg = {"target_folder": c.translate_path(args.target_folder), "conflict_action": "overwrite", "post_action": "leave", "auto_extract": True}
        c.process_zip(d, cfg)

def main():
    if len(sys.platform) > 0 and len(sys.argv) > 1: run_cli()
    else:
        root = tk.Tk(); app = InboxMoverGUI(root)
        root.lift(); root.attributes('-topmost', True); root.after_idle(root.attributes, '-topmost', False)
        root.mainloop()

if __name__ == '__main__':
    main()