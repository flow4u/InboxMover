#!/usr/bin/env python3
"""
Inbox Mover v0.10
A utility to process and extract zip files containing a receipt.json,
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

# Tkinter imports
import tkinter as tk
from tkinter import ttk, filedialog, messagebox, simpledialog

VERSION = "0.10"
CONFIG_DIR = "permit_configs"

# --------------------------------------------------------------------------- #
# CORE LOGIC (Untouched)
# --------------------------------------------------------------------------- #

class InboxMoverCore:
    def __init__(self):
        self.ensure_config_dir()
        self.log_file = os.path.join(CONFIG_DIR, "process_log.jsonl")

    def ensure_config_dir(self):
        if not os.path.exists(CONFIG_DIR):
            os.makedirs(CONFIG_DIR)

    def load_app_settings(self):
        settings_path = os.path.join(CONFIG_DIR, "app_settings.json")
        if os.path.exists(settings_path):
            try:
                with open(settings_path, 'r') as f:
                    return json.load(f)
            except Exception:
                pass
        return {"dark_mode": True, "font_size": 11, "window_geometry": "1120x950", "search_folder_1": "", "search_folder_2": ""}

    def save_app_settings(self, settings):
        settings_path = os.path.join(CONFIG_DIR, "app_settings.json")
        with open(settings_path, 'w') as f:
            json.dump(settings, f, indent=4)

    def find_transfer_folders(self, search_folders):
        """Find direct subdirectories starting with 'transfer-' and inspect them."""
        folders_data = []
        seen_paths = set()
        
        if isinstance(search_folders, str):
            search_folders = [search_folders]

        for search_folder in search_folders:
            if not search_folder or not os.path.isdir(search_folder):
                continue

            for item in os.listdir(search_folder):
                item_path = os.path.join(search_folder, item)
                if os.path.isdir(item_path) and item.lower().startswith('transfer-'):
                    if item_path not in seen_paths:
                        seen_paths.add(item_path)
                        folder_data = self.inspect_transfer_folder(item_path)
                        folders_data.append(folder_data)
        
        folders_data.sort(key=lambda x: x['folder_name'], reverse=True)
        return folders_data

    def inspect_transfer_folder(self, folder_path):
        """Inspect a transfer folder for a valid zip containing receipt.json."""
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
            "file_list": []
        }
        
        valid_zip_found = False
        file_list = []
        for root, _, files in os.walk(folder_path):
            for file in files:
                rel_file = os.path.relpath(os.path.join(root, file), folder_path)
                file_list.append(rel_file)
                if file.lower().endswith('.zip') and not valid_zip_found:
                    zip_path = os.path.join(root, file)
                    zip_info = self.inspect_zip(zip_path)
                    if zip_info and zip_info.get("receipt_raw"):
                        data["zip_path"] = zip_path
                        data["permitId"] = zip_info["permitId"]
                        data["receipt"] = zip_info["receipt"]
                        data["receipt_raw"] = zip_info["receipt_raw"]
                        data["has_valid_zip"] = True
                        data["can_process"] = True
                        valid_zip_found = True
        
        data["file_list"] = file_list
        
        if not valid_zip_found:
            if not file_list:
                data["receipt_raw"] = "<Folder is empty>"
                data["can_process"] = False
            else:
                data["receipt_raw"] = "NO RECEIPT.JSON FOUND.\nWILL PROCESS ALL FILES IN FOLDER:\n\n" + "\n".join(sorted(file_list))
                data["can_process"] = True
                
        return data

    def inspect_zip(self, zip_path):
        """Read a zip file to extract receipt.json without full extraction."""
        data = {
            "permitId": "DEFAULT",
            "receipt": None,
            "receipt_raw": ""
        }
        try:
            with zipfile.ZipFile(zip_path, 'r') as zf:
                receipt_filename = next((f for f in zf.namelist() if f.endswith('receipt.json')), None)
                if receipt_filename:
                    try:
                        with zf.open(receipt_filename) as f:
                            content = f.read().decode('utf-8')
                            data["receipt_raw"] = content
                            try:
                                receipt_json = json.loads(content)
                                data["receipt"] = receipt_json
                                data["permitId"] = receipt_json.get("permitId", "DEFAULT")
                            except json.JSONDecodeError:
                                pass
                    except RuntimeError:
                        data["receipt_raw"] = "<receipt.json is password protected>"
                return data
        except zipfile.BadZipFile:
            return None
        except Exception as e:
            print(f"Error inspecting {zip_path}: {e}")
            return None

    def load_config(self, permit_id):
        """Load configuration for a specific Config ID."""
        if not permit_id:
            return None
        config_path = os.path.join(CONFIG_DIR, f"{permit_id}.json")
        if os.path.exists(config_path):
            try:
                with open(config_path, 'r') as f:
                    return json.load(f)
            except Exception:
                return None
        return None

    def get_all_configs(self):
        """Return a dictionary of all saved configs (excluding system files)."""
        configs = {}
        if not os.path.exists(CONFIG_DIR):
            return configs
        for f in os.listdir(CONFIG_DIR):
            if f.endswith('.json') and f not in ('app_settings.json', 'patterns.json'):
                permit_id = f[:-5]
                cfg = self.load_config(permit_id)
                if cfg:
                    configs[permit_id] = cfg
        return configs

    def save_config(self, permit_id, config_data):
        """Save configuration for a specific Config ID."""
        if not permit_id:
            raise ValueError("Cannot save configuration without a Config ID.")
        config_path = os.path.join(CONFIG_DIR, f"{permit_id}.json")
        with open(config_path, 'w') as f:
            json.dump(config_data, f, indent=4)

    def delete_config(self, permit_id):
        """Delete a configuration for a specific Config ID."""
        if not permit_id:
            return
        config_path = os.path.join(CONFIG_DIR, f"{permit_id}.json")
        if os.path.exists(config_path):
            os.remove(config_path)

    def load_patterns(self):
        """Load the pattern matching configurations."""
        patterns_path = os.path.join(CONFIG_DIR, "patterns.json")
        if os.path.exists(patterns_path):
            try:
                with open(patterns_path, 'r') as f:
                    return json.load(f)
            except Exception:
                return {}
        return {}

    def save_pattern(self, pattern, config_data):
        """Save a configuration for a specific file pattern."""
        patterns = self.load_patterns()
        patterns[pattern] = config_data
        patterns_path = os.path.join(CONFIG_DIR, "patterns.json")
        with open(patterns_path, 'w') as f:
            json.dump(patterns, f, indent=4)

    def delete_pattern(self, pattern):
        """Delete a configuration for a specific file pattern."""
        patterns = self.load_patterns()
        if pattern in patterns:
            del patterns[pattern]
            patterns_path = os.path.join(CONFIG_DIR, "patterns.json")
            with open(patterns_path, 'w') as f:
                json.dump(patterns, f, indent=4)

    def write_log(self, status, folder_data, config, actions, message=""):
        """Write a structured JSON log entry."""
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
        except Exception as e:
            print(f"Failed to write log: {e}")
            
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
            local_log_path = os.path.join(target_local_dir, "Inbox Process.log")
            
            ts = log_entry["timestamp"].replace("T", " ")[:19]
            lines = [f"[{ts}] {status} | User: {log_entry['user']} | Config: {log_entry['config_id']} | Folder: {log_entry['folder_name']}"]
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
                except Exception as e:
                    print(f"Failed to write local log: {e}")

        for act in actions:
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
                        
                    os.rename(extracted_path, renamed_path)
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
                    
                    # Clean the path and check if it starts with a 'transfer-' root folder
                    safe_name = original_name.lstrip('/\\')
                    parts = safe_name.replace('\\', '/').split('/')
                    
                    if len(parts) > 0 and parts[0].lower().startswith('transfer-'):
                        if len(parts) == 1:
                            continue # Skip the empty root directory itself
                        safe_name = '/'.join(parts[1:]) # Strip the transfer- folder from the path
                    
                    is_absolute = original_name.startswith('/') or original_name.startswith('\\') or (len(original_name) >= 3 and original_name[1] == ':' and original_name[2] in ('/', '\\'))
                    
                    if original_name.lower().endswith('receipt.json'):
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
                            break # Success
                        except RuntimeError as e:
                            err_str = str(e).lower()
                            # RuntimeError handles bad or missing passwords in standard ZipCrypto
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
                        
                    if progress_callback:
                        progress_callback(i + 1, total)

        try:
            if auto_extract and folder_data.get('has_valid_zip') and folder_data.get('zip_path'):
                extract_zip_file(folder_data['zip_path'])
            else:
                folder_path = folder_data.get('folder_path')
                folder_name = folder_data.get('folder_name')
                for root, _, files in os.walk(folder_path):
                    for file in files:
                        src_path = os.path.join(root, file)
                        if auto_extract and file.lower().endswith('.zip'):
                            extract_zip_file(src_path)
                        else:
                            rel_path = os.path.relpath(src_path, folder_path)
                            ext_path = os.path.join(target_folder, rel_path)
                            os.makedirs(os.path.dirname(ext_path), exist_ok=True)
                            final_path = get_final_path(ext_path)
                            shutil.copy2(src_path, final_path)
                            
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
                    
                shutil.move(folder_path, dest_path)
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
# REDESIGNED GUI APPLICATION
# --------------------------------------------------------------------------- #

class InboxMoverGUI:
    def __init__(self, root):
        self.root = root
        self.root.title(f"Inbox Mover v{VERSION}")
        self.root.minsize(920, 850)
        
        self.core = InboxMoverCore()
        settings = self.core.load_app_settings()
        self.is_dark_mode = settings.get("dark_mode", True)
        self.base_font_size = settings.get("font_size", 11)
        
        window_geometry = settings.get("window_geometry", "1120x950")
        self.root.geometry(window_geometry)
        self.root.protocol("WM_DELETE_WINDOW", self.on_closing)
        
        self.folders_data = []
        self.current_index = -1

        # Variables
        sf1 = settings.get("search_folder_1", settings.get("search_folder", ""))
        sf2 = settings.get("search_folder_2", "")
        if not sf1: sf1 = "i:/"
        if not sf2: sf2 = "z:/inbox"

        self.search_folder_1_var = tk.StringVar(value=sf1)
        self.search_folder_2_var = tk.StringVar(value=sf2)
        self.target_folder_var = tk.StringVar()
        self.target_zip_folder_var = tk.StringVar()
        self.receipt_folder_var = tk.StringVar()
        
        self.conflict_action_var = tk.StringVar(value="overwrite")
        self.post_action_var = tk.StringVar(value="leave")
        self.active_pattern_var = tk.StringVar(value="")
        self.auto_extract_var = tk.BooleanVar(value=True)
        
        self.inbox_name_var = tk.StringVar(value="")
        self.zip_name_var = tk.StringVar(value="No Transfer Folders Found")
        self.permit_id_var = tk.StringVar(value="")
        self.last_processed_var = tk.StringVar(value="")
        self.nav_count_var = tk.StringVar(value="[ 0 / 0 ]")

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
            
        # Start the keyboard focus on the Process Folder button
        self.root.after(100, lambda: self.focus_btn(self.btn_process))

    def setup_ui(self):
        self.main_frame = ttk.Frame(self.root, padding=20)
        self.main_frame.pack(fill=tk.BOTH, expand=True)

        # --- HEADER ---
        header_frame = ttk.Frame(self.main_frame)
        header_frame.pack(fill=tk.X, pady=(0, 15))
        
        title_frame = ttk.Frame(header_frame)
        title_frame.pack(side=tk.LEFT)
        self.lbl_title = ttk.Label(title_frame, text="Inbox Mover")
        self.lbl_title.pack(anchor=tk.W)
        self.lbl_version = ttk.Label(title_frame, text=f"- v{VERSION}")
        self.lbl_version.pack(anchor=tk.W)
        
        tools_frame = ttk.Frame(header_frame)
        tools_frame.pack(side=tk.RIGHT, anchor=tk.S)
        self.theme_btn = ttk.Button(tools_frame, text="☀", width=3, command=self.toggle_theme, takefocus=0)
        self.theme_btn.pack(side=tk.RIGHT)
        self.btn_help = ttk.Button(tools_frame, text="?", width=3, command=self.show_help, takefocus=0)
        self.btn_help.pack(side=tk.RIGHT, padx=5)
        self.btn_clear_log = ttk.Button(tools_frame, text="🗑 Clear Log", command=self.clear_log, takefocus=0)
        self.btn_clear_log.pack(side=tk.RIGHT, padx=5)
        self.btn_view_log = ttk.Button(tools_frame, text="📄 View Log", command=self.view_log, takefocus=0)
        self.btn_view_log.pack(side=tk.RIGHT, padx=5)
        self.btn_open_log_folder = ttk.Button(tools_frame, text="📂 Log Folder", command=self.open_log_folder, takefocus=0)
        self.btn_open_log_folder.pack(side=tk.RIGHT, padx=5)
        
        self.btn_increase_font = ttk.Button(tools_frame, text="A+", width=3, command=self.increase_font, takefocus=0)
        self.btn_increase_font.pack(side=tk.RIGHT, padx=2)
        self.btn_reset_view = ttk.Button(tools_frame, text="Reset View", command=self.reset_view, takefocus=0)
        self.btn_reset_view.pack(side=tk.RIGHT, padx=2)
        self.btn_decrease_font = ttk.Button(tools_frame, text="A-", width=3, command=self.decrease_font, takefocus=0)
        self.btn_decrease_font.pack(side=tk.RIGHT, padx=2)

        ttk.Separator(self.main_frame, orient='horizontal').pack(fill=tk.X, pady=(0, 15))

        # --- GLOBAL DIRECTORIES ---
        dirs_wrapper = ttk.Frame(self.main_frame)
        dirs_wrapper.pack(fill=tk.X, pady=(0, 15))
        self.lbl_dirs_header = ttk.Label(dirs_wrapper, text="Global Directories", style="Header.TLabel")
        self.lbl_dirs_header.pack(anchor=tk.W, pady=(0, 10))
        
        dirs_grid = ttk.Frame(dirs_wrapper)
        dirs_grid.pack(fill=tk.X)
        
        self.create_folder_row(dirs_grid, "Search Folder 1:", self.search_folder_1_var, 0, self.on_search_folder_changed)
        self.create_folder_row(dirs_grid, "Search Folder 2:", self.search_folder_2_var, 1, self.on_search_folder_changed)
        self.create_folder_row(dirs_grid, "Target Folder:", self.target_folder_var, 2)
        self.create_folder_row(dirs_grid, "Processed Folder:", self.target_zip_folder_var, 3)
        self.create_folder_row(dirs_grid, "Receipt Folder:", self.receipt_folder_var, 4)

        # --- THE JOB CARD (Highlighted Area) ---
        self.card_frame = ttk.Frame(self.main_frame, style="Card.TFrame", padding=20)
        self.card_frame.pack(fill=tk.BOTH, expand=True)

        # Card Top Bar (Navigation & Utils)
        card_top = ttk.Frame(self.card_frame, style="Card.TFrame")
        card_top.pack(fill=tk.X, pady=(0, 10))
        
        nav_controls = ttk.Frame(card_top, style="Card.TFrame")
        nav_controls.pack(side=tk.LEFT)
        self.btn_refresh = ttk.Button(nav_controls, text="↻ Refresh", command=self.on_search_folder_changed, takefocus=0)
        self.btn_refresh.pack(side=tk.LEFT, padx=(0, 5))
        self.btn_prev = ttk.Button(nav_controls, text="⇦ Prev", width=8, command=self.prev_zip, takefocus=0)
        self.btn_prev.pack(side=tk.LEFT, padx=2)
        self.lbl_nav_count = ttk.Label(nav_controls, textvariable=self.nav_count_var, style="Card.TLabel")
        self.lbl_nav_count.pack(side=tk.LEFT, padx=10)
        self.btn_next = ttk.Button(nav_controls, text="Next ⇨", width=8, command=self.next_zip, takefocus=0)
        self.btn_next.pack(side=tk.LEFT, padx=2)

        utils_controls = ttk.Frame(card_top, style="Card.TFrame")
        utils_controls.pack(side=tk.RIGHT)
        self.btn_open_local_log = ttk.Button(utils_controls, text="📄 Process Log", command=self.open_local_log, takefocus=0)
        self.btn_open_folder = ttk.Button(utils_controls, text="📂 Open Folder", command=self.open_current_folder, takefocus=0)
        # Packed dynamically in update_nav_buttons

        # Card Info
        card_info = ttk.Frame(self.card_frame, style="Card.TFrame")
        card_info.pack(fill=tk.X, pady=(5, 15))
        self.lbl_zip_name = ttk.Label(card_info, textvariable=self.zip_name_var, style="CardTitle.TLabel")
        self.lbl_zip_name.pack(anchor=tk.W)
        
        # Config ID & Manage
        permit_frame = ttk.Frame(card_info, style="Card.TFrame")
        permit_frame.pack(anchor=tk.W, pady=(5, 0), fill=tk.X)
        self.lbl_permit_id = ttk.Label(permit_frame, textvariable=self.permit_id_var, style="Card.TLabel")
        self.lbl_permit_id.pack(side=tk.LEFT)
        
        self.btn_delete_config = ttk.Button(permit_frame, text="🗑 Delete", command=self.delete_current_config, takefocus=0, state=tk.DISABLED)
        self.btn_delete_config.pack(side=tk.LEFT, padx=(10, 0))
        
        self.btn_manage_configs = ttk.Button(permit_frame, text="⚙ Manage", command=self.open_manage_configs, takefocus=0)
        self.btn_manage_configs.pack(side=tk.LEFT, padx=(5, 0))
        
        self.lbl_inbox_name = ttk.Label(card_info, textvariable=self.inbox_name_var, style="CardDim.TLabel")
        self.lbl_inbox_name.pack(anchor=tk.W, pady=(5, 0))
        self.lbl_last_processed = ttk.Label(card_info, textvariable=self.last_processed_var, style="CardAccent.TLabel")
        self.lbl_last_processed.pack(anchor=tk.W, pady=(5, 0))

        # Pattern Matcher Input
        pattern_frame = ttk.Frame(card_info, style="Card.TFrame")
        pattern_frame.pack(anchor=tk.W, pady=(5, 0), fill=tk.X)
        ttk.Label(pattern_frame, text="Auto-Match Pattern:", style="Card.TLabel").pack(side=tk.LEFT, padx=(0, 5))
        self.entry_pattern = ttk.Entry(pattern_frame, textvariable=self.active_pattern_var, width=30, takefocus=0)
        self.entry_pattern.pack(side=tk.LEFT)
        ttk.Label(pattern_frame, text="(e.g., backup*.*)", style="CardDim.TLabel").pack(side=tk.LEFT, padx=(5, 0))
        
        self.btn_delete_pattern = ttk.Button(pattern_frame, text="🗑 Delete", command=self.delete_current_pattern, takefocus=0, state=tk.DISABLED)
        self.btn_delete_pattern.pack(side=tk.LEFT, padx=(10, 0))

        self.btn_manage_patterns = ttk.Button(pattern_frame, text="⚙ Manage", command=self.open_manage_patterns, takefocus=0)
        self.btn_manage_patterns.pack(side=tk.LEFT, padx=(5, 0))

        # Card Options
        card_options = ttk.Frame(self.card_frame, style="Card.TFrame")
        card_options.pack(fill=tk.X, pady=(0, 15))
        
        # New options top row for Checkboxes
        options_top = ttk.Frame(card_options, style="Card.TFrame")
        options_top.pack(fill=tk.X, pady=(0, 10))
        ttk.Checkbutton(options_top, text="Auto-Extract ZIP files", variable=self.auto_extract_var, style="Card.TCheckbutton", takefocus=0).pack(anchor=tk.W)

        # Existing columns
        options_columns = ttk.Frame(card_options, style="Card.TFrame")
        options_columns.pack(fill=tk.X)
        
        conflict_col = ttk.Frame(options_columns, style="Card.TFrame")
        conflict_col.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
        ttk.Label(conflict_col, text="Conflict Resolution:", style="CardHeader.TLabel").pack(anchor=tk.W, pady=(0, 5))
        ttk.Radiobutton(conflict_col, text="Overwrite existing file", variable=self.conflict_action_var, value="overwrite", style="Card.TRadiobutton", takefocus=0).pack(anchor=tk.W)
        ttk.Radiobutton(conflict_col, text="Keep both (add number)", variable=self.conflict_action_var, value="keep_both", style="Card.TRadiobutton", takefocus=0).pack(anchor=tk.W)
        ttk.Radiobutton(conflict_col, text="Rename existing file", variable=self.conflict_action_var, value="rename_existing", style="Card.TRadiobutton", takefocus=0).pack(anchor=tk.W)

        post_col = ttk.Frame(options_columns, style="Card.TFrame")
        post_col.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
        ttk.Label(post_col, text="Post Processing:", style="CardHeader.TLabel").pack(anchor=tk.W, pady=(0, 5))
        ttk.Radiobutton(post_col, text="Leave the files in place", variable=self.post_action_var, value="leave", style="Card.TRadiobutton", takefocus=0).pack(anchor=tk.W)
        ttk.Radiobutton(post_col, text="Delete the files", variable=self.post_action_var, value="delete", style="Card.TRadiobutton", takefocus=0).pack(anchor=tk.W)
        ttk.Radiobutton(post_col, text="Move the files to Processed Folder", variable=self.post_action_var, value="move", style="Card.TRadiobutton", takefocus=0).pack(anchor=tk.W)

        # Card Bottom Bar (Actions) - Pack this FIRST so it anchors to the bottom and never gets pushed off-screen
        card_actions = ttk.Frame(self.card_frame, style="Card.TFrame")
        card_actions.pack(side=tk.BOTTOM, fill=tk.X)
        self.btn_process = ttk.Button(card_actions, text="PROCESS FOLDER", style="Process.TButton", command=self.process_current_zip)
        self.btn_process.pack(side=tk.RIGHT, ipady=4, ipadx=10)
        self.btn_save_config = ttk.Button(card_actions, text="Save Config", command=self.save_permit_config)
        self.btn_save_config.pack(side=tk.RIGHT, padx=10)

        # Keyboard Navigation & Enter Support
        self.btn_process.bind('<Tab>', lambda e: self.focus_btn(self.btn_save_config))
        self.btn_save_config.bind('<Tab>', lambda e: self.focus_btn(self.btn_process))
        self.btn_process.bind('<Shift-Tab>', lambda e: self.focus_btn(self.btn_save_config))
        self.btn_save_config.bind('<Shift-Tab>', lambda e: self.focus_btn(self.btn_process))

        self.btn_process.bind('<Return>', lambda e: self.invoke_btn(self.btn_process))
        self.btn_save_config.bind('<Return>', lambda e: self.invoke_btn(self.btn_save_config))

        self.btn_process.bind('<FocusIn>', lambda e: self.refresh_btn_text(self.btn_process))
        self.btn_process.bind('<FocusOut>', lambda e: self.refresh_btn_text(self.btn_process))
        self.btn_save_config.bind('<FocusIn>', lambda e: self.refresh_btn_text(self.btn_save_config))
        self.btn_save_config.bind('<FocusOut>', lambda e: self.refresh_btn_text(self.btn_save_config))

        # Card Text Area - Pack this LAST with expand=True so it dynamically fills the REMAINING space
        text_wrapper = ttk.Frame(self.card_frame, style="Card.TFrame")
        text_wrapper.pack(side=tk.TOP, fill=tk.BOTH, expand=True, pady=(0, 15))
        ttk.Label(text_wrapper, text="Source Folder Content:", style="CardHeader.TLabel").pack(anchor=tk.W, pady=(0, 5))
        self.receipt_text = tk.Text(text_wrapper, wrap=tk.WORD, state=tk.DISABLED, relief="flat", highlightthickness=1, height=10, takefocus=0)
        scrollbar = ttk.Scrollbar(text_wrapper, orient=tk.VERTICAL, command=self.receipt_text.yview)
        self.receipt_text.configure(yscrollcommand=scrollbar.set)
        self.receipt_text.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
        scrollbar.pack(side=tk.RIGHT, fill=tk.Y)

        self.update_nav_buttons()

    def create_folder_row(self, parent, label_text, str_var, row, callback=None):
        lbl = ttk.Label(parent, text=label_text)
        lbl.grid(row=row, column=0, sticky=tk.W, pady=4, padx=(0, 15))
        
        entry = ttk.Entry(parent, textvariable=str_var, takefocus=0)
        entry.grid(row=row, column=1, sticky=tk.EW, pady=4)
        parent.columnconfigure(1, weight=1)
        
        btn_frame = ttk.Frame(parent)
        btn_frame.grid(row=row, column=2, sticky=tk.E, pady=4, padx=(10, 0))
        
        def browse():
            folder = filedialog.askdirectory()
            if folder:
                str_var.set(folder)
                if callback: callback()
                    
        def open_dir():
            path = str_var.get()
            if not path or not os.path.isdir(path):
                messagebox.showwarning("Warning", "The specified folder does not exist.")
                return
            if sys.platform == "win32": os.startfile(path)
            elif sys.platform == "darwin": subprocess.call(["open", path])
            else: subprocess.call(["xdg-open", path])

        ttk.Button(btn_frame, text="Browse...", command=browse, takefocus=0).pack(side=tk.LEFT, padx=(0, 5))
        ttk.Button(btn_frame, text="Open", width=6, command=open_dir, takefocus=0).pack(side=tk.LEFT)
        
        if callback:
            entry.bind('<FocusOut>', lambda e: callback())
            entry.bind('<Return>', lambda e: callback())

    def apply_fonts(self):
        style = ttk.Style()
        base = self.base_font_size
        font_family = "Segoe UI" if sys.platform == "win32" else "Helvetica"
        
        style.configure(".", font=(font_family, base))
        style.configure("TButton", font=(font_family, base))
        style.configure("Accent.TButton", font=(font_family, base, "bold"))
        
        # New styles based on dynamic sizes
        style.configure("Header.TLabel", font=(font_family, base + 2, "bold"))
        style.configure("CardHeader.TLabel", font=(font_family, base, "bold"))
        style.configure("CardTitle.TLabel", font=(font_family, base + 6, "bold"))
        style.configure("CardDim.TLabel", font=(font_family, base))
        style.configure("CardAccent.TLabel", font=(font_family, base, "italic"))
        style.configure("Process.TButton", font=(font_family, base + 2, "bold"))

        self.lbl_title.config(font=(font_family, base + 14, "bold"))
        self.lbl_version.config(font=(font_family, base, "italic"))
        self.receipt_text.configure(font=("Courier", base))

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
        self.base_font_size = 11
        
        # Force the window to un-maximize if it was maximized/fullscreen
        try:
            # Handle Linux (X11) zoomed state
            if self.root.attributes('-zoomed'):
                self.root.attributes('-zoomed', False)
        except Exception:
            pass
            
        try:
            # Handle Windows/Mac normal state
            self.root.state('normal')
        except Exception:
            pass
            
        # Aggressive resize: Temporarily restrict min/max size to force the OS 
        # Window Manager to strictly respect the new dimensions, even if manually resized.
        self.root.minsize(1120, 950)
        self.root.maxsize(1120, 950)
        
        self.root.geometry("1120x950")
        self.root.update_idletasks() # Force UI to process the forced dimensions
        
        # Restore standard resizability boundaries
        self.root.minsize(920, 850)
        screen_w = self.root.winfo_screenwidth()
        screen_h = self.root.winfo_screenheight()
        self.root.maxsize(screen_w, screen_h)
        
        self.apply_fonts()
        self.save_settings()

    def apply_theme(self):
        style = ttk.Style(self.root)
        style.theme_use('clam')
        
        if self.is_dark_mode:
            bg_color = "#1e1e1e"
            card_bg = "#252526"
            fg_color = "#cccccc"
            fg_dim = "#999999"
            fg_accent = "#60a5fa"
            btn_bg = "#333333"
            btn_active = "#444444"
            entry_bg = "#3c3c3c"
            entry_border = "#444444"
            text_bg = "#1e1e1e"
            
            accent_bg = "#d97706" # Amber
            accent_active = "#f59e0b"
            
            process_bg = "#10b981" # Emerald Green
            process_active = "#059669"
            process_fg = "#ffffff"
            
            self.root.configure(bg=bg_color)
            self.theme_btn.config(text="☀")
        else:
            bg_color = "#f3f4f6"
            card_bg = "#ffffff"
            fg_color = "#111827"
            fg_dim = "#6b7280"
            fg_accent = "#2563eb"
            btn_bg = "#e5e7eb"
            btn_active = "#d1d5db"
            entry_bg = "#ffffff"
            entry_border = "#d1d5db"
            text_bg = "#f9fafb"
            
            accent_bg = "#f59e0b"
            accent_active = "#d97706"
            
            process_bg = "#10b981"
            process_active = "#059669"
            process_fg = "#ffffff"
            
            self.root.configure(bg=bg_color)
            self.theme_btn.config(text="☾")

        # Global Config
        style.configure(".", background=bg_color, foreground=fg_color, fieldbackground=entry_bg)
        style.configure("TFrame", background=bg_color)
        style.configure("TSeparator", background=entry_border)
        
        # Standard Buttons
        style.configure("TButton", background=btn_bg, foreground=fg_color, padding=5, borderwidth=0)
        style.map("TButton", background=[('active', btn_active), ('focus', btn_active), ('disabled', bg_color)], foreground=[('disabled', fg_dim)])
        
        # Action Buttons
        style.configure("Accent.TButton", background=accent_bg, foreground="#ffffff", padding=5, borderwidth=0)
        style.map("Accent.TButton", background=[('active', accent_active), ('focus', accent_active)])
        
        style.configure("Process.TButton", background=process_bg, foreground=process_fg, borderwidth=0)
        style.map("Process.TButton", background=[('active', process_active), ('focus', process_active), ('disabled', btn_bg)], foreground=[('disabled', fg_dim)])

        # Card Styles
        style.configure("Card.TFrame", background=card_bg)
        style.configure("Card.TLabel", background=card_bg, foreground=fg_color)
        style.configure("CardHeader.TLabel", background=card_bg, foreground=fg_color)
        style.configure("CardTitle.TLabel", background=card_bg, foreground=fg_color)
        style.configure("CardDim.TLabel", background=card_bg, foreground=fg_dim)
        style.configure("CardAccent.TLabel", background=card_bg, foreground=fg_accent)
        style.configure("Card.TRadiobutton", background=card_bg, foreground=fg_color)
        style.map("Card.TRadiobutton", background=[('active', card_bg)])
        style.configure("Card.TCheckbutton", background=card_bg, foreground=fg_color)
        style.map("Card.TCheckbutton", background=[('active', card_bg)])

        # Entries, Comboboxes, and Texts
        style.configure("TEntry", fieldbackground=entry_bg, foreground=fg_color, bordercolor=entry_border, lightcolor=entry_border, darkcolor=entry_border)
        
        style.configure("TCombobox", fieldbackground=entry_bg, background=btn_bg, foreground=fg_color, arrowcolor=fg_color, bordercolor=entry_border)
        style.map("TCombobox", fieldbackground=[('readonly', entry_bg)], foreground=[('readonly', fg_color)], selectbackground=[('readonly', fg_accent)], selectforeground=[('readonly', '#ffffff')])
        
        # Style the actual dropdown list attached to the Combobox
        self.root.option_add('*TCombobox*Listbox.background', entry_bg)
        self.root.option_add('*TCombobox*Listbox.foreground', fg_color)
        self.root.option_add('*TCombobox*Listbox.selectBackground', fg_accent)
        self.root.option_add('*TCombobox*Listbox.selectForeground', '#ffffff')
        
        self.receipt_text.configure(bg=text_bg, fg=fg_color, insertbackground=fg_color, highlightbackground=entry_border, highlightcolor=entry_border)

    def toggle_theme(self):
        self.is_dark_mode = not self.is_dark_mode
        self.apply_theme()
        self.apply_fonts() # Reapply to ensure dynamic font mapping sticks
        self.save_settings()

    def save_settings(self):
        self.core.save_app_settings({
            "dark_mode": self.is_dark_mode, 
            "font_size": self.base_font_size,
            "window_geometry": self.root.geometry(),
            "search_folder_1": self.search_folder_1_var.get(),
            "search_folder_2": self.search_folder_2_var.get()
        })

    def on_closing(self):
        self.save_settings()
        self.root.destroy()
        
    def open_log_folder(self):
        log_dir = os.path.abspath(CONFIG_DIR)
        if not os.path.exists(log_dir):
            os.makedirs(log_dir)
        if sys.platform == "win32": os.startfile(log_dir)
        elif sys.platform == "darwin": subprocess.call(["open", log_dir])
        else: subprocess.call(["xdg-open", log_dir])

    def view_log(self):
        if not os.path.exists(self.core.log_file):
            messagebox.showinfo("Log Empty", "No log file has been created yet.")
            return
            
        log_win = tk.Toplevel(self.root)
        log_win.title("Processing Log")
        log_win.geometry("900x600")
        log_win.transient(self.root)
        log_win.grab_set()
        log_win.configure(bg=self.root.cget("bg"))
        
        frame = ttk.Frame(log_win, padding="15")
        frame.pack(fill=tk.BOTH, expand=True)
        
        text_widget = tk.Text(frame, wrap=tk.WORD, font=("Courier", self.base_font_size), relief="flat", highlightthickness=1)
        scrollbar = ttk.Scrollbar(frame, orient=tk.VERTICAL, command=text_widget.yview)
        text_widget.configure(yscrollcommand=scrollbar.set)
        
        if self.is_dark_mode:
            text_widget.configure(bg="#1e1e1e", fg="#ffffff", insertbackground="#ffffff", highlightbackground="#444444")
        else:
            text_widget.configure(bg="#f9fafb", fg="#000000", insertbackground="#000000", highlightbackground="#d1d5db")
            
        text_widget.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
        scrollbar.pack(side=tk.RIGHT, fill=tk.Y)
        
        text_widget.tag_configure("success", foreground="#10b981")
        text_widget.tag_configure("error", foreground="#ef4444")
        text_widget.tag_configure("header", font=("Courier", self.base_font_size, "bold"))
        text_widget.tag_configure("info", foreground="#60a5fa" if self.is_dark_mode else "#2563eb")
        
        try:
            with open(self.core.log_file, 'r', encoding='utf-8') as f:
                lines = f.readlines()
                
            if not lines:
                text_widget.insert(tk.END, "Log file is currently empty.")
            else:
                for line in reversed(lines):
                    if not line.strip(): continue
                    try:
                        entry = json.loads(line)
                        ts = entry.get("timestamp", "").replace("T", " ")[:19]
                        user = entry.get("user", "Unknown")
                        status = entry.get("status", "UNKNOWN")
                        folder = entry.get("folder_name", "")
                        cfg = entry.get("config_id", "")
                        msg = entry.get("message", "")
                        
                        head_str = f"[{ts}] {status} | User: {user} | Config: {cfg} | Folder: {folder}\n"
                        status_tag = "success" if status == "SUCCESS" else ("error" if status == "ERROR" else "header")
                        text_widget.insert(tk.END, head_str, status_tag)
                        if msg: text_widget.insert(tk.END, f"  Message: {msg}\n")
                        
                        actions = entry.get("actions", [])
                        if actions:
                            text_widget.insert(tk.END, "  Actions:\n")
                            for act in actions:
                                a_type = str(act.get("type", "")).upper()
                                a_src = act.get("source", "")
                                a_dest = act.get("destination", "")
                                a_msg = act.get("message", "")
                                if a_type == "CONFLICT_RESOLVED":
                                    text_widget.insert(tk.END, f"    - CONFLICT: {a_src} -> {a_msg}\n", "info")
                                else:
                                    text_widget.insert(tk.END, f"    - {a_type}: {a_src} -> {a_dest}\n")
                        text_widget.insert(tk.END, "-" * 80 + "\n\n")
                    except json.JSONDecodeError:
                        text_widget.insert(tk.END, f"Failed to parse line: {line}\n", "error")
        except Exception as e:
            text_widget.insert(tk.END, f"Error reading log file: {e}\n", "error")
            
        text_widget.config(state=tk.DISABLED)
        ttk.Button(log_win, text="Close", command=log_win.destroy, width=15).pack(pady=(10, 0))

    def open_local_log(self):
        if self.current_index < 0 or not self.folders_data: return
        folder_path = self.folders_data[self.current_index].get('folder_path')
        if not folder_path or not os.path.isdir(folder_path): return
            
        local_log_path = os.path.join(folder_path, "Inbox Process.log")
        if not os.path.exists(local_log_path): return

        log_win = tk.Toplevel(self.root)
        log_win.title("Local Process Log")
        log_win.geometry("900x600")
        log_win.transient(self.root)
        log_win.grab_set()
        log_win.configure(bg=self.root.cget("bg"))
        
        frame = ttk.Frame(log_win, padding="15")
        frame.pack(fill=tk.BOTH, expand=True)
        
        text_widget = tk.Text(frame, wrap=tk.WORD, font=("Courier", self.base_font_size), relief="flat", highlightthickness=1)
        scrollbar = ttk.Scrollbar(frame, orient=tk.VERTICAL, command=text_widget.yview)
        text_widget.configure(yscrollcommand=scrollbar.set)
        
        if self.is_dark_mode:
            text_widget.configure(bg="#1e1e1e", fg="#ffffff", insertbackground="#ffffff", highlightbackground="#444444")
        else:
            text_widget.configure(bg="#f9fafb", fg="#000000", insertbackground="#000000", highlightbackground="#d1d5db")
            
        text_widget.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
        scrollbar.pack(side=tk.RIGHT, fill=tk.Y)
        
        text_widget.tag_configure("success", foreground="#10b981")
        text_widget.tag_configure("error", foreground="#ef4444")
        text_widget.tag_configure("header", font=("Courier", self.base_font_size, "bold"))
        text_widget.tag_configure("info", foreground="#60a5fa" if self.is_dark_mode else "#2563eb")
        
        try:
            with open(local_log_path, 'r', encoding='utf-8') as f:
                lines = f.readlines()
            if not lines:
                text_widget.insert(tk.END, "Log file is currently empty.")
            else:
                for line in lines:
                    if line.startswith("[") and "SUCCESS" in line: text_widget.insert(tk.END, line, "success")
                    elif line.startswith("[") and "ERROR" in line: text_widget.insert(tk.END, line, "error")
                    elif line.startswith("["): text_widget.insert(tk.END, line, "header")
                    elif "CONFLICT" in line: text_widget.insert(tk.END, line, "info")
                    else: text_widget.insert(tk.END, line)
        except Exception as e:
            text_widget.insert(tk.END, f"Error reading log file: {e}\n", "error")
            
        text_widget.config(state=tk.DISABLED)
        ttk.Button(log_win, text="Close", command=log_win.destroy, width=15).pack(pady=(10, 0))

    def clear_log(self):
        if not os.path.exists(self.core.log_file):
            messagebox.showinfo("Log Empty", "The log file is already empty.")
            return
        if messagebox.askyesno("Confirm Clear Log", "Are you sure you want to permanently delete all processing logs?"):
            try:
                open(self.core.log_file, 'w').close()
                messagebox.showinfo("Success", "Log file cleared.")
            except Exception as e:
                messagebox.showerror("Error", f"Failed to clear log:\n{e}")

    def show_help(self):
        help_win = tk.Toplevel(self.root)
        help_win.title("Inbox Mover - Help")
        help_win.geometry("850x700")
        help_win.configure(bg=self.root.cget("bg"))
        
        frame = ttk.Frame(help_win, padding="20")
        frame.pack(fill=tk.BOTH, expand=True)
        
        font_family = "Segoe UI" if sys.platform == "win32" else "Helvetica"
        base = self.base_font_size
        
        help_text = tk.Text(frame, wrap=tk.WORD, relief="flat", highlightthickness=1, padx=15, pady=15)
        scrollbar = ttk.Scrollbar(frame, orient=tk.VERTICAL, command=help_text.yview)
        help_text.configure(yscrollcommand=scrollbar.set)
        
        if self.is_dark_mode:
            help_text.configure(bg="#1e1e1e", fg="#cccccc", insertbackground="#ffffff", highlightbackground="#444444")
            code_bg = "#333333"
            code_fg = "#60a5fa"
            header_fg = "#ffffff"
        else:
            help_text.configure(bg="#f9fafb", fg="#111827", insertbackground="#000000", highlightbackground="#d1d5db")
            code_bg = "#e5e7eb"
            code_fg = "#2563eb"
            header_fg = "#000000"
            
        help_text.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
        scrollbar.pack(side=tk.RIGHT, fill=tk.Y)
        
        # Configure Markdown Tags
        help_text.tag_configure("h1", font=(font_family, base + 8, "bold"), spacing1=15, spacing3=10, foreground=header_fg)
        help_text.tag_configure("h2", font=(font_family, base + 4, "bold"), spacing1=15, spacing3=5, foreground=header_fg)
        help_text.tag_configure("bold", font=(font_family, base, "bold"))
        help_text.tag_configure("bullet", lmargin1=20, lmargin2=35, spacing1=3, spacing3=3)
        help_text.tag_configure("code", font=("Courier", base), background=code_bg, foreground=code_fg)
        help_text.tag_configure("normal", font=(font_family, base), spacing1=3, spacing3=3)
        
        instructions = """# Inbox Mover - User Instructions

