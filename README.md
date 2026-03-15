# **Inbox Mover \- the perfect FileButler companion (v0.9)**

**Inbox Mover** is a cross-platform utility designed to process files in the **transfer-** folders found in **i:** and **z:\\inbox** on [**myDRE**](https://mydre.org)**.**

**Inbox Mover** processes the files into a designated target folder while automatically resolving file conflicts based on your preferences. Zip-files will be automatically extracted. It features both a modern, user-friendly graphical interface (GUI) and a Command-Line Interface (CLI) for automation.

**Inbox Mover** is optimized to use the **receipt.json** in **myDRE FileButler.**

## **🛠️ Prerequisites**

To run Inbox Mover, you need **Python 3** installed on your system. The application uses Python's built-in libraries, including tkinter for the GUI, meaning no external packages (via pip) are required.

### **Installing Python and Tkinter**

#### **On Windows**

1. Go to the official Python website: [python.org/downloads](https://www.python.org/downloads/)  
2. Download the latest Python 3 installer for Windows.  
3. Run the installer.  
4. **⚠️ CRITICAL STEP:** At the bottom of the installer window, ensure you check the box that says **"Add Python to PATH"** before clicking "Install Now".  
5. *Note: Tkinter is included by default with standard Windows Python installations.*

#### **On Linux (Ubuntu/Debian-based)**

Most Linux distributions come with Python pre-installed, but tkinter often needs to be installed separately. Open your terminal and run:

sudo apt update  
sudo apt install python3 python3-tk

## **🚀 Installation & Setup**

Because Inbox Mover is a standalone script, "installing" it just means placing it somewhere convenient and setting up a shortcut.

### **Windows Setup**

1. Extract the ZIP file containing inbox\_mover.py.  
2. Move the inbox\_mover.py file to C:\\scripts\\ *(You may need to create the "scripts" folder on your C: drive if it doesn't exist yet)*.  
3. Right-click on inbox\_mover.py and select **"Create shortcut"**.  
4. Right-click the newly created shortcut and select **"Properties"**.  
5. In the Properties window, locate the **"Run"** dropdown menu, change it to **"Minimized"**, and click **OK**. *(This ensures the background command prompt stays out of your way when launching the app).*  
6. Finally, move this customized shortcut to C:\\Users\\Public\\Desktop\\. This makes the application easily accessible on the desktop for every user account on the computer\!

### **Linux Setup**

1. Extract the ZIP file containing inbox\_mover.py.  
2. You can place it in your \~/Desktop directory or in a shared location like /opt/InboxMover if multiple users need access.

*Note: The application will automatically create a permit\_configs folder in the same directory as the script to save your settings and logs.*

## **🖱️ How to Use (GUI Mode)**

To launch the graphical interface, double-click the shortcut you created on your Desktop.

**UI Controls:** You can adjust the application's appearance using the buttons in the top right:

* **☀ / ☾:** Toggle between Light and Dark mode.  
* **?:** Open the formatted Markdown Help Menu.  
* **A+ / A-:** Increase or decrease the application font size.  
* **Reset View:** Snap the window back to its default size (1100x950) and default font size (11).

### **1\. Directories**

* **Search Folder 1 & 2:** The root directories where the app looks for child folders starting with transfer-. You can specify up to two search locations (defaults to i:/ and z:/inbox).  
* **Target Folder:** The directory where the contents of the ZIP files will be extracted. Missing folders are created automatically.  
* **Processed Folder:** (Optional) The directory where the entire transfer folder is moved if the "Move" post-action is selected.  
* **Receipt Folder:** (Optional) A dedicated folder where the receipt.json will be extracted (prepended with a timestamp to prevent overwriting).

### **2\. Navigation**

* Use the **'⇦ Prev'** and **'Next ⇨'** buttons (or your keyboard's Left/Right arrow keys) to cycle through the found transfer folders.  
* Click **'↻ Refresh'** to manually rescan the Search Folders for new or modified transfer folders.  
* Use the **"📂 Open Folder"** button to quickly view the selected transfer- folder in your native file explorer.

### **3\. Conflict Resolution (If a file already exists)**

* **Overwrite:** Replaces the existing file in the target folder with the new one.  
* **Keep both:** Extracts the new file and adds a number to its filename (e.g., file (1).txt).  
* **Rename existing:** Renames the file *already on your disk* by prepending a timestamp (e.g., YYMMDD-HHMMSS\_filename.txt), then extracts the new file normally.

### **4\. Post Processing**

* **Leave the files in place:** Leaves the source files where they are.  
* **Delete the files:** Permanently deletes the entire transfer folder and all its contents after successful extraction.  
* **Move the files to Processed Folder:** Moves the entire transfer folder and all its contents to the Processed Folder.

### **5\. Configurations & Config IDs**

The application reads the receipt.json file inside the ZIP to identify a **Config ID** (previously Permit ID).

* If no receipt.json is found, a "DEFAULT" Config ID is assigned. The app will use your saved DEFAULT settings as a fallback.  
* Once you set up your folders and rules for a specific Config ID, click **'Save Config'**.  
* The next time you encounter a transfer folder with that exact Config ID, the application will automatically load your saved folder paths and settings over the DEFAULT baseline.

### **6\. Advanced Logging & Overrides**

* **Global Audit Logging:** Every processed file and resolved conflict is written to a machine-readable JSONL file (process\_log.jsonl). Click **'📄 View Log'** to inspect it.  
* **Local Process Logs:** When a folder is processed successfully (and not deleted), an Inbox Process.log file is dropped directly into the folder (or its new location in the Processed Folder).  
  * If this file exists, the GUI will dynamically display a **"Latest: \[Timestamp\] SUCCESS"** label.  
  * A **"📄 Process Log"** button will dynamically appear to let you view this specific folder's history in a clean, color-coded modal window.  
* **Receipt JSON Injection:** For complete traceability, the application will parse the successfully extracted receipt.json and cleanly inject the processing metadata directly into it under a new "processing\_logs" array.  
* **Receipt Overrides:** If the receipt.json inside the ZIP contains keys like target\_folder, process\_folder, receipt\_folder, conflict\_resolution, or post\_processing, these values will automatically override your GUI settings. The "Save Config" button will turn orange (Save Config \*) to notify you.  
* **Absolute Path Extraction:** If a file compressed within the ZIP archive contains an absolute path (e.g., C:\\reports\\data.csv), the app bypasses the "Target Folder" setting and extracts that specific file to its absolute path.

## **💻 How to Use (CLI Mode)**

You can run Inbox Mover headlessly from the command line, which is useful for automation scripts or batch processing.

**Basic Usage:**

python inbox\_mover.py \--cli \-s \<search\_folder\_1\> \<search\_folder\_2\> \-t \<target\_folder\> \[options\]

**Available Arguments:**

* \--cli: Triggers the Command-Line mode.  
* \-s, \--search-folders: (Required) One or more folders to search for transfer folders.  
* \-t, \--target-folder: (Required) Default target folder for extraction.  
* \-z, \--target-zip-folder: Target folder for moving processed folders (Processed Folder).  
* \-r, \--receipt-folder: Target folder specifically for the receipt.json file.  
* \-c, \--conflict-action: Action when extracted file already exists. Choices: overwrite, keep\_both, rename\_existing (Default: overwrite).  
* \-p, \--post-action: Action to perform after extraction. Choices: leave, delete, move (Default: leave).