#!/usr/bin/env python3
"""
Inbox Mover v0.1
the FileButler companion
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

# Tkinter imports
import tkinter as tk
from tkinter import ttk, filedialog, messagebox

VERSION = "0.1"
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
        return {"dark_mode": True, "font_size": 10, "window_geometry": "800x650"}

    def save_app_settings(self, settings):
        settings_path = os.path.join(CONFIG_DIR, "app_settings.json")
        with open(settings_path, 'w') as f:
            json.dump(settings, f, indent=4)

    def find_zips(self, search_folder):
        """Recursively find all zip files and read their receipt.json."""
        zips_data = []
        if not os.path.isdir(search_folder):
            return zips_data

        for root, _, files in os.walk(search_folder):
            for file in files:
                if file.lower().endswith('.zip'):
                    zip_path = os.path.join(root, file)
                    zip_info = self.inspect_zip(zip_path)
                    if zip_info:
                        zips_data.append(zip_info)
        return zips_data

    def inspect_zip(self, zip_path):
        """Read a zip file to extract receipt.json without full extraction."""
        data = {
            "path": zip_path,
            "name": os.path.basename(zip_path),
            "permitId": "UNKNOWN",
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
                            data["permitId"] = receipt_json.get("permitId", "UNKNOWN")
                        except json.JSONDecodeError:
                            pass
        except zipfile.BadZipFile:
            return None # Skip invalid zips
        except Exception as e:
            print(f"Error inspecting {zip_path}: {e}")
            return None

        return data

    def load_config(self, permit_id):
        """Load configuration for a specific permit ID."""
        if not permit_id or permit_id == "UNKNOWN":
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
        """Save configuration for a specific permit ID."""
        if not permit_id or permit_id == "UNKNOWN":
            raise ValueError("Cannot save configuration for an UNKNOWN Permit ID.")
        config_path = os.path.join(CONFIG_DIR, f"{permit_id}.json")
        with open(config_path, 'w') as f:
            json.dump(config_data, f, indent=4)

    def process_zip(self, zip_data, config, progress_callback=None):
        """
        Extract the zip and apply conflict resolution and post-processing.
        config requires: target_folder, conflict_action, post_action, target_zip_folder
        """
        zip_path = zip_data['path']
        target_folder = config.get('target_folder')
        conflict_action = config.get('conflict_action', 'overwrite')
        post_action = config.get('post_action', 'leave')
        target_zip_folder = config.get('target_zip_folder')

        if not target_folder or not os.path.isdir(target_folder):
            raise ValueError(f"Target folder '{target_folder}' is invalid.")

        with zipfile.ZipFile(zip_path, 'r') as zf:
            file_list = [f for f in zf.infolist() if not f.is_dir()]
            total_files = len(file_list)

            for i, zinfo in enumerate(file_list):
                # Sanitize target path to prevent directory traversal
                safe_name = zinfo.filename.lstrip('/')
                extracted_path = os.path.join(target_folder, safe_name)
                extracted_dir = os.path.dirname(extracted_path)
                
                os.makedirs(extracted_dir, exist_ok=True)

                if os.path.exists(extracted_path):
                    if conflict_action == 'overwrite':
                        pass # proceed to overwrite
                    elif conflict_action == 'keep_both':
                        # Add number to the new file being extracted
                        base, ext = os.path.splitext(extracted_path)
                        counter = 1
                        while os.path.exists(f"{base} ({counter}){ext}"):
                            counter += 1
                        extracted_path = f"{base} ({counter}){ext}"
                    elif conflict_action == 'rename_existing':
                        # Rename the existing file on disk
                        timestamp = datetime.datetime.now().strftime("%y%m%d-%H%M%S")
                        base, ext = os.path.splitext(extracted_path)
                        dirname = os.path.dirname(extracted_path)
                        filename = os.path.basename(extracted_path)
                        renamed_path = os.path.join(dirname, f"{timestamp}_{filename}")
                        
                        # Handle extremely rare collision of timestamps
                        if os.path.exists(renamed_path):
                            counter = 1
                            while os.path.exists(f"{renamed_path}_{counter}"):
                                counter += 1
                            renamed_path = f"{renamed_path}_{counter}"
                            
                        os.rename(extracted_path, renamed_path)
                        # The new file will now be extracted to extracted_path

                with zf.open(zinfo) as source, open(extracted_path, "wb") as target:
                    shutil.copyfileobj(source, target)

                if progress_callback:
                    progress_callback(i + 1, total_files)

        # Post Processing
        if post_action == 'delete':
            os.remove(zip_path)
        elif post_action == 'move':
            if not target_zip_folder or not os.path.isdir(target_zip_folder):
                raise ValueError(f"Target zip folder '{target_zip_folder}' is invalid.")
            dest_path = os.path.join(target_zip_folder, os.path.basename(zip_path))
            
            # handle if zip already exists in target zip folder
            if os.path.exists(dest_path):
                base, ext = os.path.splitext(dest_path)
                counter = 1
                while os.path.exists(f"{base}_{counter}{ext}"):
                    counter += 1
                dest_path = f"{base}_{counter}{ext}"
                
            shutil.move(zip_path, dest_path)
        # 'leave' does nothing


# --------------------------------------------------------------------------- #
# GUI APPLICATION
# --------------------------------------------------------------------------- #

class InboxMoverGUI:
    def __init__(self, root):
        self.root = root
        self.root.title(f"Inbox Mover v{VERSION} - the FileButler companion")
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
        
        self.zips_data = []
        self.current_index = -1

        # Variables
        self.search_folder_var = tk.StringVar()
        self.target_folder_var = tk.StringVar()
        self.target_zip_folder_var = tk.StringVar()
        
        self.conflict_action_var = tk.StringVar(value="overwrite")
        self.post_action_var = tk.StringVar(value="leave")
        
        self.zip_name_var = tk.StringVar(value="No Zip Files Found")
        self.permit_id_var = tk.StringVar(value="")

        self.setup_ui()
        self.apply_theme()
        self.apply_fonts()
        self.bind_keys()

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
        self.lbl_version = ttk.Label(title_frame, text=f"the FileButler companion - version {VERSION}")
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
        
        self.create_folder_row(folder_frame, "Search Folder:", self.search_folder_var, 0, self.on_search_folder_changed)
        self.create_folder_row(folder_frame, "Target Folder:", self.target_folder_var, 1)
        self.create_folder_row(folder_frame, "Target Zip Folder:", self.target_zip_folder_var, 2)

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
        self.lbl_zip_name = ttk.Label(info_frame, textvariable=self.zip_name_var)
        self.lbl_zip_name.pack(anchor=tk.W)
        self.lbl_permit_id = ttk.Label(info_frame, textvariable=self.permit_id_var)
        self.lbl_permit_id.pack(anchor=tk.W)

        # Config Save
        self.btn_save_config = ttk.Button(nav_frame, text="Save Permit Id Config", command=self.save_permit_config)
        self.btn_save_config.pack(side=tk.RIGHT)

        # --- Options Section ---
        options_frame = ttk.Frame(main_frame)
        options_frame.pack(fill=tk.X, pady=5)
        
        # Conflict Options
        conflict_frame = ttk.LabelFrame(options_frame, text="If the file already exists then:", padding="10")
        conflict_frame.pack(side=tk.LEFT, fill=tk.BOTH, expand=True, padx=(0, 5))
        ttk.Radiobutton(conflict_frame, text="Overwrite existing file", variable=self.conflict_action_var, value="overwrite").pack(anchor=tk.W)
        ttk.Radiobutton(conflict_frame, text="Keep both (add number)", variable=self.conflict_action_var, value="keep_both").pack(anchor=tk.W)
        ttk.Radiobutton(conflict_frame, text="Rename existing file", variable=self.conflict_action_var, value="rename_existing").pack(anchor=tk.W)

        # Post Action Options
        post_frame = ttk.LabelFrame(options_frame, text="After processing:", padding="10")
        post_frame.pack(side=tk.RIGHT, fill=tk.BOTH, expand=True, padx=(5, 0))
        ttk.Radiobutton(post_frame, text="Leave the zip in place", variable=self.post_action_var, value="leave").pack(anchor=tk.W)
        ttk.Radiobutton(post_frame, text="Delete the zip", variable=self.post_action_var, value="delete").pack(anchor=tk.W)
        ttk.Radiobutton(post_frame, text="Move the zip", variable=self.post_action_var, value="move").pack(anchor=tk.W)

        # --- Receipt.json Text Area ---
        text_frame = ttk.LabelFrame(main_frame, text="<receipt.json>", padding="10")
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
        
        def browse():
            folder = filedialog.askdirectory()
            if folder:
                str_var.set(folder)
                if callback:
                    callback()
                    
        ttk.Button(parent, text="Browse...", command=browse).grid(row=row, column=2, sticky=tk.E, pady=3)
        if callback:
            entry.bind('<FocusOut>', lambda e: callback())
            entry.bind('<Return>', lambda e: callback())

    def apply_fonts(self):
        style = ttk.Style()
        style.configure(".", font=("Helvetica", self.base_font_size))
        style.configure("TButton", font=("Helvetica", self.base_font_size))
        style.configure("TLabelframe.Label", font=("Helvetica", self.base_font_size, "bold"))
        
        self.lbl_title.config(font=("Helvetica", self.base_font_size + 14, "bold"))
        self.lbl_version.config(font=("Helvetica", self.base_font_size, "italic"))
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
            
            self.theme_btn.config(text="☾ Dark Mode")

        # Configure TTK styles
        style.configure(".", background=bg_color, foreground=fg_color, fieldbackground=entry_bg)
        style.configure("TFrame", background=bg_color)
        style.configure("TLabelframe", background=bg_color, foreground=fg_color)
        style.configure("TLabelframe.Label", background=bg_color, foreground=fg_color)
        style.configure("TButton", background=btn_bg, foreground=btn_fg, padding=5, borderwidth=0)
        style.map("TButton", background=[('active', btn_active)])
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
            "window_geometry": self.root.geometry()
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
• Search Folder: The root directory where the app looks for ZIP files. It searches all subfolders recursively.
• Target Folder: The directory where the contents of the ZIP will be extracted.
• Target Zip Folder: (Optional) The directory where the original ZIP file is moved if the "Move the zip" post-action is selected.

2. NAVIGATION
• Use the '⇦ Prev' and 'Next ⇨' buttons (or your keyboard's Left/Right arrow keys) to cycle through the found ZIP files.
• Click '↻ Refresh' to rescan the Search Folder for new or modified ZIPs.

3. CONFLICT RESOLUTION (If file already exists in Target Folder)
• Overwrite: Replaces the existing file with the new one from the ZIP.
• Keep both: Extracts the new file and adds a number to its filename (e.g., file (1).txt).
• Rename existing: Renames the file already on your disk by prepending a timestamp (YYMMDD-HHMMSS_filename), then extracts the new file normally.

4. POST PROCESSING
• Leave: Keeps the original ZIP file exactly where it was found.
• Delete: Permanently deletes the ZIP file after successful extraction.
• Move: Moves the ZIP file to the specified 'Target Zip Folder'.

5. CONFIGURATIONS & PERMIT IDs
• The app reads 'receipt.json' inside the ZIP to find a 'Permit ID'.
• If you set up your folders and rules for a specific Permit ID, click 'Save Permit Id Config'.
• The next time you encounter a ZIP with that exact Permit ID, the application will automatically load your saved folder paths and conflict/post-action settings.

CLI MODE
You can also run this application via the command line for automation. Run `python inbox_mover.py --cli --help` in your terminal for details.
"""
        help_text.insert(tk.END, instructions)
        help_text.config(state=tk.DISABLED)

    def bind_keys(self):
        self.root.bind("<Left>", lambda e: self.prev_zip())
        self.root.bind("<Right>", lambda e: self.next_zip())

    def on_search_folder_changed(self):
        folder = self.search_folder_var.get()
        if os.path.isdir(folder):
            self.zips_data = self.core.find_zips(folder)
            if self.zips_data:
                self.current_index = 0
            else:
                self.current_index = -1
                self.clear_zip_display()
                messagebox.showinfo("Info", "No valid zip files found in the search folder.")
            self.update_display()
        elif folder:
            messagebox.showwarning("Warning", "The specified search folder does not exist.")

    def clear_zip_display(self):
        self.zip_name_var.set("No Zip Files Found")
        self.permit_id_var.set("")
        self.set_receipt_text("")
        self.update_nav_buttons()

    def update_display(self):
        if not self.zips_data or self.current_index < 0 or self.current_index >= len(self.zips_data):
            self.clear_zip_display()
            return

        current_zip = self.zips_data[self.current_index]
        self.zip_name_var.set(f"[{self.current_index + 1}/{len(self.zips_data)}] {current_zip['name']}")
        self.permit_id_var.set(f"Permit ID: {current_zip['permitId']}")
        self.set_receipt_text(current_zip['receipt_raw'])
        
        # Attempt to load config for this permitId
        config = self.core.load_config(current_zip['permitId'])
        if config:
            if 'target_folder' in config: self.target_folder_var.set(config['target_folder'])
            if 'target_zip_folder' in config: self.target_zip_folder_var.set(config['target_zip_folder'])
            if 'conflict_action' in config: self.conflict_action_var.set(config['conflict_action'])
            if 'post_action' in config: self.post_action_var.set(config['post_action'])
            
        self.update_nav_buttons()

    def set_receipt_text(self, text):
        self.receipt_text.config(state=tk.NORMAL)
        self.receipt_text.delete(1.0, tk.END)
        self.receipt_text.insert(tk.END, text if text else "<No receipt.json found in this zip>")
        self.receipt_text.config(state=tk.DISABLED)

    def update_nav_buttons(self):
        has_zips = len(self.zips_data) > 0
        self.btn_prev.config(state=tk.NORMAL if has_zips and self.current_index > 0 else tk.DISABLED)
        self.btn_next.config(state=tk.NORMAL if has_zips and self.current_index < len(self.zips_data) - 1 else tk.DISABLED)
        self.btn_process.config(state=tk.NORMAL if has_zips else tk.DISABLED)
        self.btn_save_config.config(state=tk.NORMAL if has_zips else tk.DISABLED)

    def prev_zip(self):
        if self.current_index > 0:
            self.current_index -= 1
            self.update_display()

    def next_zip(self):
        if self.current_index < len(self.zips_data) - 1:
            self.current_index += 1
            self.update_display()

    def save_permit_config(self):
        if self.current_index < 0: return
        
        permit_id = self.zips_data[self.current_index]['permitId']
        if permit_id == "UNKNOWN":
            messagebox.showwarning("Warning", "Cannot save configuration: Permit ID is UNKNOWN.")
            return
            
        config = {
            "target_folder": self.target_folder_var.get(),
            "target_zip_folder": self.target_zip_folder_var.get(),
            "conflict_action": self.conflict_action_var.get(),
            "post_action": self.post_action_var.get()
        }
        
        try:
            self.core.save_config(permit_id, config)
            messagebox.showinfo("Success", f"Configuration saved for Permit ID: {permit_id}")
        except Exception as e:
            messagebox.showerror("Error", f"Failed to save configuration:\n{e}")

    def process_current_zip(self):
        if self.current_index < 0: return
        
        current_zip = self.zips_data[self.current_index]
        config = {
            "target_folder": self.target_folder_var.get(),
            "target_zip_folder": self.target_zip_folder_var.get(),
            "conflict_action": self.conflict_action_var.get(),
            "post_action": self.post_action_var.get()
        }

        # Validation
        if not config['target_folder']:
            messagebox.showerror("Error", "Please specify a Target Folder.")
            return
        if config['post_action'] == 'move' and not config['target_zip_folder']:
            messagebox.showerror("Error", "Please specify a Target Zip Folder when 'Move' is selected.")
            return

        self.btn_process.config(state=tk.DISABLED, text="Processing...")
        
        # Run in thread to prevent UI freezing
        def worker():
            try:
                self.core.process_zip(current_zip, config)
                self.root.after(0, self.on_process_success)
            except Exception as e:
                self.root.after(0, lambda err=e: self.on_process_error(err))

        threading.Thread(target=worker, daemon=True).start()

    def on_process_success(self):
        messagebox.showinfo("Success", "Zip processed successfully.")
        self.btn_process.config(text="PROCESS")
        
        # Refresh the list quietly as the zip might have been moved or deleted
        folder = self.search_folder_var.get()
        if os.path.isdir(folder):
            self.zips_data = self.core.find_zips(folder)
            if self.zips_data:
                # Ensure the index stays within bounds if files were removed
                if self.current_index >= len(self.zips_data):
                    self.current_index = max(0, len(self.zips_data) - 1)
            else:
                self.current_index = -1
            self.update_display()

    def on_process_error(self, err):
        messagebox.showerror("Processing Error", str(err))
        self.btn_process.config(state=tk.NORMAL, text="PROCESS")
        self.update_nav_buttons()


