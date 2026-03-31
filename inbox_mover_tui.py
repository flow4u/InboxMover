#!/usr/bin/env python3
"""
Inbox Mover TUI v0.17.3
A TUI conversion of the Inbox Mover utility.
Runs on standard Python libraries (uses curses for the interface).

WINDOWS USERS:
Standard Python on Windows does not include the 'curses' module.
To run this script, please install the compatibility layer via pip:
    pip install windows-curses
"""

import os
import sys
import json
import zipfile
import shutil
import datetime
import argparse
import getpass
import re
import fnmatch
import curses
import textwrap

VERSION = "0.17.3 (TUI)"

# --------------------------------------------------------------------------- #
# CORE LOGIC (Preserved from original)
# --------------------------------------------------------------------------- #

class InboxMoverCore:
    def __init__(self):
        self.local_config_dir = "permit_configs"
        if not os.path.exists(self.local_config_dir):
            os.makedirs(self.local_config_dir, exist_ok=True)

        self._config_cache = {}
        self._patterns_cache = {}

        settings = self.load_app_settings()
        self.use_global = settings.get("use_global", False)
        self.global_dir = settings.get("global_dir", "")

        self.set_workspace()

    def translate_path(self, path):
        if not path or not isinstance(path, str):
            return path
        p_check = path.replace('\\', '/')
        if sys.platform == "win32":
            if p_check.startswith("/mnt/data/"):
                suffix = path[10:].replace('/', '\\')
                return f"z:\\{suffix}"
            elif p_check.startswith("/mnt/inbox/"):
                suffix = path[11:].replace('/', '\\')
                return f"i:\\{suffix}"
            elif p_check == "/mnt/data": return "z:\\"
            elif p_check == "/mnt/inbox": return "i:\\"
        else:
            if p_check.lower().startswith("z:/"):
                suffix = path[3:].replace('\\', '/')
                return f"/mnt/data/{suffix}"
            elif p_check.lower().startswith("i:/"):
                suffix = path[3:].replace('\\', '/')
                return f"/mnt/inbox/{suffix}"
            elif p_check.lower() in ("z:", "z:/"): return "/mnt/data/"
            elif p_check.lower() in ("i:", "i:/"): return "/mnt/inbox/"
        return path

    def set_workspace(self):
        if self.use_global and self.global_dir:
            self.config_dir = self.global_dir
        else:
            self.config_dir = self.local_config_dir
        self.ensure_config_dir()
        self.log_file = os.path.join(self.config_dir, "process_log.jsonl")
        self.reload_cache()

    def reload_cache(self):
        self._config_cache = {}
        self._patterns_cache = {}
        if not os.path.exists(self.config_dir): return
        patterns_path = os.path.join(self.config_dir, "patterns.json")
        if os.path.exists(patterns_path):
            try:
                with open(patterns_path, 'r', encoding='utf-8') as f:
                    self._patterns_cache = json.load(f)
            except Exception: pass
        try:
            for f in os.listdir(self.config_dir):
                if f.endswith('.json') and f not in ('app_settings.json', 'patterns.json'):
                    permit_id = f[:-5]
                    try:
                        with open(os.path.join(self.config_dir, f), 'r', encoding='utf-8') as file:
                            self._config_cache[permit_id] = json.load(file)
                    except Exception: pass
        except Exception: pass

    def ensure_config_dir(self):
        if not os.path.exists(self.config_dir):
            os.makedirs(self.config_dir, exist_ok=True)
        patterns_file = os.path.join(self.config_dir, "patterns.json")
        if not os.path.exists(patterns_file):
            with open(patterns_file, 'w', encoding='utf-8') as f:
                json.dump({}, f, indent=4)

    def load_app_settings(self):
        settings_path = os.path.join(self.local_config_dir, "app_settings.json")
        if sys.platform == "win32": def_sf1, def_sf2 = "i:\\", "z:\\inbox"
        else: def_sf1, def_sf2 = "/mnt/inbox/", "/mnt/data/inbox"
        settings = {"search_folder_1": def_sf1, "search_folder_2": def_sf2, "use_global": False, "global_dir": ""}
        if os.path.exists(settings_path):
            try:
                with open(settings_path, 'r', encoding='utf-8') as f:
                    settings.update(json.load(f))
            except Exception: pass
        settings["search_folder_1"] = self.translate_path(settings.get("search_folder_1", ""))
        settings["search_folder_2"] = self.translate_path(settings.get("search_folder_2", ""))
        return settings

    def save_app_settings(self, settings):
        settings_path = os.path.join(self.local_config_dir, "app_settings.json")
        with open(settings_path, 'w', encoding='utf-8') as f:
            json.dump(settings, f, indent=4)

    def find_transfer_folders(self, search_folders):
        folders_data = []
        seen_paths = set()
        if isinstance(search_folders, str): search_folders = [search_folders]
        for search_folder in search_folders:
            if not search_folder or not os.path.isdir(search_folder): continue
            try: items = os.listdir(search_folder)
            except Exception: continue
            for item in items:
                if item.lower().startswith('transfer-'):
                    item_path = os.path.join(search_folder, item)
                    if item_path not in seen_paths and os.path.isdir(item_path):
                        seen_paths.add(item_path)
                        folders_data.append(self.inspect_transfer_folder(item_path))
        folders_data.sort(key=lambda x: x['folder_name'], reverse=True)
        return folders_data

    def inspect_transfer_folder(self, folder_path):
        data = {"folder_path": folder_path, "folder_name": os.path.basename(folder_path), "zip_path": None, "permitId": "DEFAULT", "receipt": None, "receipt_raw": "", "has_valid_zip": False, "can_process": False, "has_log": False, "file_list": []}
        if os.path.exists(os.path.join(folder_path, "Process.log")) or os.path.exists(os.path.join(folder_path, "Inbox Process.log")):
            data["has_log"] = True
        valid_zip_found, loose_receipt_data, loose_receipt_raw, has_loose_receipt = False, None, "", False
        for root, _, files in os.walk(folder_path):
            for file in files:
                rel_file = os.path.relpath(os.path.join(root, file), folder_path)
                data["file_list"].append(rel_file)
                if fnmatch.fnmatch(file.lower(), 'receipt*.json'):
                    has_loose_receipt = True
                    try:
                        with open(os.path.join(root, file), 'r', encoding='utf-8') as f:
                            raw = f.read()
                            if not loose_receipt_raw: loose_receipt_raw = raw
                            if raw.strip() and not loose_receipt_data:
                                try: loose_receipt_data = json.loads(raw)
                                except Exception: pass
                    except Exception: pass
        for root, _, files in os.walk(folder_path):
            for file in files:
                if file.lower().endswith('.zip') and not valid_zip_found:
                    zip_path = os.path.join(root, file)
                    data["zip_path"], data["has_valid_zip"], valid_zip_found = zip_path, True, True
                    if has_loose_receipt:
                        data["receipt"], data["receipt_raw"] = loose_receipt_data, loose_receipt_raw
                        data["permitId"] = loose_receipt_data.get("permitId", "DEFAULT") if loose_receipt_data else "DEFAULT"
                    else:
                        zip_info = self.inspect_zip(zip_path)
                        if zip_info:
                            data.update({"permitId": zip_info["permitId"], "receipt": zip_info["receipt"], "receipt_raw": zip_info["receipt_raw"]})
                    break
        if not valid_zip_found:
            if has_loose_receipt:
                data["receipt"], data["receipt_raw"] = loose_receipt_data, loose_receipt_raw
                data["permitId"] = loose_receipt_data.get("permitId", "DEFAULT") if loose_receipt_data else "DEFAULT"
                data["can_process"] = True
            else: data["can_process"] = len(data["file_list"]) > 0
        else: data["can_process"] = True
        return data

    def inspect_zip(self, zip_path):
        try:
            with zipfile.ZipFile(zip_path, 'r') as zf:
                receipt_filenames = [f for f in zf.namelist() if fnmatch.fnmatch(os.path.basename(f).lower(), 'receipt*.json')]
                if receipt_filenames:
                    content = zf.open(receipt_filenames[0]).read().decode('utf-8')
                    try: r_data = json.loads(content)
                    except: r_data = {}
                    return {"permitId": r_data.get("permitId", "DEFAULT"), "receipt": r_data, "receipt_raw": content}
        except: pass
        return None

    def save_config(self, permit_id, config_data):
        if not permit_id: return
        config_path = os.path.join(self.config_dir, f"{permit_id}.json")
        with open(config_path, 'w', encoding='utf-8') as f:
            json.dump(config_data, f, indent=4)
        self._config_cache[permit_id] = config_data

    def save_pattern(self, pattern, config_data):
        self._patterns_cache[pattern] = config_data
        with open(os.path.join(self.config_dir, "patterns.json"), 'w', encoding='utf-8') as f:
            json.dump(self._patterns_cache, f, indent=4)

    def write_log(self, status, folder_data, config, actions, message=""):
        log_entry = {"timestamp": datetime.datetime.now().isoformat(), "user": getpass.getuser(), "status": status, "folder_name": folder_data.get('folder_name', 'Unknown'), "config_id": folder_data.get('permitId', 'Unknown'), "files_processed": len(actions), "message": message}
        try:
            with open(self.log_file, 'a', encoding='utf-8') as f:
                f.write(json.dumps(log_entry) + '\n')
        except: pass

    def safe_copy(self, src, dst):
        try: shutil.copy2(src, dst)
        except: shutil.copyfile(src, dst)

    def process_zip(self, folder_data, config, progress_callback=None):
        target_folder = config.get('target_folder')
        if not target_folder: raise ValueError("Target folder missing.")
        os.makedirs(target_folder, exist_ok=True)
        actions_log = []

        try:
            folder_path = folder_data.get('folder_path')
            for root, _, files in os.walk(folder_path):
                for file in files:
                    src_path = os.path.join(root, file)
                    if file.lower() in ('process.log', 'inbox process.log'): continue
                    
                    if config.get('auto_extract', True) and file.lower().endswith('.zip'):
                        with zipfile.ZipFile(src_path, 'r') as zf:
                            for zinfo in zf.infolist():
                                if zinfo.is_dir(): continue
                                ext_path = os.path.join(target_folder, zinfo.filename.lstrip('/\\'))
                                os.makedirs(os.path.dirname(ext_path), exist_ok=True)
                                with zf.open(zinfo) as source, open(ext_path, "wb") as target:
                                    shutil.copyfileobj(source, target)
                                actions_log.append({"type": "extract", "dest": ext_path})
                    else:
                        rel = os.path.relpath(src_path, folder_path)
                        dst = os.path.join(target_folder, rel)
                        os.makedirs(os.path.dirname(dst), exist_ok=True)
                        self.safe_copy(src_path, dst)
                        actions_log.append({"type": "copy", "dest": dst})
            
            post = config.get('post_action', 'leave')
            if post == 'delete': shutil.rmtree(folder_path)
            elif post == 'move':
                dest_root = config.get('target_zip_folder') or target_folder
                dest = os.path.join(dest_root, folder_data['folder_name'])
                os.makedirs(dest_root, exist_ok=True)
                shutil.move(folder_path, dest)
                
            self.write_log("SUCCESS", folder_data, config, actions_log)
        except Exception as e:
            self.write_log("ERROR", folder_data, config, actions_log, str(e))
            raise e

