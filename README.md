# Inbox Mover - the perfect FileButler companion

**Inbox Mover** is a cross-platform utility designed to process ```Transfer-``` folders typically found in myDRE ```i:``` and ```z:\inobox```. **Inbox Mover** processes files in these folders into a designated target folder while automatically resolving file conflicts based on your preferences. It features both a user-friendly Material-inspired graphical interface (GUI) and a Command-Line Interface (CLI) for automation.

**Inbox Mover** is optimized to process ZIP files uploaded with **myDRE FileButler** (typically containing a ```receipt.json``` file). 

<img src="https://github.com/flow4u/InboxMover/blob/main/Screenshot%202026-03-14%2011.35.06.png" alt="Screenshot 2026-03-14 11.35.06.png"/>

## 🛠️ Prerequisites

To run Inbox Mover, you need Python 3 installed on your system. The application uses Python's built-in libraries, including ```tkinter``` for the GUI, meaning no external packages (via ```pip```) are required.

### **Installing Python and Tkinter**

**On Windows**

1. Go to the official Python website: [https://python.org/downloads]

2. Download the latest Python 3 installer for Windows.

3. Run the installer.

4. **⚠️ CRITICAL STEPS**: Install for as admin for all users. At the bottom of the installer window, ensure you check the box that says "Add Python to PATH" before clicking "Install Now".

5. *Note: Tkinter is included by default with standard Windows Python installations.*

**On Linux (Ubuntu/Debian-based)**

Most Linux distributions come with Python pre-installed, but ```tkinter``` often needs to be installed separately. Open your terminal and run:

```
sudo apt update
sudo apt install python3 python3-tk
```


## 🚀 Installation & Setup

Because Inbox Mover is a standalone script, "installing" it just means placing it somewhere convenient and setting up a shortcut.

### Windows Setup

1. Extract the ZIP file containing ```inbox_mover.py```.

2. Move the ```inbox_mover.py``` file to ```C:\scripts\inbox_mover``` *(You may need to create the "inbox_mover" folder on your C: drive if it doesn't exist yet)*.

3. Right-click on ```inbox_mover.py``` and select **"Create shortcut"**.

4. Right-click the newly created shortcut and select **"Properties"**.

5. In the Properties window, locate the **"Run"** dropdown menu, change it to **"Minimized"**, and click **OK**. *(This ensures the background command prompt stays out of your way when launching the app)*.

6. Finally, move this customized shortcut to **C:\Users\Public\Desktop\**. This makes the application easily accessible on the desktop for every user account on the computer!

### Linux Setup

1. Extract the ZIP file containing inbox_mover.py.

2. You can place it in your ```~/Desktop``` directory or in a shared location like ```/opt/InboxMover``` if multiple users need access.

*Note: The application will automatically create a ```permit_configs``` folder in the same directory as the script to save your settings.*

## 🖱️ How to Use (GUI Mode)

To launch the graphical interface, double-click the shortcut you created on your Desktop.

1. Directories

* **Search Folder 1 & 2:** The root directories where the app looks for child folders starting with ```transfer-```. You can specify up to two search locations.

* **Target Folder:** The directory where the contents of the ZIP files will be extracted.

* **Processed Folder:** (Optional) The directory where the entire transfer folder is moved if the "Move" post-action is selected.

* **Receipt Folder:** (Optional) A dedicated folder where the ```receipt.json``` will be extracted (prepended with a timestamp to prevent overwriting).

2. Navigation

* Use the **'⇦ Prev'** and **'Next ⇨'** buttons (or your keyboard's Left/Right arrow keys) to cycle through the found transfer folders.

* Click **'↻ Refresh'** to manually rescan the Search Folders for new or modified transfer folders.

* Use the **"Open"** buttons next to directory paths to quickly view those locations in your native file explorer.

3. Conflict Resolution (If a file already exists)

* **Overwrite:** Replaces the existing file in the target folder with the new one.

* **Keep both:** Extracts the new file and adds a number to its filename (e.g., ```file (1).txt)```.

* **Rename existing:** Renames the file already on your disk by prepending a timestamp (e.g., ```YYMMDD-HHMMSS_filename.txt```), then extracts the new file normally.

4. Post Processing

* **Leave:** Leaves the files in places.

* **Delete:** Permanently deletes the entire transfer folder and all its contents after successful extraction.

* **Move:** Moves the entire transfer folder and all its contents to the Processed Folder.

5. Configurations & Config IDs

The application reads the ```receipt.json ```file inside the ZIP to identify a Config ID (previously Permit ID).

* If no ```receipt.json``` is found, a "DEFAULT" Config ID is assigned.

* Once you set up your folders and rules for a specific Config ID, click 'Save Config'.

* The next time you encounter a transfer folder with that exact Config ID, the application will automatically load your saved folder paths and settings.

## 💻 How to Use (CLI Mode)

You can run Inbox Mover headlessly from the command line, which is useful for automation scripts or batch processing.

### Basic Usage:

```python inbox_mover.py --cli -s <search_folder_1> <search_folder_2> -t <target_folder> [options]```


**Available Arguments:**

* ```--cli```: Triggers the Command-Line mode.

* ```-s, --search-folders```: (Required) One or more folders to search for transfer folders.

* ```-t, --target-folder```: (Required) Default target folder for extraction.

* ```-z, --target-zip-folder```: Target folder for moving processed folders (Processed Folder).

* ```-r, --receipt-folder```: Target folder specifically for the receipt.json file.

* ```-c, --conflict-action```: Action when extracted file already exists. Choices: overwrite, keep_both, rename_existing (Default: overwrite).

* ```-p, --post-action```: Action to perform after extraction. Choices: leave, delete, move (Default: leave).

*Note: If a saved configuration exists for a Config ID found during a CLI run, the script will prioritize the saved configuration over the CLI arguments.*
