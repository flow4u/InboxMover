#!/usr/bin/env python3
"""
Inbox Mover v0.6
the perfect FileButler companion
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

# Tkinter imports
import tkinter as tk
from tkinter import ttk, filedialog, messagebox

VERSION = "0.6"
CONFIG_DIR = "permit_configs"

# --------------------------------------------------------------------------- #
# CORE LOGIC
# --------------------------------------------------------------------------- #

class InboxMoverCore:
    def __init__(self):
        self.ensure_config_dir()

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
        return {"dark_mode": True, "font_size": 10, "window_geometry": "800x650", "search_folder_1": "", "search_folder_2": ""}

    def save_app_settings(self, settings):
        settings_path = os.path.join(CONFIG_DIR, "app_settings.json")
        with open(settings_path, 'w') as f:
            json.dump(settings, f, indent=4)

    def find_transfer_folders(self, search_folders):
        """Find direct subdirectories starting with 'transfer-' and inspect them."""
        folders_data = []
        seen_paths = set()
        
        # Allow passing a single string instead of a list for convenience
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
        
        # Sort to ensure consistent navigation order in descending order
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
            "can_process": False
        }
        
        valid_zip_found = False
        for root, _, files in os.walk(folder_path):
            for file in files:
                if file.lower().endswith('.zip'):
                    zip_path = os.path.join(root, file)
                    zip_info = self.inspect_zip(zip_path)
                    # Only treat as a valid zip if it actually contains the receipt.json
                    if zip_info and zip_info.get("receipt_raw"):
                        data["zip_path"] = zip_path
                        data["permitId"] = zip_info["permitId"]
                        data["receipt"] = zip_info["receipt"]
                        data["receipt_raw"] = zip_info["receipt_raw"]
                        data["has_valid_zip"] = True
                        data["can_process"] = True
                        valid_zip_found = True
                        break
            if valid_zip_found:
                break
                
        if not valid_zip_found:
            file_list = []
            for root, _, files in os.walk(folder_path):
                for f in sorted(files):
                    rel_file = os.path.relpath(os.path.join(root, f), folder_path)
                    file_list.append(rel_file)
            
            if not file_list:
                data["receipt_raw"] = "<Folder is empty>"
                data["can_process"] = False
            else:
                data["receipt_raw"] = "NO RECEIPT.JSON FOUND.\nWILL PROCESS ALL FILES IN FOLDER:\n\n" + "\n".join(file_list)
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
                # Look for receipt.json anywhere in the zip structure
                receipt_filename = next((f for f in zf.namelist() if f.endswith('receipt.json')), None)
                if receipt_filename:
                    with zf.open(receipt_filename) as f:
                        content = f.read().decode('utf-8')
                        data["receipt_raw"] = content
                        try:
                            receipt_json = json.loads(content)
                            data["receipt"] = receipt_json
                            data["permitId"] = receipt_json.get("permitId", "DEFAULT")
                        except json.JSONDecodeError:
                            pass
                return data
        except zipfile.BadZipFile:
            return None # Skip invalid zips
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

    def save_config(self, permit_id, config_data):
        """Save configuration for a specific Config ID."""
        if not permit_id:
            raise ValueError("Cannot save configuration without a Config ID.")
        config_path = os.path.join(CONFIG_DIR, f"{permit_id}.json")
        with open(config_path, 'w') as f:
            json.dump(config_data, f, indent=4)

    def process_zip(self, folder_data, config, progress_callback=None):
        """
        Extract the zip and apply conflict resolution and post-processing.
        config requires: target_folder, conflict_action, post_action, target_zip_folder
        """
        target_folder = config.get('target_folder')
        conflict_action = config.get('conflict_action', 'overwrite')
        post_action = config.get('post_action', 'leave')
        target_zip_folder = config.get('target_zip_folder')
        receipt_folder = config.get('receipt_folder')

        if not target_folder or not os.path.isdir(target_folder):
            raise ValueError(f"Target folder '{target_folder}' is invalid.")

        def get_final_path(extracted_path):
            if os.path.exists(extracted_path):
                if conflict_action == 'overwrite':
                    pass
                elif conflict_action == 'keep_both':
                    base, ext = os.path.splitext(extracted_path)
                    counter = 1
                    while os.path.exists(f"{base} ({counter}){ext}"):
                        counter += 1
                    extracted_path = f"{base} ({counter}){ext}"
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
            return extracted_path

        def extract_zip_file(zip_path):
            with zipfile.ZipFile(zip_path, 'r') as zf:
                file_list = [f for f in zf.infolist() if not f.is_dir()]
                total = len(file_list)
                for i, zinfo in enumerate(file_list):
                    original_name = zinfo.filename
                    
                    # Determine if the path inside the zip is absolute
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
                    final_path = get_final_path(ext_path)

                    with zf.open(zinfo) as source, open(final_path, "wb") as target:
                        shutil.copyfileobj(source, target)
                        
                    if progress_callback:
                        progress_callback(i + 1, total)

        if folder_data.get('has_valid_zip') and folder_data.get('zip_path'):
            extract_zip_file(folder_data['zip_path'])
        else:
            folder_path = folder_data.get('folder_path')
            for root, _, files in os.walk(folder_path):
                for file in files:
                    src_path = os.path.join(root, file)
                    if file.lower().endswith('.zip'):
                        extract_zip_file(src_path)
                    else:
                        rel_path = os.path.relpath(src_path, folder_path)
                        ext_path = os.path.join(target_folder, rel_path)
                        os.makedirs(os.path.dirname(ext_path), exist_ok=True)
                        final_path = get_final_path(ext_path)
                        shutil.copy2(src_path, final_path)

        # Post Processing
        if post_action == 'delete':
            folder_path = folder_data.get('folder_path')
            if folder_path and os.path.isdir(folder_path):
                shutil.rmtree(folder_path)
        elif post_action == 'move':
            if not target_zip_folder or not os.path.isdir(target_zip_folder):
                raise ValueError(f"Processed Folder '{target_zip_folder}' is invalid.")
            
            folder_path = folder_data.get('folder_path')
            if not folder_path or not os.path.isdir(folder_path):
                raise ValueError(f"Source folder '{folder_path}' is invalid or missing.")
                
            folder_name = folder_data.get('folder_name')
            dest_path = os.path.join(target_zip_folder, folder_name)
            
            # handle if folder already exists in target zip folder
            if os.path.exists(dest_path):
                counter = 1
                while os.path.exists(f"{dest_path}_{counter}"):
                    counter += 1
                dest_path = f"{dest_path}_{counter}"
                
            shutil.move(folder_path, dest_path)
        # 'leave' does nothing


# --------------------------------------------------------------------------- #
# GUI APPLICATION
# --------------------------------------------------------------------------- #

class InboxMoverGUI:
    def __init__(self, root):
        self.root = root
        self.root.title(f"Inbox Mover v{VERSION} - the perfect FileButler companion")
        self.root.minsize(700, 600)
        self.root.resizable(True, True)
        
        self.core = InboxMoverCore()
        
        # Load app settings
        settings = self.core.load_app_settings()
        self.is_dark_mode = settings.get("dark_mode", True)
        self.base_font_size = settings.get("font_size", 10)
        
        # Apply saved geometry or default
        window_geometry = settings.get("window_geometry", "800x650")
        self.root.geometry(window_geometry)
        
        # Intercept window close to save settings (including geometry)
        self.root.protocol("WM_DELETE_WINDOW", self.on_closing)
        
        self.folders_data = []
        self.current_index = -1

        # Variables
        self.search_folder_1_var = tk.StringVar(value=settings.get("search_folder_1", settings.get("search_folder", "")))
        self.search_folder_2_var = tk.StringVar(value=settings.get("search_folder_2", ""))
        self.target_folder_var = tk.StringVar()
        self.target_zip_folder_var = tk.StringVar()
        self.receipt_folder_var = tk.StringVar()
        
        self.conflict_action_var = tk.StringVar(value="overwrite")
        self.post_action_var = tk.StringVar(value="leave")
        
        self.inbox_name_var = tk.StringVar(value="")
        self.zip_name_var = tk.StringVar(value="No Transfer Folders Found")
        self.permit_id_var = tk.StringVar(value="")

        # Trace variables to detect unsaved changes dynamically
        self.target_folder_var.trace_add("write", self.check_unsaved_changes)
        self.target_zip_folder_var.trace_add("write", self.check_unsaved_changes)
        self.receipt_folder_var.trace_add("write", self.check_unsaved_changes)
        self.conflict_action_var.trace_add("write", self.check_unsaved_changes)
        self.post_action_var.trace_add("write", self.check_unsaved_changes)

        self.setup_ui()
        self.apply_theme()
        self.apply_fonts()
        self.bind_keys()
        
        # Auto-load if search folders were saved
        if self.search_folder_1_var.get() or self.search_folder_2_var.get():
            self.on_search_folder_changed(startup=True)

    def setup_ui(self):
        # Configure main padding
        main_frame = ttk.Frame(self.root, padding="15")
        main_frame.pack(fill=tk.BOTH, expand=True)

        # --- Header ---
        header_frame = ttk.Frame(main_frame)
        header_frame.pack(fill=tk.X, pady=(0, 10))
        
        title_frame = ttk.Frame(header_frame)
        title_frame.pack(side=tk.LEFT)
        
        self.lbl_title = ttk.Label(title_frame, text="Inbox Mover")
        self.lbl_title.pack(anchor=tk.W)
        self.lbl_version = ttk.Label(title_frame, text=f"the perfect FileButler companion - version {VERSION}")
        self.lbl_version.pack(anchor=tk.W)
        
        self.theme_btn = ttk.Button(header_frame, text="Toggle Light Mode", command=self.toggle_theme)
        self.theme_btn.pack(side=tk.RIGHT)
        self.btn_help = ttk.Button(header_frame, text="? Help", width=6, command=self.show_help)
        self.btn_help.pack(side=tk.RIGHT, padx=5)
        self.btn_increase_font = ttk.Button(header_frame, text="A+", width=3, command=self.increase_font)
        self.btn_increase_font.pack(side=tk.RIGHT, padx=2)
        self.btn_decrease_font = ttk.Button(header_frame, text="A-", width=3, command=self.decrease_font)
        self.btn_decrease_font.pack(side=tk.RIGHT, padx=2)

        # --- Folders Section ---
        folder_frame = ttk.LabelFrame(main_frame, text="Directories", padding="10")
        folder_frame.pack(fill=tk.X, pady=5)
        
        self.create_folder_row(folder_frame, "Search Folder 1:", self.search_folder_1_var, 0, self.on_search_folder_changed)
        self.create_folder_row(folder_frame, "Search Folder 2:", self.search_folder_2_var, 1, self.on_search_folder_changed)
        self.create_folder_row(folder_frame, "Target Folder:", self.target_folder_var, 2)
        self.create_folder_row(folder_frame, "Processed Folder:", self.target_zip_folder_var, 3)
        self.create_folder_row(folder_frame, "Receipt Folder:", self.receipt_folder_var, 4)

        # --- Navigation & Process Section ---
        nav_frame = ttk.Frame(main_frame)
        nav_frame.pack(fill=tk.X, pady=15)
        
        # Nav Buttons
        nav_btns = ttk.Frame(nav_frame)
        nav_btns.pack(side=tk.LEFT)
        
        self.btn_refresh = ttk.Button(nav_btns, text="↻ Refresh", width=10, command=self.on_search_folder_changed)
        self.btn_refresh.pack(side=tk.LEFT, padx=2)
        
        self.btn_prev = ttk.Button(nav_btns, text="⇦ Prev", width=8, command=self.prev_zip)
        self.btn_prev.pack(side=tk.LEFT, padx=2)
        self.btn_process = ttk.Button(nav_btns, text="PROCESS", width=12, command=self.process_current_zip)
        self.btn_process.pack(side=tk.LEFT, padx=2)
        self.btn_next = ttk.Button(nav_btns, text="Next ⇨", width=8, command=self.next_zip)
        self.btn_next.pack(side=tk.LEFT, padx=2)

        # Info Labels
        info_frame = ttk.Frame(nav_frame)
        info_frame.pack(side=tk.LEFT, padx=30, expand=True, fill=tk.X)
        self.lbl_inbox_name = ttk.Label(info_frame, textvariable=self.inbox_name_var)
        self.lbl_inbox_name.pack(anchor=tk.W)
        self.lbl_zip_name = ttk.Label(info_frame, textvariable=self.zip_name_var)
        self.lbl_zip_name.pack(anchor=tk.W)
        self.lbl_permit_id = ttk.Label(info_frame, textvariable=self.permit_id_var)
        self.lbl_permit_id.pack(anchor=tk.W)

        # Config Save
        self.btn_save_config = ttk.Button(nav_frame, text="Save Config", command=self.save_permit_config)
        self.btn_save_config.pack(side=tk.RIGHT)

        # --- Options Section ---
        options_frame = ttk.Frame(main_frame)
        options_frame.pack(fill=tk.X, pady=5)
        
        # Conflict Options
        conflict_frame = ttk.LabelFrame(options_frame, text="If the file already exists in the target folder then:", padding="10")
        conflict_frame.pack(side=tk.LEFT, fill=tk.BOTH, expand=True, padx=(0, 5))
        ttk.Radiobutton(conflict_frame, text="Overwrite existing file", variable=self.conflict_action_var, value="overwrite").pack(anchor=tk.W)
        ttk.Radiobutton(conflict_frame, text="Keep both (add number)", variable=self.conflict_action_var, value="keep_both").pack(anchor=tk.W)
        ttk.Radiobutton(conflict_frame, text="Rename existing file", variable=self.conflict_action_var, value="rename_existing").pack(anchor=tk.W)

        # Post Action Options
        post_frame = ttk.LabelFrame(options_frame, text="After processing:", padding="10")
        post_frame.pack(side=tk.RIGHT, fill=tk.BOTH, expand=True, padx=(5, 0))
        ttk.Radiobutton(post_frame, text="Leave the files in places", variable=self.post_action_var, value="leave").pack(anchor=tk.W)
        ttk.Radiobutton(post_frame, text="Delete the files", variable=self.post_action_var, value="delete").pack(anchor=tk.W)
        ttk.Radiobutton(post_frame, text="Move the files to Processed Folder", variable=self.post_action_var, value="move").pack(anchor=tk.W)

        # --- Source Folder Content Text Area ---
        text_frame = ttk.LabelFrame(main_frame, text="Source Folder Content", padding="10")
        text_frame.pack(fill=tk.BOTH, expand=True, pady=10)
        
        self.receipt_text = tk.Text(text_frame, wrap=tk.WORD, state=tk.DISABLED)
        scrollbar = ttk.Scrollbar(text_frame, orient=tk.VERTICAL, command=self.receipt_text.yview)
        self.receipt_text.configure(yscrollcommand=scrollbar.set)
        
        self.receipt_text.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
        scrollbar.pack(side=tk.RIGHT, fill=tk.Y)

        self.update_nav_buttons()

    def create_folder_row(self, parent, label_text, str_var, row, callback=None):
        ttk.Label(parent, text=label_text).grid(row=row, column=0, sticky=tk.W, pady=3)
        entry = ttk.Entry(parent, textvariable=str_var, width=50)
        entry.grid(row=row, column=1, sticky=tk.EW, padx=5, pady=3)
        parent.columnconfigure(1, weight=1)
        
        btn_frame = ttk.Frame(parent)
        btn_frame.grid(row=row, column=2, sticky=tk.E, pady=3)
        
        def browse():
            folder = filedialog.askdirectory()
            if folder:
                str_var.set(folder)
                if callback:
                    callback()
                    
        def open_dir():
            path = str_var.get()
            if not path or not os.path.isdir(path):
                messagebox.showwarning("Warning", "The specified folder does not exist.")
                return
            if sys.platform == "win32":
                os.startfile(path)
            elif sys.platform == "darwin":
                subprocess.call(["open", path])
            else:
                subprocess.call(["xdg-open", path])

        ttk.Button(btn_frame, text="Browse...", command=browse).pack(side=tk.LEFT, padx=(0, 2))
        ttk.Button(btn_frame, text="Open", width=6, command=open_dir).pack(side=tk.LEFT)
        
        if callback:
            entry.bind('<FocusOut>', lambda e: callback())
            entry.bind('<Return>', lambda e: callback())

    def apply_fonts(self):
        style = ttk.Style()
        style.configure(".", font=("Helvetica", self.base_font_size))
        style.configure("TButton", font=("Helvetica", self.base_font_size))
        style.configure("Accent.TButton", font=("Helvetica", self.base_font_size, "bold"))
        style.configure("TLabelframe.Label", font=("Helvetica", self.base_font_size, "bold"))
        
        self.lbl_title.config(font=("Helvetica", self.base_font_size + 14, "bold"))
        self.lbl_version.config(font=("Helvetica", self.base_font_size, "italic"))
        self.lbl_inbox_name.config(font=("Helvetica", self.base_font_size, "italic"))
        self.lbl_zip_name.config(font=("Helvetica", self.base_font_size + 2, "bold"))
        self.lbl_permit_id.config(font=("Helvetica", self.base_font_size))
        
        self.receipt_text.configure(font=("Courier", self.base_font_size))

    def increase_font(self):
        if self.base_font_size < 30:
            self.base_font_size += 1
            self.apply_fonts()
            self.save_settings()

    def decrease_font(self):
        if self.base_font_size > 6:
            self.base_font_size -= 1
            self.apply_fonts()
            self.save_settings()

    def apply_theme(self):
        style = ttk.Style(self.root)
        style.theme_use('clam') # Clean base theme
        
        if self.is_dark_mode:
            self.root.configure(bg="#1e1e1e")
            bg_color = "#1e1e1e"
            fg_color = "#ffffff"
            frame_bg = "#2d2d2d"
            btn_bg = "#3a3a3a"
            btn_fg = "#ffffff"
            btn_active = "#505050"
            entry_bg = "#333333"
            entry_fg = "#ffffff"
            text_bg = "#1e1e1e"
            
            accent_bg = "#d97706" # Material Amber 600
            accent_fg = "#ffffff"
            accent_active = "#f59e0b" # Material Amber 500
            
            self.theme_btn.config(text="☀ Light Mode")
        else:
            self.root.configure(bg="#f0f0f0")
            bg_color = "#f0f0f0"
            fg_color = "#000000"
            frame_bg = "#ffffff"
            btn_bg = "#e0e0e0"
            btn_fg = "#000000"
            btn_active = "#d0d0d0"
            entry_bg = "#ffffff"
            entry_fg = "#000000"
            text_bg = "#ffffff"
            
            accent_bg = "#f59e0b" # Material Amber 500
            accent_fg = "#ffffff"
            accent_active = "#d97706" # Material Amber 600
            
            self.theme_btn.config(text="☾ Dark Mode")

        # Configure TTK styles
        style.configure(".", background=bg_color, foreground=fg_color, fieldbackground=entry_bg)
        style.configure("TFrame", background=bg_color)
        style.configure("TLabelframe", background=bg_color, foreground=fg_color)
        style.configure("TLabelframe.Label", background=bg_color, foreground=fg_color)
        style.configure("TButton", background=btn_bg, foreground=btn_fg, padding=5, borderwidth=0)
        style.map("TButton", background=[('active', btn_active)])
        
        style.configure("Accent.TButton", background=accent_bg, foreground=accent_fg, padding=5, borderwidth=0)
        style.map("Accent.TButton", background=[('active', accent_active)])
        
        style.configure("TRadiobutton", background=bg_color, foreground=fg_color)
        style.configure("TEntry", fieldbackground=entry_bg, foreground=entry_fg)
        
        # Standard TK widgets
        self.receipt_text.configure(bg=text_bg, fg=fg_color, insertbackground=fg_color)

    def toggle_theme(self):
        self.is_dark_mode = not self.is_dark_mode
        self.apply_theme()
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

    def show_help(self):
        help_win = tk.Toplevel(self.root)
        help_win.title("Inbox Mover - Help")
        help_win.geometry("650x500")
        help_win.minsize(500, 400)
        
        # Match root background
        help_win.configure(bg=self.root.cget("bg"))
        
        frame = ttk.Frame(help_win, padding="15")
        frame.pack(fill=tk.BOTH, expand=True)
        
        help_text = tk.Text(frame, wrap=tk.WORD, font=("Helvetica", self.base_font_size))
        scrollbar = ttk.Scrollbar(frame, orient=tk.VERTICAL, command=help_text.yview)
        help_text.configure(yscrollcommand=scrollbar.set)
        
        if self.is_dark_mode:
            help_text.configure(bg="#1e1e1e", fg="#ffffff", insertbackground="#ffffff")
        else:
            help_text.configure(bg="#ffffff", fg="#000000", insertbackground="#000000")
            
        help_text.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
        scrollbar.pack(side=tk.RIGHT, fill=tk.Y)
        
        instructions = """INBOX MOVER - USER INSTRUCTIONS