# --------------------------------------------------------------------------- #
# TUI INTERFACE
# --------------------------------------------------------------------------- #

class InboxMoverTUI:
    def __init__(self, stdscr):
        self.stdscr = stdscr
        self.core = InboxMoverCore()
        self.settings = self.core.load_app_settings()
        self.folders_data = []
        self.current_index = 0
        self.running = True
        
        # UI State
        self.search_folders = [self.settings.get("search_folder_1", ""), self.settings.get("search_folder_2", "")]
        self.target_folder = ""
        self.processed_folder = ""
        self.conflict_action = "overwrite"
        self.post_action = "leave"
        self.auto_extract = True
        
        # Curses setup
        try: curses.curs_set(0)
        except: pass
            
        curses.start_color()
        curses.init_pair(1, curses.COLOR_CYAN, curses.COLOR_BLACK)   # Header
        curses.init_pair(2, curses.COLOR_BLACK, curses.COLOR_WHITE)  # Selection
        curses.init_pair(3, curses.COLOR_GREEN, curses.COLOR_BLACK)  # Success
        curses.init_pair(4, curses.COLOR_RED, curses.COLOR_BLACK)    # Error/Warning
        curses.init_pair(5, curses.COLOR_YELLOW, curses.COLOR_BLACK) # Labels
        
        self.refresh_queue()

    def refresh_queue(self):
        self.folders_data = self.core.find_transfer_folders([f for f in self.search_folders if f])
        if self.folders_data:
            self.current_index = min(self.current_index, len(self.folders_data) - 1)
            self.update_fields_from_selection()
        else:
            self.current_index = 0

    def update_fields_from_selection(self):
        if not self.folders_data: return
        cd = self.folders_data[self.current_index]
        cfg = self.core._config_cache.get(cd['permitId'], self.core._config_cache.get("DEFAULT", {}))
        
        r = cd.get('receipt') or {}
        self.target_folder = self.core.translate_path(r.get('target_folder') or cfg.get('target_folder', ""))
        self.processed_folder = self.core.translate_path(r.get('process_folder') or cfg.get('target_zip_folder', ""))
        self.conflict_action = r.get('conflict_resolution') or cfg.get('conflict_action', "overwrite")
        self.post_action = r.get('post_processing') or cfg.get('post_action', "leave")
        self.auto_extract = r.get('auto_extract', cfg.get('auto_extract', True))

    def draw(self):
        self.stdscr.clear()
        height, width = self.stdscr.getmaxyx()
        
        if height < 12 or width < 45:
            self.stdscr.addstr(0, 0, "Terminal too small.")
            self.stdscr.refresh()
            return

        # Header
        self.stdscr.attron(curses.color_pair(1) | curses.A_BOLD)
        self.stdscr.addstr(0, 2, f"INBOX MOVER v{VERSION}")
        mode_text = "TEAM SHARED" if self.core.use_global else "PERSONAL LOCAL"
        self.stdscr.addstr(0, width - len(mode_text) - 5, mode_text, curses.color_pair(5))
        self.stdscr.attroff(curses.color_pair(1) | curses.A_BOLD)
        self.stdscr.addstr(1, 0, "─" * width)

        # Left Column: Queue
        col_width = width // 3
        self.stdscr.addstr(2, 2, "PENDING QUEUE", curses.A_UNDERLINE)
        if not self.folders_data:
            self.stdscr.addstr(4, 2, "No folders found.")
        else:
            for i, folder in enumerate(self.folders_data):
                if i + 4 >= height - 3: break
                attr = curses.color_pair(2) if i == self.current_index else curses.A_NORMAL
                indicator = " > " if i == self.current_index else "   "
                name = folder['folder_name'][:col_width-10]
                status = "[OK]" if folder['can_process'] else "[!!]"
                try: self.stdscr.addstr(4 + i, 1, f"{indicator}{name} {status}", attr)
                except curses.error: pass

        # Vertical Divider
        for r in range(2, height - 2):
            try: self.stdscr.addstr(r, col_width, "│")
            except curses.error: pass

        # Right Column: Details
        if self.folders_data:
            cd = self.folders_data[self.current_index]
            start_x = col_width + 3
            if start_x < width:
                self.stdscr.addstr(2, start_x, f"FOLDER: {cd['folder_name'][:width-start_x-1]}", curses.A_BOLD)
                self.stdscr.addstr(3, start_x, f"Config ID: {cd['permitId'][:width-start_x-1]}", curses.color_pair(5))
                
                # Fields
                self.stdscr.addstr(5, start_x, "TARGET FOLDER:")
                self.stdscr.addstr(6, start_x + 2, (self.target_folder or "<not set>")[:width-start_x-4], curses.A_DIM)
                
                self.stdscr.addstr(8, start_x, "PROCESSED FOLDER:")
                self.stdscr.addstr(9, start_x + 2, (self.processed_folder or "<not set>")[:width-start_x-4], curses.A_DIM)
                
                self.stdscr.addstr(11, start_x, f"CONFLICT: {self.conflict_action}")
                self.stdscr.addstr(12, start_x, f"POST ACTION: {self.post_action}")
                self.stdscr.addstr(13, start_x, f"AUTO EXTRACT: {'Yes' if self.auto_extract else 'No'}")

                # File List
                self.stdscr.addstr(15, start_x, "FILES:", curses.A_UNDERLINE)
                for i, f in enumerate(cd['file_list'][:5]):
                    if 16 + i < height - 3:
                        self.stdscr.addstr(16 + i, start_x + 2, f"- {f[:width-start_x-10]}")
                if len(cd['file_list']) > 5 and 21 < height - 3:
                    self.stdscr.addstr(21, start_x + 2, f"... and {len(cd['file_list'])-5} more.")

        # Footer / Commands
        footer_y = height - 2
        self.stdscr.addstr(footer_y - 1, 0, "─" * width)
        commands = [("P", "Process"), ("E", "Edit Rules"), ("S", "Settings"), ("L", "Logs"), ("Q", "Quit")]
        cmd_str = "  ".join([f"[{k}] {v}" for k, v in commands])
        try: self.stdscr.addstr(footer_y, 2, cmd_str[:width-3])
        except curses.error: pass

        self.stdscr.refresh()

    def get_input(self, prompt, initial_value=""):
        height, width = self.stdscr.getmaxyx()
        self.stdscr.move(height-1, 0)
        self.stdscr.clrtoeol()
        try: self.stdscr.addstr(height-1, 2, prompt[:width-10], curses.color_pair(1))
        except: pass
        self.stdscr.refresh()
        curses.echo()
        try:
            input_bytes = self.stdscr.getstr(height-1, len(prompt) + 2)
            input_str = input_bytes.decode('utf-8').strip()
        except: input_str = ""
        curses.noecho()
        return input_str or initial_value

    def show_message(self, title, msg, color=0):
        self.stdscr.clear()
        h, w = self.stdscr.getmaxyx()
        self.stdscr.addstr(max(0, h//2 - 2), max(0, (w - len(title))//2), title, curses.A_BOLD | curses.color_pair(color))
        wrapped = textwrap.wrap(msg, width=max(10, w-10))
        for i, line in enumerate(wrapped):
            if h//2 + i < h - 1:
                self.stdscr.addstr(h//2 + i, max(0, (w - len(line))//2), line)
        self.stdscr.addstr(min(h-1, h//2 + len(wrapped) + 2), max(0, (w - 18)//2), "Press any key...", curses.A_DIM)
        self.stdscr.getch()

    def handle_settings(self):
        """Dedicated settings management menu."""
        sub_running = True
        idx = 0
        while sub_running:
            self.stdscr.clear()
            h, w = self.stdscr.getmaxyx()
            self.stdscr.addstr(2, 4, "APP SETTINGS", curses.A_BOLD | curses.color_pair(1))
            self.stdscr.addstr(3, 4, "─────────────────")
            
            opts = [
                f"Search Folder 1: {self.search_folders[0]}",
                f"Search Folder 2: {self.search_folders[1]}",
                f"Workspace Mode:  {'TEAM SHARED' if self.core.use_global else 'PERSONAL LOCAL'}",
                f"Global Directory: {self.core.global_dir or '<not set>'}",
                "Save and Return",
                "Cancel"
            ]
            
            for i, opt in enumerate(opts):
                attr = curses.color_pair(2) if i == idx else curses.A_NORMAL
                self.stdscr.addstr(5 + i, 6, opt, attr)
            
            self.stdscr.addstr(h-3, 4, "Use Arrows to select, Enter to modify/confirm.")
            self.stdscr.refresh()
            
            k = self.stdscr.getch()
            if k == curses.KEY_UP: idx = (idx - 1) % len(opts)
            elif k == curses.KEY_DOWN: idx = (idx + 1) % len(opts)
            elif k in (10, 13, curses.KEY_ENTER):
                if idx == 0: self.search_folders[0] = self.get_input("New Search Path 1: ", self.search_folders[0])
                elif idx == 1: self.search_folders[1] = self.get_input("New Search Path 2: ", self.search_folders[1])
                elif idx == 2: self.core.use_global = not self.core.use_global
                elif idx == 3: self.core.global_dir = self.get_input("New Global Path: ", self.core.global_dir)
                elif idx == 4:
                    self.settings.update({
                        "search_folder_1": self.search_folders[0],
                        "search_folder_2": self.search_folders[1],
                        "use_global": self.core.use_global,
                        "global_dir": self.core.global_dir
                    })
                    self.core.save_app_settings(self.settings)
                    self.core.set_workspace() # Re-init workspace paths
                    self.refresh_queue()
                    sub_running = False
                elif idx == 5: sub_running = False

    def handle_edit_rules(self):
        """Edit rules for the current folder."""
        if not self.folders_data: return
        sub_running = True
        idx = 0
        while sub_running:
            self.stdscr.clear()
            h, w = self.stdscr.getmaxyx()
            self.stdscr.addstr(2, 4, "EDIT FOLDER RULES (OVERRIDE)", curses.A_BOLD | curses.color_pair(1))
            self.stdscr.addstr(3, 4, "──────────────────────────────")
            
            opts = [
                f"Target Folder:    {self.target_folder}",
                f"Processed Folder: {self.processed_folder}",
                f"Conflict Action:  {self.conflict_action}",
                f"Post Action:      {self.post_action}",
                f"Auto Extract:     {'Yes' if self.auto_extract else 'No'}",
                "Done",
                "Cancel"
            ]
            
            for i, opt in enumerate(opts):
                attr = curses.color_pair(2) if i == idx else curses.A_NORMAL
                self.stdscr.addstr(5 + i, 6, opt, attr)
            
            self.stdscr.refresh()
            k = self.stdscr.getch()
            if k == curses.KEY_UP: idx = (idx - 1) % len(opts)
            elif k == curses.KEY_DOWN: idx = (idx + 1) % len(opts)
            elif k in (10, 13, curses.KEY_ENTER):
                if idx == 0: self.target_folder = self.get_input("Target: ", self.target_folder)
                elif idx == 1: self.processed_folder = self.get_input("Processed: ", self.processed_folder)
                elif idx == 2:
                    actions = ["overwrite", "keep_both", "rename_existing"]
                    self.conflict_action = actions[(actions.index(self.conflict_action) + 1) % 3]
                elif idx == 3:
                    actions = ["leave", "delete", "move"]
                    self.post_action = actions[(actions.index(self.post_action) + 1) % 3]
                elif idx == 4: self.auto_extract = not self.auto_extract
                elif idx == 5: sub_running = False
                elif idx == 6: 
                    self.update_fields_from_selection() # Reset
                    sub_running = False

    def handle_process(self):
        if not self.folders_data: return
        cd = self.folders_data[self.current_index]
        cfg = {
            "target_folder": self.target_folder,
            "target_zip_folder": self.processed_folder,
            "conflict_action": self.conflict_action,
            "post_action": self.post_action,
            "auto_extract": self.auto_extract
        }
        try:
            self.core.process_zip(cd, cfg)
            self.show_message("SUCCESS", f"Processed {cd['folder_name']} successfully.", 3)
            self.refresh_queue()
        except Exception as e:
            self.show_message("ERROR", str(e), 4)

    def run(self):
        while self.running:
            self.draw()
            key = self.stdscr.getch()

            if key == ord('q') or key == ord('Q'):
                self.running = False
            elif key == curses.KEY_UP:
                self.current_index = max(0, self.current_index - 1)
                self.update_fields_from_selection()
            elif key == curses.KEY_DOWN:
                self.current_index = min(len(self.folders_data) - 1, self.current_index + 1)
                self.update_fields_from_selection()
            elif key == ord('r') or key == ord('R'):
                self.refresh_queue()
            elif key == ord('p') or key == ord('P'):
                self.handle_process()
            elif key == ord('s') or key == ord('S'):
                self.handle_settings()
            elif key == ord('e') or key == ord('E'):
                self.handle_edit_rules()
            elif key == ord('l') or key == ord('L'):
                if os.path.exists(self.core.log_file):
                    try:
                        with open(self.core.log_file, 'r') as f:
                            lines = f.readlines()
                            logs = "".join(lines[-10:]) if lines else "Log is empty."
                            self.show_message("RECENT LOGS", logs)
                    except: self.show_message("LOGS", "Error reading log file.")
                else: self.show_message("LOGS", "No log file found.")

def main():
    parser = argparse.ArgumentParser(description="Inbox Mover TUI")
    parser.add_argument('--cli', action='store_true', help="Run in non-interactive CLI mode")
    args = parser.parse_args()

    if args.cli:
        print("CLI Mode currently requires arguments. See help.")
    else:
        curses.wrapper(lambda stdscr: InboxMoverTUI(stdscr).run())

if __name__ == "__main__":
    main()