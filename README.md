# Inbox Mover - the FileButler companion

**Inbox Mover** is a cross-platform utility designed to process and extract ZIP files (typically containing a ```receipt.json``` file) into a designated target folder while automatically resolving file conflicts based on your preferences. It features both a user-friendly Material-inspired graphical interface (GUI) and a Command-Line Interface (CLI) for automation.

## 🛠️ Prerequisites

To run Inbox Mover, you need Python 3 installed on your system. The application uses Python's built-in libraries, including tkinter for the GUI, meaning no external packages (via pip) are required.

### Installing Python and Tkinter

#### On Windows

1. Go to the official Python website: python.org/downloads

2. Download the latest Python 3 installer for Windows.

3. Run the installer.

4. ⚠️ CRITICAL STEP: At the bottom of the installer window, ensure you check the box that says **"Add Python to PATH"** before clicking "Install Now".

5. *Note: Tkinter is included by default with standard Windows Python installations.*

##### On Linux (Ubuntu/Debian-based)

Most Linux distributions come with Python pre-installed, but ```tkinter``` often needs to be installed separately. Open your terminal and run:

```
sudo apt update
sudo apt install python3 python3-tk
```


## 🚀 Installation & Setup

Because Inbox Mover is a standalone script, "installing" it just means placing it somewhere convenient.

1. Extract the ZIP file containing ```inbox_mover.py```.

2. **Suggested Location (Windows):** Move the extracted folder to ```C:\Users\Public\Desktop\InboxMover```. This makes the tool easily accessible to any user account on that computer. You can also right-click inbox_mover.py and select "Send to -> Desktop (create shortcut)".

3. **Suggested Location (Linux):** You can place it in your ```~/Desktop directory``` or in a shared location like ```/opt/InboxMover``` if multiple users need access.

*Note: The application will automatically create a ```permit_configs``` folder in the same directory as the script to save your settings.*

## 🖱️ How to Use (GUI Mode)

To launch the graphical interface:

* **Windows:** Simply double-click the ```inbox_mover.py``` file.

* **Linux/Mac:** Open a terminal, navigate to the folder, and run ```python3 inbox_mover.py```.

**1. Directories**

* **Search Folder:** The root directory where the app looks for ZIP files. It searches all subfolders recursively.

* **Target Folder:** The directory where the contents of the ZIP will be extracted.

* **Target Zip Folder:** (Optional) The directory where the original ZIP file is moved if the "Move the zip" post-action is selected.

**2. Navigation**

* Use the '**⇦ Prev**' and '**Next ⇨**' buttons (or your keyboard's Left/Right arrow keys) to cycle through the found ZIP files.

* Click '**↻ Refresh**' to manually rescan the Search Folder for new or modified ZIPs.

**3. Conflict Resolution** (If a file already exists)

* **Overwrite:** Replaces the existing file in the target folder with the new one from the ZIP.

* **Keep both:** Extracts the new file and adds a number to its filename (e.g., ```file (1).txt)```.

* **Rename existing:** Renames the file already on your disk by prepending a timestamp (e.g., ```YYMMDD-HHMMSS_filename.txt```), then extracts the new file normally.

**4. Post Processing**

* **Leave:** Keeps the original ZIP file exactly where it was found.

* **Delete:** Permanently deletes the ZIP file after successful extraction.

* **Move:** Moves the ZIP file to the specified 'Target Zip Folder'.

**5. Configurations & Permit IDs**

The application reads the ```receipt.json``` file inside the ZIP to identify a Permit ID.

* If you set up your folders and rules for a specific Permit ID, click 'Save Permit Id Config'.

* The next time you encounter a ZIP with that exact Permit ID, the application will automatically load your saved folder paths and settings.

## 💻 How to Use (CLI Mode)

You can run Inbox Mover headlessly from the command line, which is useful for automation scripts or batch processing.

**Basic Usage:**

```python inbox_mover.py --cli -s <search_folder> -t <target_folder> [options]```


**Available Arguments:**

* ```--cli```: Triggers the Command-Line mode.

* ```-s, --search-folder```: (Required) Folder to search for ZIP files.

* ```-t, --target-folder```: (Required) Default target folder for extraction.

* ```-z, --target-zip-folder```: Target folder for moving processed ZIPs.

* ```-c, --conflict-action```: Action when extracted file already exists. Choices: ```overwrite```, ```keep_both```, ```rename_existing``` (Default: ```overwrite```).

* ```-p, --post-action```: Action to perform on ZIP after extraction. Choices: ```leave```, ```delete```, ```move``` (Default: ```leave```).

*Note: If a saved configuration exists for a Permit ID found during a CLI run, the script will prioritize the saved configuration over the CLI arguments.*