OVERVIEW
Inbox Mover processes ZIP files (typically containing a receipt.json) by extracting them into a designated target folder while resolving file conflicts automatically.

1. DIRECTORIES
• Search Folder 1 & 2: The root directories where the app looks for child folders starting with "transfer-". You can specify up to two search locations.
• Target Folder: The directory where the contents of the ZIP will be extracted.
• Processed Folder: (Optional) The directory where the original ZIP file is moved if the "Move the files to Processed Folder" post-action is selected.
• Receipt Folder: (Optional) A dedicated folder where receipt.json will be extracted (prepended with a timestamp).

2. NAVIGATION
• Use the '⇦ Prev' and 'Next ⇨' buttons (or your keyboard's Left/Right arrow keys) to cycle through the found transfer folders.
• Click '↻ Refresh' to rescan the Search Folder for new or modified transfer folders.

3. CONFLICT RESOLUTION (If file already exists in Target Folder)
• Overwrite: Replaces the existing file with the new one from the ZIP.
• Keep both: Extracts the new file and adds a number to its filename (e.g., file (1).txt).
• Rename existing: Renames the file already on your disk by prepending a timestamp (YYMMDD-HHMMSS_filename), then extracts the new file normally.

4. POST PROCESSING
• Leave: Leaves the files in places.
• Delete: Permanently deletes the entire transfer folder and all its contents after successful extraction.
• Move: Moves the entire transfer folder and all its contents to the Processed Folder.

5. CONFIGURATIONS & CONFIG IDs
• The app reads 'receipt.json' inside the ZIP to find a 'Config ID' (previously Permit ID).
• If no receipt.json is found, or it lacks an ID, a "DEFAULT" Config ID is assigned.
• If you set up your folders and rules for a specific Config ID, click 'Save Config'.
• The next time you encounter a ZIP with that exact Config ID, the application will automatically load your saved folder paths and conflict/post-action settings.

6. ADVANCED FEATURES (OVERRIDES & PATHS)
• Absolute Paths: If a file inside the ZIP is mapped to an absolute path (e.g., C:\\logs\\file.txt), it ignores the Target Folder and extracts directly to that path, creating folders as needed.
• Receipt Overrides: If receipt.json contains keys like 'target_folder', 'process_folder', 'receipt_folder', 'conflict_resolution', or 'post_processing', these values will automatically override your saved GUI settings. The 'Save Config' button will turn orange to indicate unsaved changes forced by the receipt.

CLI MODE
You can also run this application via the command line for automation. Run `python inbox_mover.py --cli --help` in your terminal for details.
"""
        help_text.insert(tk.END, instructions)
        help_text.config(state=tk.DISABLED)

    def bind_keys(self):
        self.root.bind("<Left>", lambda e: self.prev_zip())
        self.root.bind("<Right>", lambda e: self.next_zip())

    def on_search_folder_changed(self, startup=False):
        folder1 = self.search_folder_1_var.get()
        folder2 = self.search_folder_2_var.get()
        
        folders_to_search = []
        if folder1 and os.path.isdir(folder1):
            folders_to_search.append(folder1)
        elif folder1 and not startup:
            messagebox.showwarning("Warning", f"Search Folder 1 does not exist:\n{folder1}")
            
        if folder2 and os.path.isdir(folder2):
            folders_to_search.append(folder2)
        elif folder2 and not startup:
            messagebox.showwarning("Warning", f"Search Folder 2 does not exist:\n{folder2}")

        if folders_to_search:
            self.folders_data = self.core.find_transfer_folders(folders_to_search)
            if self.folders_data:
                self.current_index = 0
            else:
                self.current_index = -1
                self.clear_zip_display()
                if not startup:
                    messagebox.showinfo("Info", "No transfer folders found in the specified search folders.")
            self.update_display()
        else:
            self.folders_data = []
            self.current_index = -1
            self.clear_zip_display()

    def clear_zip_display(self):
        self.inbox_name_var.set("")
        self.zip_name_var.set("No Transfer Folders Found")
        self.permit_id_var.set("")
        self.set_receipt_text("")
        self.update_nav_buttons()

    def update_display(self):
        if not self.folders_data or self.current_index < 0 or self.current_index >= len(self.folders_data):
            self.clear_zip_display()
            return

        current_data = self.folders_data[self.current_index]
        parent_dir = os.path.dirname(current_data['folder_path'])
        self.inbox_name_var.set(f"Inbox: {parent_dir}")
        self.zip_name_var.set(f"[{self.current_index + 1}/{len(self.folders_data)}] {current_data['folder_name']}")
        self.permit_id_var.set(f"Config ID: {current_data['permitId']}")
        self.set_receipt_text(current_data['receipt_raw'])
        
        # Attempt to load config for this permitId
        config = self.core.load_config(current_data['permitId'])
        if config:
            if 'target_folder' in config: self.target_folder_var.set(config['target_folder'])
            if 'target_zip_folder' in config: self.target_zip_folder_var.set(config['target_zip_folder'])
            if 'receipt_folder' in config: self.receipt_folder_var.set(config['receipt_folder'])
            if 'conflict_action' in config: self.conflict_action_var.set(config['conflict_action'])
            if 'post_action' in config: self.post_action_var.set(config['post_action'])
            
        # Apply overrides from receipt.json if present and not empty
        receipt = current_data.get('receipt') or {}
        if receipt.get('target_folder'): self.target_folder_var.set(receipt.get('target_folder'))
        if receipt.get('process_folder'): self.target_zip_folder_var.set(receipt.get('process_folder'))
        if receipt.get('receipt_folder'): self.receipt_folder_var.set(receipt.get('receipt_folder'))
        if receipt.get('conflict_resolution'): self.conflict_action_var.set(receipt.get('conflict_resolution'))
        if receipt.get('post_processing'): self.post_action_var.set(receipt.get('post_processing'))
            
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
        self.btn_process.config(state=tk.NORMAL if can_process else tk.DISABLED)
        self.btn_save_config.config(state=tk.NORMAL if can_process else tk.DISABLED)

    def check_unsaved_changes(self, *args):
        # Protect against trace firing during UI setup before button exists
        if not hasattr(self, 'btn_save_config'):
            return 
            
        if self.current_index < 0 or not self.folders_data:
            self.btn_save_config.config(style="TButton", text="Save Config")
            return
            
        current_data = self.folders_data[self.current_index]
        permit_id = current_data.get('permitId')
        can_process = current_data.get('can_process', False)
        
        if not permit_id or not can_process:
            self.btn_save_config.config(style="TButton", text="Save Config")
            return
            
        saved_config = self.core.load_config(permit_id)
        
        current_config = {
            "target_folder": self.target_folder_var.get(),
            "target_zip_folder": self.target_zip_folder_var.get(),
            "receipt_folder": self.receipt_folder_var.get(),
            "conflict_action": self.conflict_action_var.get(),
            "post_action": self.post_action_var.get()
        }

        is_unsaved = False
        if saved_config is None:
            # If there's no saved config, it's definitively unsaved
            is_unsaved = True
        else:
            # Compare exact match
            if current_config != saved_config:
                is_unsaved = True
                
        if is_unsaved:
            self.btn_save_config.config(style="Accent.TButton", text="Save Config *")
        else:
            self.btn_save_config.config(style="TButton", text="Save Config")

    def prev_zip(self):
        if self.current_index > 0:
            self.current_index -= 1
            self.update_display()

    def next_zip(self):
        if self.current_index < len(self.folders_data) - 1:
            self.current_index += 1
            self.update_display()

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
            "post_action": self.post_action_var.get()
        }
        
        try:
            self.core.save_config(permit_id, config)
            messagebox.showinfo("Success", f"Configuration saved for Config ID: {permit_id}")
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
            "post_action": self.post_action_var.get()
        }

        # Validation
        if not config['target_folder']:
            messagebox.showerror("Error", "Please specify a Target Folder.")
            return
        if config['post_action'] == 'move' and not config['target_zip_folder']:
            messagebox.showerror("Error", "Please specify a Processed Folder when 'Move' is selected.")
            return

        self.btn_process.config(state=tk.DISABLED, text="Processing...")
        
        # Run in thread to prevent UI freezing
        def worker():
            try:
                self.core.process_zip(current_data, config)
                self.root.after(0, self.on_process_success)
            except Exception as e:
                self.root.after(0, lambda err=e: self.on_process_error(err))

        threading.Thread(target=worker, daemon=True).start()

    def on_process_success(self):
        messagebox.showinfo("Success", "Zip processed successfully.")
        self.btn_process.config(text="PROCESS")
        
        # Refresh the list quietly as the folder might have been moved or deleted
        folder1 = self.search_folder_1_var.get()
        folder2 = self.search_folder_2_var.get()
        folders_to_search = []
        if folder1 and os.path.isdir(folder1): folders_to_search.append(folder1)
        if folder2 and os.path.isdir(folder2): folders_to_search.append(folder2)
        
        if folders_to_search:
            self.folders_data = self.core.find_transfer_folders(folders_to_search)
            if self.folders_data:
                # Ensure the index stays within bounds if files were removed
                if self.current_index >= len(self.folders_data):
                    self.current_index = max(0, len(self.folders_data) - 1)
            else:
                self.current_index = -1
            self.update_display()
        else:
            self.folders_data = []
            self.current_index = -1
            self.clear_zip_display()

    def on_process_error(self, err):
        messagebox.showerror("Processing Error", str(err))
        self.btn_process.config(state=tk.NORMAL, text="PROCESS")
        self.update_nav_buttons()


# --------------------------------------------------------------------------- #
# CLI APPLICATION
# --------------------------------------------------------------------------- #

def run_cli():
    parser = argparse.ArgumentParser(description=f"Inbox Mover v{VERSION} - the perfect FileButler companion CLI")
    parser.add_argument('--cli', action='store_true', help=argparse.SUPPRESS) # Hide the switch that triggered this
    parser.add_argument('-s', '--search-folders', nargs='+', required=True, help='One or more folders to search for transfer folders')
    parser.add_argument('-t', '--target-folder', required=True, help='Default target folder for extraction')
    parser.add_argument('-z', '--target-zip-folder', help='Processed Folder for moving processed zips')
    parser.add_argument('-r', '--receipt-folder', help='Target folder for the receipt.json file')
    parser.add_argument('-c', '--conflict-action', choices=['overwrite', 'keep_both', 'rename_existing'], default='overwrite', help='Action when extracted file already exists')
    parser.add_argument('-p', '--post-action', choices=['leave', 'delete', 'move'], default='leave', help='Action to perform on zip after extraction')
    
    args = parser.parse_args()

    core = InboxMoverCore()
    folders = core.find_transfer_folders(args.search_folders)
    
    if not folders:
        print(f"No transfer folders found in the specified search folders.")
        return

    print(f"Found {len(folders)} transfer folders to inspect.")
    
    for data in folders:
        print(f"\nProcessing Folder: {data['folder_name']} (Config ID: {data['permitId']})")
        
        if not data.get('can_process'):
            print("  Folder is empty. Skipping.")
            continue
        
        # Load specific config if exists, otherwise fallback to CLI args
        config = core.load_config(data['permitId'])
        if not config:
            config = {
                "target_folder": args.target_folder,
                "target_zip_folder": args.target_zip_folder,
                "receipt_folder": args.receipt_folder,
                "conflict_action": args.conflict_action,
                "post_action": args.post_action
            }
            print("  Using CLI arguments for configuration.")
        else:
            print("  Loaded saved configuration for this Config ID.")

        # Apply overrides from receipt.json if present and not empty
        receipt = data.get('receipt') or {}
        if receipt.get('target_folder'): config['target_folder'] = receipt.get('target_folder')
        if receipt.get('process_folder'): config['target_zip_folder'] = receipt.get('process_folder')
        if receipt.get('receipt_folder'): config['receipt_folder'] = receipt.get('receipt_folder')
        if receipt.get('conflict_resolution'): config['conflict_action'] = receipt.get('conflict_resolution')
        if receipt.get('post_processing'): config['post_action'] = receipt.get('post_processing')

        # Validate post-action move requirement
        if config.get('post_action') == 'move' and not config.get('target_zip_folder'):
            print("  Error: Post action is 'move' but no Processed Folder specified. Skipping.")
            continue
            
        try:
            core.process_zip(data, config)
            print("  Successfully processed.")
        except Exception as e:
            print(f"  Error processing zip: {e}")

# --------------------------------------------------------------------------- #
# MAIN ENTRY POINT
# --------------------------------------------------------------------------- #

def main():
    # If '--cli' is in arguments, run command line mode
    if '--cli' in sys.argv:
        run_cli()
    else:
        # Run GUI mode
        root = tk.Tk()
        app = InboxMoverGUI(root)
        
        # Bring to front on Mac/Windows
        root.lift()
        root.attributes('-topmost', True)
        root.after_idle(root.attributes, '-topmost', False)
        
        root.mainloop()

if __name__ == '__main__':
    main()