# --------------------------------------------------------------------------- #
# CLI APPLICATION
# --------------------------------------------------------------------------- #

def run_cli():
    parser = argparse.ArgumentParser(description=f"Inbox Mover v{VERSION} - the FileButler companion CLI")
    parser.add_argument('--cli', action='store_true', help=argparse.SUPPRESS) # Hide the switch that triggered this
    parser.add_argument('-s', '--search-folder', required=True, help='Folder to search for zip files')
    parser.add_argument('-t', '--target-folder', required=True, help='Default target folder for extraction')
    parser.add_argument('-z', '--target-zip-folder', help='Target folder for moving processed zips')
    parser.add_argument('-c', '--conflict-action', choices=['overwrite', 'keep_both', 'rename_existing'], default='overwrite', help='Action when extracted file already exists')
    parser.add_argument('-p', '--post-action', choices=['leave', 'delete', 'move'], default='leave', help='Action to perform on zip after extraction')
    
    args = parser.parse_args()

    core = InboxMoverCore()
    zips = core.find_zips(args.search_folder)
    
    if not zips:
        print(f"No valid zip files found in {args.search_folder}")
        return

    print(f"Found {len(zips)} zip files to process.")
    
    for zip_data in zips:
        print(f"\nProcessing: {zip_data['name']} (Permit ID: {zip_data['permitId']})")
        
        # Load specific config if exists, otherwise fallback to CLI args
        config = core.load_config(zip_data['permitId'])
        if not config:
            config = {
                "target_folder": args.target_folder,
                "target_zip_folder": args.target_zip_folder,
                "conflict_action": args.conflict_action,
                "post_action": args.post_action
            }
            print("  Using CLI arguments for configuration.")
        else:
            print("  Loaded saved configuration for this Permit ID.")

        # Validate post-action move requirement
        if config.get('post_action') == 'move' and not config.get('target_zip_folder'):
            print("  Error: Post action is 'move' but no target zip folder specified. Skipping.")
            continue
            
        try:
            core.process_zip(zip_data, config)
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