## Overview
Inbox Mover processes ZIP files (typically containing a `receipt.json`) by extracting them into a designated target folder while resolving file conflicts automatically.

## ⌨️ Keyboard Support
Inbox Mover is optimized for speed using keyboard shortcuts:
* **Left / Right Arrows:** Quickly cycle through the found transfer folders without using the mouse.
* **Tab / Shift-Tab:** Switch focus instantly between the primary **PROCESS FOLDER** and **Save Config** buttons.
* **Enter:** Activate the currently highlighted action button.

## 1. Directories
* **Search Folder 1 & 2:** The root directories where the app looks for child folders starting with `transfer-`. You can specify up to two search locations.
* **Target Folder:** The directory where the contents of the ZIP will be extracted.
* **Processed Folder:** (Optional) The directory where the original ZIP file is moved if the `Move the files to Processed Folder` post-action is selected.
* **Receipt Folder:** (Optional) A dedicated folder where `receipt.json` will be extracted (prepended with a timestamp).

## 2. Navigation
* Use the **⇦ Prev** and **Next ⇨** buttons (or your keyboard's Left/Right arrow keys) to cycle through the found transfer folders.
* Click **↻ Refresh** to rescan the Search Folder for new or modified transfer folders.

## 3. Conflict Resolution (If file already exists in Target Folder)
* **Overwrite:** Replaces the existing file with the new one from the ZIP.
* **Keep both:** Extracts the new file and adds a number to its filename (e.g., `file (1).txt`).
* **Rename existing:** Renames the file already on your disk by prepending a timestamp (`YYMMDD-HHMMSS_filename`), then extracts the new file normally.

## 4. Post Processing
* **Leave:** Leaves the files in place.
* **Delete:** Permanently deletes the entire transfer folder and all its contents after successful extraction.
* **Move:** Moves the entire transfer folder and all its contents to the Processed Folder.

## 5. Configurations & Config IDs
* The app reads `receipt.json` inside the ZIP to find a **Config ID** (previously Permit ID).
* If no `receipt.json` is found, or it lacks an ID, a **DEFAULT** Config ID is assigned.
* If you set up your folders and rules for a specific Config ID, click **Save Config**.
* The next time you encounter a ZIP with that exact Config ID, the application will automatically load your saved folder paths and conflict/post-action settings.
* **Manage Configs:** Use the **⚙ Manage** button to view, edit, or delete all your saved Config IDs in a dedicated window, or use the **🗑 Delete** button to quickly remove the current one.

## 6. Auto-Match Pattern (Filename Routing)
If a transfer folder doesn't have a `receipt.json` but contains specific files (like database dumps or logs), you can route it based on a filename pattern.
* **How to use:** Enter a wildcard pattern like `backup*.sql` in the **Auto-Match Pattern** field.
* Configure your desired Target Folder and post-actions, then click **Save Config**.
* The next time a transfer folder contains any file matching that pattern (e.g., `backup_2026.sql`), the application will automatically detect it and load those specific settings!
* **Manage Patterns:** Use the **⚙ Manage** button to view, edit, or delete all your saved patterns, or use the **🗑 Delete** button to remove the active pattern.
* *Note: Pattern matching is subordinate to a valid Config ID but overrides the DEFAULT config baseline.*

## 7. Advanced Features
* **Auto-Extract Checkbox:** By default, the app extracts the contents of ZIP files. Uncheck the "Auto-Extract ZIP files" option to simply copy the `.zip` file itself to the target folder instead.
* **Absolute Paths:** If a file inside the ZIP is mapped to an absolute path (e.g., `C:\\logs\\file.txt`), it ignores the Target Folder and extracts directly to that path, creating folders as needed.
* **Receipt Overrides:** If `receipt.json` contains keys like `target_folder`, `process_folder`, `receipt_folder`, `conflict_resolution`, or `post_processing`, these will automatically override your saved GUI settings. The **Save Config** button will turn orange to indicate unsaved changes forced by the receipt.
* **Processing Log:** The application automatically logs every extracted file, moved file, conflict rename, and post-processing action into a machine-readable JSONL file. Click **📄 View Log** to open it in a readable window, or **📂 Log Folder** to browse the files directly.

## CLI Mode
You can also run this application via the command line for automation. Run `python inbox_mover.py --cli --help` in your terminal for details.
"""
        
        def process_inline(text, base_tag):
            # Split by markdown bold (**...**) and code (`...`) tokens
            parts = re.split(r'(\*\*.*?\*\*|`.*?`)', text)
            for part in parts:
                if part.startswith('**') and part.endswith('**'):
                    help_text.insert(tk.END, part[2:-2], (base_tag, "bold"))
                elif part.startswith('`') and part.endswith('`'):
                    help_text.insert(tk.END, part[1:-1], (base_tag, "code"))
                else:
                    help_text.insert(tk.END, part, base_tag)

        # Simple Markdown Parser
        for line in instructions.strip().split('\n'):
            if line.startswith("# "):
                help_text.insert(tk.END, line[2:] + '\n', "h1")
            elif line.startswith("## "):
                help_text.insert(tk.END, line[3:] + '\n', "h2")
            elif line.startswith("* "):
                process_inline("• " + line[2:] + '\n', "bullet")
            else:
                process_inline(line + '\n', "normal")

        help_text.config(state=tk.DISABLED)

    def bind_keys(self):
        self.root.bind("<Left>", lambda e: self.prev_zip())
        self.root.bind("<Right>", lambda e: self.next_zip())

    def on_search_folder_changed(self, startup=False):
        folder1 = self.search_folder_1_var.get()
        folder2 = self.search_folder_2_var.get()
        
        folders_to_search = []
        if folder1 and os.path.isdir(folder1): folders_to_search.append(folder1)
        elif folder1 and not startup: messagebox.showwarning("Warning", f"Search Folder 1 does not exist:\n{folder1}")
            
        if folder2 and os.path.isdir(folder2): folders_to_search.append(folder2)
        elif folder2 and not startup: messagebox.showwarning("Warning", f"Search Folder 2 does not exist:\n{folder2}")

        if folders_to_search:
            self.folders_data = self.core.find_transfer_folders(folders_to_search)
            if self.folders_data:
                self.current_index = 0
            else:
                self.current_index = -1
                self.clear_zip_display()
                if not startup: messagebox.showinfo("Info", "No transfer folders found in the specified search folders.")
            self.update_display()
        else:
            self.folders_data = []
            self.current_index = -1
            self.clear_zip_display()

    def clear_zip_display(self):
        self.inbox_name_var.set("")
        self.zip_name_var.set("No Transfer Folders Found")
        self.permit_id_var.set("")
        self.last_processed_var.set("")
        self.active_pattern_var.set("")
        self.nav_count_var.set("[ 0 / 0 ]")
        self.set_receipt_text("")
        self.update_nav_buttons()

    def update_display(self):
        if not self.folders_data or self.current_index < 0 or self.current_index >= len(self.folders_data):
            self.clear_zip_display()
            return

        current_data = self.folders_data[self.current_index]
        parent_dir = os.path.dirname(current_data['folder_path'])
        
        self.nav_count_var.set(f"[ {self.current_index + 1} / {len(self.folders_data)} ]")
        self.zip_name_var.set(current_data['folder_name'])
        self.inbox_name_var.set(f"Inbox: {parent_dir}")
        self.permit_id_var.set(f"Config ID: {current_data['permitId']}")
        self.set_receipt_text(current_data['receipt_raw'])
        
        local_log_path = os.path.join(current_data['folder_path'], "Inbox Process.log")
        if os.path.exists(local_log_path):
            try:
                with open(local_log_path, 'r', encoding='utf-8') as f:
                    first_line = f.readline().strip()
                    if first_line:
                        display_text = first_line.split(' | Config: ')[0]
                        self.last_processed_var.set(f"Latest: {display_text}")
                    else:
                        self.last_processed_var.set("Latest: [Log is empty]")
            except Exception:
                self.last_processed_var.set("Latest: [Error reading log]")
        else:
            self.last_processed_var.set("")
        
        self.target_folder_var.set("")
        self.target_zip_folder_var.set("")
        self.receipt_folder_var.set("")
        self.conflict_action_var.set("overwrite")
        self.post_action_var.set("leave")
        self.active_pattern_var.set("")
        self.auto_extract_var.set(True)

        # 1. Base Fallback: DEFAULT config
        default_config = self.core.load_config("DEFAULT")
        if default_config:
            if default_config.get('target_folder'): self.target_folder_var.set(default_config['target_folder'])
            if default_config.get('target_zip_folder'): self.target_zip_folder_var.set(default_config['target_zip_folder'])
            if default_config.get('receipt_folder'): self.receipt_folder_var.set(default_config['receipt_folder'])
            if default_config.get('conflict_action'): self.conflict_action_var.set(default_config['conflict_action'])
            if default_config.get('post_action'): self.post_action_var.set(default_config['post_action'])
            if 'auto_extract' in default_config: self.auto_extract_var.set(default_config['auto_extract'])

        # 2. Pattern Match (Subordinate to explicit Permit ID)
        matched_pattern = None
        if current_data['permitId'] == "DEFAULT" and current_data['file_list']:
            patterns = self.core.load_patterns()
            for pattern, p_config in patterns.items():
                if any(fnmatch.fnmatch(f, pattern) for f in current_data['file_list']):
                    matched_pattern = pattern
                    self.active_pattern_var.set(pattern)
                    if p_config.get('target_folder'): self.target_folder_var.set(p_config['target_folder'])
                    if p_config.get('target_zip_folder'): self.target_zip_folder_var.set(p_config['target_zip_folder'])
                    if p_config.get('receipt_folder'): self.receipt_folder_var.set(p_config['receipt_folder'])
                    if p_config.get('conflict_action'): self.conflict_action_var.set(p_config['conflict_action'])
                    if p_config.get('post_action'): self.post_action_var.set(p_config['post_action'])
                    if 'auto_extract' in p_config: self.auto_extract_var.set(p_config['auto_extract'])
                    break # Stop at first matched pattern

        # 3. Config ID (Overrides Pattern Match)
        if current_data['permitId'] != "DEFAULT":
            specific_config = self.core.load_config(current_data['permitId'])
            if specific_config:
                if specific_config.get('target_folder'): self.target_folder_var.set(specific_config['target_folder'])
                if specific_config.get('target_zip_folder'): self.target_zip_folder_var.set(specific_config['target_zip_folder'])
                if specific_config.get('receipt_folder'): self.receipt_folder_var.set(specific_config['receipt_folder'])
                if specific_config.get('conflict_action'): self.conflict_action_var.set(specific_config['conflict_action'])
                if specific_config.get('post_action'): self.post_action_var.set(specific_config['post_action'])
                if 'auto_extract' in specific_config: self.auto_extract_var.set(specific_config['auto_extract'])
            
        # 4. Receipt Overrides (Highest Priority)
        receipt = current_data.get('receipt') or {}
        if receipt.get('target_folder'): self.target_folder_var.set(receipt.get('target_folder'))
        if receipt.get('process_folder'): self.target_zip_folder_var.set(receipt.get('process_folder'))
        if receipt.get('receipt_folder'): self.receipt_folder_var.set(receipt.get('receipt_folder'))
        if receipt.get('conflict_resolution'): self.conflict_action_var.set(receipt.get('conflict_resolution'))
        if receipt.get('post_processing'): self.post_action_var.set(receipt.get('post_processing'))
        if 'auto_extract' in receipt: self.auto_extract_var.set(receipt.get('auto_extract'))
            
        self.update_nav_buttons()
        self.check_unsaved_changes()

    def set_receipt_text(self, text):
        self.receipt_text.config(state=tk.NORMAL)
        self.receipt_text.delete(1.0, tk.END)
        self.receipt_text.insert(tk.END, text if text else "<No receipt.json found in this transfer folder>")
        self.receipt_text.config(state=tk.DISABLED)

    def update_nav_buttons(self):
        has_folders = len(self.folders_data) > 0
        can_process = False
        if has_folders and self.current_index >= 0:
            can_process = self.folders_data[self.current_index].get('can_process', False)

        self.btn_prev.config(state=tk.NORMAL if has_folders and self.current_index > 0 else tk.DISABLED)
        self.btn_next.config(state=tk.NORMAL if has_folders and self.current_index < len(self.folders_data) - 1 else tk.DISABLED)
        
        if hasattr(self, 'btn_save_config'):
            # Setup Action Buttons
            if can_process:
                self.btn_process.config(state=tk.NORMAL, style="Process.TButton", text="PROCESS FOLDER")
            else:
                self.btn_process.config(state=tk.DISABLED, style="TButton", text="PROCESS FOLDER")
            self.refresh_btn_text(self.btn_process)

            self.btn_save_config.config(state=tk.NORMAL if can_process else tk.DISABLED)
            self.refresh_btn_text(self.btn_save_config)

            # Setup Utility Buttons (Top Right of Card)
            self.btn_open_folder.pack_forget()
            self.btn_open_local_log.pack_forget()
            
            self.btn_open_folder.pack(side=tk.RIGHT, padx=(5, 0))
            self.btn_open_folder.config(state=tk.NORMAL if has_folders else tk.DISABLED)
            
            local_log_exists = False
            if has_folders and self.current_index >= 0:
                folder_path = self.folders_data[self.current_index].get('folder_path')
                if folder_path and os.path.exists(os.path.join(folder_path, "Inbox Process.log")):
                    local_log_exists = True
            
            if local_log_exists:
                self.btn_open_local_log.pack(side=tk.RIGHT, padx=(5, 0))

    def open_current_folder(self):
        if self.current_index < 0 or not self.folders_data: return
        folder_path = self.folders_data[self.current_index].get('folder_path')
        if folder_path and os.path.isdir(folder_path):
            if sys.platform == "win32": os.startfile(folder_path)
            elif sys.platform == "darwin": subprocess.call(["open", folder_path])
            else: subprocess.call(["xdg-open", folder_path])

    def check_unsaved_changes(self, *args):
        if not hasattr(self, 'btn_save_config'): return 
            
        if self.current_index < 0 or not self.folders_data:
            self.btn_save_config.config(style="TButton", text="Save Config")
            self.refresh_btn_text(self.btn_save_config)
            return
            
        current_data = self.folders_data[self.current_index]
        permit_id = current_data.get('permitId')
        can_process = current_data.get('can_process', False)
        
        if not permit_id or not can_process:
            self.btn_save_config.config(style="TButton", text="Save Config")
            self.refresh_btn_text(self.btn_save_config)
            return
            
        current_config = {
            "target_folder": self.target_folder_var.get(),
            "target_zip_folder": self.target_zip_folder_var.get(),
            "receipt_folder": self.receipt_folder_var.get(),
            "conflict_action": self.conflict_action_var.get(),
            "post_action": self.post_action_var.get(),
            "auto_extract": self.auto_extract_var.get()
        }

        is_unsaved = False
        active_pattern = self.active_pattern_var.get().strip()
        patterns = self.core.load_patterns()
        
        # Manage Delete button state for patterns
        if hasattr(self, 'btn_delete_pattern'):
            if active_pattern and active_pattern in patterns:
                self.btn_delete_pattern.config(state=tk.NORMAL)
            else:
                self.btn_delete_pattern.config(state=tk.DISABLED)
                
        # Manage Delete button state for configs
        if hasattr(self, 'btn_delete_config'):
            if permit_id and self.core.load_config(permit_id):
                self.btn_delete_config.config(state=tk.NORMAL)
            else:
                self.btn_delete_config.config(state=tk.DISABLED)
        
        # Determine comparison baseline based on what we are saving to
        if active_pattern:
            saved_config = patterns.get(active_pattern)
            if saved_config != current_config:
                is_unsaved = True
        else:
            saved_config = self.core.load_config(permit_id)
            if saved_config is None:
                empty_config = {"target_folder": "", "target_zip_folder": "", "receipt_folder": "", "conflict_action": "overwrite", "post_action": "leave", "auto_extract": True}
                if current_config != empty_config: is_unsaved = True
            else:
                if current_config != saved_config: is_unsaved = True
                
        if is_unsaved: 
            self.btn_save_config.config(style="Accent.TButton", text="Save Config *")
        else: 
            self.btn_save_config.config(style="TButton", text="Save Config")
            
        self.refresh_btn_text(self.btn_save_config)

    def prev_zip(self):
        if self.current_index > 0:
            self.current_index -= 1
            self.update_display()

    def next_zip(self):
        if self.current_index < len(self.folders_data) - 1:
            self.current_index += 1
            self.update_display()

    def delete_current_config(self):
        if self.current_index < 0: return
        permit_id = self.folders_data[self.current_index].get('permitId')
        if not permit_id: return
        
        if messagebox.askyesno("Confirm Delete", f"Are you sure you want to permanently delete the config for:\n\n'{permit_id}'?"):
            try:
                self.core.delete_config(permit_id)
                messagebox.showinfo("Success", f"Config for '{permit_id}' has been deleted.")
                self.update_display() # Reload the display to update unsaved status
            except Exception as e:
                messagebox.showerror("Error", f"Failed to delete config:\n{e}")

    def open_manage_configs(self):
        manage_win = tk.Toplevel(self.root)
        manage_win.title("Manage Saved Configs")
        manage_win.geometry("950x600")
        manage_win.transient(self.root)
        manage_win.grab_set()
        manage_win.configure(bg=self.root.cget("bg"))

        main_paned = ttk.PanedWindow(manage_win, orient=tk.HORIZONTAL)
        main_paned.pack(fill=tk.BOTH, expand=True, padx=15, pady=15)

        # --- Left side: Listbox ---
        left_frame = ttk.Frame(main_paned, style="Card.TFrame", padding=10)
        main_paned.add(left_frame, weight=1)

        ttk.Label(left_frame, text="Saved Config IDs", style="Header.TLabel").pack(anchor=tk.W, pady=(0, 10))
        
        listbox_frame = ttk.Frame(left_frame)
        listbox_frame.pack(fill=tk.BOTH, expand=True)
        
        font_family = "Segoe UI" if sys.platform == "win32" else "Helvetica"
        self.config_listbox = tk.Listbox(listbox_frame, font=(font_family, self.base_font_size), selectbackground="#2563eb", activestyle="none", highlightthickness=0)
        
        if self.is_dark_mode:
            self.config_listbox.configure(bg="#1e1e1e", fg="#cccccc", selectforeground="#ffffff")
        else:
            self.config_listbox.configure(bg="#f9fafb", fg="#111827", selectforeground="#ffffff")
            
        scrollbar = ttk.Scrollbar(listbox_frame, orient=tk.VERTICAL, command=self.config_listbox.yview)
        self.config_listbox.configure(yscrollcommand=scrollbar.set)
        self.config_listbox.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
        scrollbar.pack(side=tk.RIGHT, fill=tk.Y)

        # --- Right side: Form ---
        right_frame = ttk.Frame(main_paned, style="Card.TFrame", padding=20)
        main_paned.add(right_frame, weight=3)

        ttk.Label(right_frame, text="Config Configuration", style="Header.TLabel").pack(anchor=tk.W, pady=(0, 15))

        m_permit_var = tk.StringVar()
        m_target_var = tk.StringVar()
        m_zip_var = tk.StringVar()
        m_receipt_var = tk.StringVar()
        m_conflict_var = tk.StringVar(value="overwrite")
        m_post_var = tk.StringVar(value="leave")
        m_auto_var = tk.BooleanVar(value=True)

        def make_row(parent, label, var, is_dir=True):
            row = ttk.Frame(parent, style="Card.TFrame")
            row.pack(fill=tk.X, pady=5)
            ttk.Label(row, text=label, width=20, style="Card.TLabel").pack(side=tk.LEFT)
            entry = ttk.Entry(row, textvariable=var)
            entry.pack(side=tk.LEFT, fill=tk.X, expand=True, padx=(0, 5))
            if is_dir:
                def browse():
                    d = filedialog.askdirectory(parent=manage_win)
                    if d: var.set(d)
                ttk.Button(row, text="Browse", command=browse, width=8).pack(side=tk.LEFT)
            return row

        make_row(right_frame, "Config ID:", m_permit_var, is_dir=False)
        ttk.Separator(right_frame, orient='horizontal').pack(fill=tk.X, pady=10)
        
        make_row(right_frame, "Target Folder:", m_target_var)
        make_row(right_frame, "Processed Folder:", m_zip_var)
        make_row(right_frame, "Receipt Folder:", m_receipt_var)

        ttk.Separator(right_frame, orient='horizontal').pack(fill=tk.X, pady=10)

        combo_frame = ttk.Frame(right_frame, style="Card.TFrame")
        combo_frame.pack(fill=tk.X, pady=5)
        
        ttk.Label(combo_frame, text="Conflict Action:", width=20, style="Card.TLabel").pack(side=tk.LEFT)
        c_cb = ttk.Combobox(combo_frame, textvariable=m_conflict_var, values=["overwrite", "keep_both", "rename_existing"], state="readonly")
        c_cb.pack(side=tk.LEFT, fill=tk.X, expand=True, padx=(0, 10))
        
        ttk.Label(combo_frame, text="Post Action:", width=15, style="Card.TLabel").pack(side=tk.LEFT)
        p_cb = ttk.Combobox(combo_frame, textvariable=m_post_var, values=["leave", "delete", "move"], state="readonly")
        p_cb.pack(side=tk.LEFT, fill=tk.X, expand=True)

        chk_frame = ttk.Frame(right_frame, style="Card.TFrame")
        chk_frame.pack(fill=tk.X, pady=(15, 5))
        ttk.Checkbutton(chk_frame, text="Auto-Extract ZIP files", variable=m_auto_var, style="Card.TCheckbutton").pack(side=tk.LEFT)

        action_frame = ttk.Frame(right_frame, style="Card.TFrame")
        action_frame.pack(side=tk.BOTTOM, fill=tk.X, pady=(20, 0))

        configs_data = self.core.get_all_configs()
        
        def refresh_list():
            self.config_listbox.delete(0, tk.END)
            for p in sorted(configs_data.keys()):
                self.config_listbox.insert(tk.END, p)

        def on_select(event):
            sel = self.config_listbox.curselection()
            if not sel: return
            p_name = self.config_listbox.get(sel[0])
            cfg = configs_data.get(p_name, {})
            
            m_permit_var.set(p_name)
            m_target_var.set(cfg.get('target_folder', ''))
            m_zip_var.set(cfg.get('target_zip_folder', ''))
            m_receipt_var.set(cfg.get('receipt_folder', ''))
            m_conflict_var.set(cfg.get('conflict_action', 'overwrite'))
            m_post_var.set(cfg.get('post_action', 'leave'))
            m_auto_var.set(cfg.get('auto_extract', True))

        self.config_listbox.bind('<<ListboxSelect>>', on_select)

        def save_config_item():
            p_name = m_permit_var.get().strip()
            if not p_name:
                messagebox.showwarning("Warning", "Config ID cannot be empty.", parent=manage_win)
                return
            
            new_cfg = {
                "target_folder": m_target_var.get(),
                "target_zip_folder": m_zip_var.get(),
                "receipt_folder": m_receipt_var.get(),
                "conflict_action": m_conflict_var.get(),
                "post_action": m_post_var.get(),
                "auto_extract": m_auto_var.get()
            }
            configs_data[p_name] = new_cfg
            self.core.save_config(p_name, new_cfg)
            refresh_list()
            messagebox.showinfo("Success", f"Saved configuration for Config ID '{p_name}'.", parent=manage_win)

        def delete_config_item():
            p_name = m_permit_var.get().strip()
            if not p_name or p_name not in configs_data:
                return
            if messagebox.askyesno("Confirm Delete", f"Are you sure you want to delete the config '{p_name}'?", parent=manage_win):
                del configs_data[p_name]
                self.core.delete_config(p_name)
                m_permit_var.set("")
                m_target_var.set("")
                m_zip_var.set("")
                m_receipt_var.set("")
                refresh_list()
                
        def on_close():
            manage_win.destroy()
            self.update_display() # Refresh main UI

        ttk.Button(action_frame, text="Close", command=on_close).pack(side=tk.RIGHT, padx=(10, 0))
        ttk.Button(action_frame, text="Save Config", style="Accent.TButton", command=save_config_item).pack(side=tk.RIGHT)
        ttk.Button(action_frame, text="Delete", command=delete_config_item).pack(side=tk.LEFT)

        refresh_list()

    def delete_current_pattern(self):
        pattern = self.active_pattern_var.get().strip()
        if not pattern:
            return
            
        patterns = self.core.load_patterns()
        if pattern in patterns:
            if messagebox.askyesno("Confirm Delete", f"Are you sure you want to permanently delete the pattern:\n\n'{pattern}'?"):
                try:
                    self.core.delete_pattern(pattern)
                    messagebox.showinfo("Success", f"Pattern '{pattern}' has been deleted.")
                    self.active_pattern_var.set("")
                    self.update_display() # Reload the display to fall back to DEFAULT or PermitID config
                except Exception as e:
                    messagebox.showerror("Error", f"Failed to delete pattern:\n{e}")

    def open_manage_patterns(self):
        manage_win = tk.Toplevel(self.root)
        manage_win.title("Manage Auto-Match Patterns")
        manage_win.geometry("950x600")
        manage_win.transient(self.root)
        manage_win.grab_set()
        manage_win.configure(bg=self.root.cget("bg"))

        main_paned = ttk.PanedWindow(manage_win, orient=tk.HORIZONTAL)
        main_paned.pack(fill=tk.BOTH, expand=True, padx=15, pady=15)

        # --- Left side: Listbox ---
        left_frame = ttk.Frame(main_paned, style="Card.TFrame", padding=10)
        main_paned.add(left_frame, weight=1)

        ttk.Label(left_frame, text="Saved Patterns", style="Header.TLabel").pack(anchor=tk.W, pady=(0, 10))
        
        listbox_frame = ttk.Frame(left_frame)
        listbox_frame.pack(fill=tk.BOTH, expand=True)
        
        font_family = "Segoe UI" if sys.platform == "win32" else "Helvetica"
        self.pattern_listbox = tk.Listbox(listbox_frame, font=(font_family, self.base_font_size), selectbackground="#2563eb", activestyle="none", highlightthickness=0)
        
        if self.is_dark_mode:
            self.pattern_listbox.configure(bg="#1e1e1e", fg="#cccccc", selectforeground="#ffffff")
        else:
            self.pattern_listbox.configure(bg="#f9fafb", fg="#111827", selectforeground="#ffffff")
            
        scrollbar = ttk.Scrollbar(listbox_frame, orient=tk.VERTICAL, command=self.pattern_listbox.yview)
        self.pattern_listbox.configure(yscrollcommand=scrollbar.set)
        self.pattern_listbox.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
        scrollbar.pack(side=tk.RIGHT, fill=tk.Y)

        # --- Right side: Form ---
        right_frame = ttk.Frame(main_paned, style="Card.TFrame", padding=20)
        main_paned.add(right_frame, weight=3)

        ttk.Label(right_frame, text="Pattern Configuration", style="Header.TLabel").pack(anchor=tk.W, pady=(0, 15))

        m_pattern_var = tk.StringVar()
        m_target_var = tk.StringVar()
        m_zip_var = tk.StringVar()
        m_receipt_var = tk.StringVar()
        m_conflict_var = tk.StringVar(value="overwrite")
        m_post_var = tk.StringVar(value="leave")
        m_auto_var = tk.BooleanVar(value=True)

        def make_row(parent, label, var, is_dir=True):
            row = ttk.Frame(parent, style="Card.TFrame")
            row.pack(fill=tk.X, pady=5)
            ttk.Label(row, text=label, width=20, style="Card.TLabel").pack(side=tk.LEFT)
            entry = ttk.Entry(row, textvariable=var)
            entry.pack(side=tk.LEFT, fill=tk.X, expand=True, padx=(0, 5))
            if is_dir:
                def browse():
                    d = filedialog.askdirectory(parent=manage_win)
                    if d: var.set(d)
                ttk.Button(row, text="Browse", command=browse, width=8).pack(side=tk.LEFT)
            return row

        make_row(right_frame, "Pattern (e.g. backup*):", m_pattern_var, is_dir=False)
        ttk.Separator(right_frame, orient='horizontal').pack(fill=tk.X, pady=10)
        
        make_row(right_frame, "Target Folder:", m_target_var)
        make_row(right_frame, "Processed Folder:", m_zip_var)
        make_row(right_frame, "Receipt Folder:", m_receipt_var)

        ttk.Separator(right_frame, orient='horizontal').pack(fill=tk.X, pady=10)

        combo_frame = ttk.Frame(right_frame, style="Card.TFrame")
        combo_frame.pack(fill=tk.X, pady=5)
        
        ttk.Label(combo_frame, text="Conflict Action:", width=20, style="Card.TLabel").pack(side=tk.LEFT)
        c_cb = ttk.Combobox(combo_frame, textvariable=m_conflict_var, values=["overwrite", "keep_both", "rename_existing"], state="readonly")
        c_cb.pack(side=tk.LEFT, fill=tk.X, expand=True, padx=(0, 10))
        
        ttk.Label(combo_frame, text="Post Action:", width=15, style="Card.TLabel").pack(side=tk.LEFT)
        p_cb = ttk.Combobox(combo_frame, textvariable=m_post_var, values=["leave", "delete", "move"], state="readonly")
        p_cb.pack(side=tk.LEFT, fill=tk.X, expand=True)

        chk_frame = ttk.Frame(right_frame, style="Card.TFrame")
        chk_frame.pack(fill=tk.X, pady=(15, 5))
        ttk.Checkbutton(chk_frame, text="Auto-Extract ZIP files", variable=m_auto_var, style="Card.TCheckbutton").pack(side=tk.LEFT)

        action_frame = ttk.Frame(right_frame, style="Card.TFrame")
        action_frame.pack(side=tk.BOTTOM, fill=tk.X, pady=(20, 0))

        patterns_data = self.core.load_patterns()
        
        def refresh_list():
            self.pattern_listbox.delete(0, tk.END)
            for p in sorted(patterns_data.keys()):
                self.pattern_listbox.insert(tk.END, p)

        def on_select(event):
            sel = self.pattern_listbox.curselection()
            if not sel: return
            p_name = self.pattern_listbox.get(sel[0])
            cfg = patterns_data.get(p_name, {})
            
            m_pattern_var.set(p_name)
            m_target_var.set(cfg.get('target_folder', ''))
            m_zip_var.set(cfg.get('target_zip_folder', ''))
            m_receipt_var.set(cfg.get('receipt_folder', ''))
            m_conflict_var.set(cfg.get('conflict_action', 'overwrite'))
            m_post_var.set(cfg.get('post_action', 'leave'))
            m_auto_var.set(cfg.get('auto_extract', True))

        self.pattern_listbox.bind('<<ListboxSelect>>', on_select)

        def save_pattern():
            p_name = m_pattern_var.get().strip()
            if not p_name:
                messagebox.showwarning("Warning", "Pattern cannot be empty.", parent=manage_win)
                return
            
            new_cfg = {
                "target_folder": m_target_var.get(),
                "target_zip_folder": m_zip_var.get(),
                "receipt_folder": m_receipt_var.get(),
                "conflict_action": m_conflict_var.get(),
                "post_action": m_post_var.get(),
                "auto_extract": m_auto_var.get()
            }
            patterns_data[p_name] = new_cfg
            self.core.save_pattern(p_name, new_cfg)
            refresh_list()
            messagebox.showinfo("Success", f"Saved configuration for pattern '{p_name}'.", parent=manage_win)

        def delete_pattern():
            p_name = m_pattern_var.get().strip()
            if not p_name or p_name not in patterns_data:
                return
            if messagebox.askyesno("Confirm Delete", f"Are you sure you want to delete the pattern '{p_name}'?", parent=manage_win):
                del patterns_data[p_name]
                self.core.delete_pattern(p_name)
                m_pattern_var.set("")
                m_target_var.set("")
                m_zip_var.set("")
                m_receipt_var.set("")
                refresh_list()
                
        def on_close():
            manage_win.destroy()
            self.update_display() # Refresh main UI to apply any pattern changes immediately

        ttk.Button(action_frame, text="Close", command=on_close).pack(side=tk.RIGHT, padx=(10, 0))
        ttk.Button(action_frame, text="Save Config", style="Accent.TButton", command=save_pattern).pack(side=tk.RIGHT)
        ttk.Button(action_frame, text="Delete", command=delete_pattern).pack(side=tk.LEFT)

        refresh_list()

    def save_permit_config(self):
        if self.current_index < 0: return
        
        permit_id = self.folders_data[self.current_index]['permitId']
        if not permit_id:
            messagebox.showwarning("Warning", "Cannot save configuration: Config ID is missing.")
            return
            
        config = {
            "target_folder": self.target_folder_var.get(),
            "target_zip_folder": self.target_zip_folder_var.get(),
            "receipt_folder": self.receipt_folder_var.get(),
            "conflict_action": self.conflict_action_var.get(),
            "post_action": self.post_action_var.get(),
            "auto_extract": self.auto_extract_var.get()
        }
        
        active_pattern = self.active_pattern_var.get().strip()
        
        try:
            if active_pattern:
                self.core.save_pattern(active_pattern, config)
                messagebox.showinfo("Success", f"Configuration saved for file pattern:\n'{active_pattern}'")
            else:
                self.core.save_config(permit_id, config)
                messagebox.showinfo("Success", f"Configuration saved for Config ID:\n{permit_id}")
                
            self.check_unsaved_changes()
        except Exception as e:
            messagebox.showerror("Error", f"Failed to save configuration:\n{e}")

    def process_current_zip(self):
        if self.current_index < 0: return
        
        current_data = self.folders_data[self.current_index]
        config = {
            "target_folder": self.target_folder_var.get(),
            "target_zip_folder": self.target_zip_folder_var.get(),
            "receipt_folder": self.receipt_folder_var.get(),
            "conflict_action": self.conflict_action_var.get(),
            "post_action": self.post_action_var.get(),
            "auto_extract": self.auto_extract_var.get()
        }

        if not config['target_folder']:
            messagebox.showerror("Error", "Please specify a Target Folder.")
            return
        if config['post_action'] == 'move' and not config['target_zip_folder']:
            messagebox.showerror("Error", "Please specify a Processed Folder when 'Move' is selected.")
            return

        self.btn_process.config(state=tk.DISABLED, text="Processing...")
        self.refresh_btn_text(self.btn_process)
        
        # Bridge the password prompt from the background thread safely into the Tkinter main loop
        def get_pwd_prompt(zip_filename):
            result = []
            event = threading.Event()
            
            def prompt_ui():
                pwd = simpledialog.askstring(
                    "Password Required", 
                    f"The file '{zip_filename}' is encrypted.\n\nPlease enter the password:", 
                    show='*', 
                    parent=self.root
                )
                result.append(pwd)
                event.set()
                
            # Schedule UI prompt into main thread
            self.root.after(0, prompt_ui)
            # Block the worker thread until UI finishes
            event.wait()
            return result[0]
        
        def worker():
            try:
                self.core.process_zip(current_data, config, password_callback=get_pwd_prompt)
                self.root.after(0, self.on_process_success)
            except Exception as e:
                self.root.after(0, lambda err=e: self.on_process_error(err))

        threading.Thread(target=worker, daemon=True).start()

    def on_process_success(self):
        messagebox.showinfo("Success", "Zip processed successfully.\n\nDetails have been written to the log.")
        self.btn_process.config(text="PROCESS FOLDER")
        self.refresh_btn_text(self.btn_process)
        
        folder1, folder2 = self.search_folder_1_var.get(), self.search_folder_2_var.get()
        folders_to_search = []
        if folder1 and os.path.isdir(folder1): folders_to_search.append(folder1)
        if folder2 and os.path.isdir(folder2): folders_to_search.append(folder2)
        
        if folders_to_search:
            self.folders_data = self.core.find_transfer_folders(folders_to_search)
            if self.folders_data:
                if self.current_index >= len(self.folders_data): self.current_index = max(0, len(self.folders_data) - 1)
            else: self.current_index = -1
            self.update_display()
        else:
            self.folders_data = []
            self.current_index = -1
            self.clear_zip_display()

    def on_process_error(self, err):
        messagebox.showerror("Processing Error", f"An error occurred.\nDetails have been written to the log.\n\nError: {err}")
        self.btn_process.config(state=tk.NORMAL, text="PROCESS FOLDER")
        self.refresh_btn_text(self.btn_process)
        self.update_nav_buttons()

    # --- Focus and Keybind Helpers ---
    def focus_btn(self, target_btn):
        if str(target_btn.cget('state')) == 'normal':
            target_btn.focus_set()
        return 'break'

    def invoke_btn(self, btn):
        if str(btn.cget('state')) == 'normal':
            btn.invoke()
        return 'break'

    def refresh_btn_text(self, btn):
        if not hasattr(self, 'btn_process') or not hasattr(self, 'btn_save_config'):
            return

        # Strip existing arrows to find the raw text
        current_text = btn.cget('text').replace('► ', '').replace(' ◄', '')

        if btn == self.btn_process:
            base_text = "Processing..." if "Processing" in current_text else "PROCESS FOLDER"
        elif btn == self.btn_save_config:
            base_text = "Save Config *" if "*" in current_text else "Save Config"
        else:
            return

        # Safely check focus (prevents KeyError when a messagebox pops up)
        has_focus = False
        try:
            has_focus = (self.root.focus_get() == btn)
        except (KeyError, tk.TclError):
            has_focus = False

        # Inject focus arrows if button is currently selected
        if has_focus and str(btn.cget('state')) == 'normal':
            btn.config(text=f"► {base_text} ◄")
        else:
            btn.config(text=base_text)


# --------------------------------------------------------------------------- #
# CLI APPLICATION (Untouched)
# --------------------------------------------------------------------------- #

def run_cli():
    parser = argparse.ArgumentParser(description=f"Inbox Mover v{VERSION}")
    parser.add_argument('--cli', action='store_true', help=argparse.SUPPRESS) 
    parser.add_argument('-s', '--search-folders', nargs='+', required=True, help='One or more folders to search for transfer folders')
    parser.add_argument('-t', '--target-folder', required=True, help='Default target folder for extraction')
    parser.add_argument('-z', '--target-zip-folder', help='Processed Folder for moving processed zips')
    parser.add_argument('-r', '--receipt-folder', help='Target folder for the receipt.json file')
    parser.add_argument('-c', '--conflict-action', choices=['overwrite', 'keep_both', 'rename_existing'], default='overwrite', help='Action when extracted file already exists')
    parser.add_argument('-p', '--post-action', choices=['leave', 'delete', 'move'], default='leave', help='Action to perform on zip after extraction')
    parser.add_argument('--no-auto-extract', action='store_false', dest='auto_extract', help='Disable automatic zip extraction and instead copy the raw zip file.')
    
    args = parser.parse_args()

    core = InboxMoverCore()
    folders = core.find_transfer_folders(args.search_folders)
    
    if not folders:
        print(f"No transfer folders found in the specified search folders.")
        return

    print(f"Found {len(folders)} transfer folders to inspect.")
    
    def get_pwd_cli(zip_filename):
        return getpass.getpass(f"\nPassword required for '{zip_filename}': ")
    
    for data in folders:
        print(f"\nProcessing Folder: {data['folder_name']} (Config ID: {data['permitId']})")
        
        if not data.get('can_process'):
            print("  Folder is empty. Skipping.")
            continue
        
        config = {
            "target_folder": args.target_folder,
            "target_zip_folder": args.target_zip_folder,
            "receipt_folder": args.receipt_folder,
            "conflict_action": args.conflict_action,
            "post_action": args.post_action,
            "auto_extract": args.auto_extract
        }
        
        default_config = core.load_config("DEFAULT")
        if default_config:
            for k, v in default_config.items():
                if v: config[k] = v
            print("  Loaded DEFAULT configuration.")
        else:
            print("  Using CLI arguments for baseline configuration.")

        if data['permitId'] != "DEFAULT":
            specific_config = core.load_config(data['permitId'])
            if specific_config:
                for k, v in specific_config.items():
                    if v: config[k] = v
                print(f"  Loaded specific configuration for Config ID: {data['permitId']}.")

        receipt = data.get('receipt') or {}
        if receipt.get('target_folder'): config['target_folder'] = receipt.get('target_folder')
        if receipt.get('process_folder'): config['target_zip_folder'] = receipt.get('process_folder')
        if receipt.get('receipt_folder'): config['receipt_folder'] = receipt.get('receipt_folder')
        if receipt.get('conflict_resolution'): config['conflict_action'] = receipt.get('conflict_resolution')
        if receipt.get('post_processing'): config['post_action'] = receipt.get('post_processing')
        if 'auto_extract' in receipt: config['auto_extract'] = receipt.get('auto_extract')

        if config.get('post_action') == 'move' and not config.get('target_zip_folder'):
            error_msg = "Post action is 'move' but no Processed Folder specified. Skipping."
            print(f"  Error: {error_msg}")
            core.write_log("ERROR", data, config, [], error_msg)
            continue
            
        try:
            core.process_zip(data, config, password_callback=get_pwd_cli)
            print("  Successfully processed. Actions written to log.")
        except Exception as e:
            print(f"  Error processing zip: {e}")

# --------------------------------------------------------------------------- #
# MAIN ENTRY POINT (Untouched)
# --------------------------------------------------------------------------- #

def main():
    if len(sys.argv) > 1:
        run_cli()
    else:
        try:
            root = tk.Tk()
            app = InboxMoverGUI(root)
            
            root.lift()
            root.attributes('-topmost', True)
            root.after_idle(root.attributes, '-topmost', False)
            
            root.mainloop()
        except tk.TclError as e:
            print(f"GUI Initialization Error: {e}")
            print("\nIt appears you are running Inbox Mover on a headless server (e.g., Azure Linux VM) without a display.")
            print("To process folders automatically from the command line, please provide the required arguments.")
            print("Example:")
            print("  python3 inbox_mover.py -s /path/to/search -t /path/to/target\n")
            print("For a full list of commands, run: python3 inbox_mover.py --help")
            sys.exit(1)

if __name__ == '__main__':
    main()