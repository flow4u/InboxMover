# **Inbox Mover (v0.17.4)**

**Inbox Mover** is a cross-platform utility designed to process files in the **transfer-** folders found in **i:** and **z:\\inbox** on [**myDRE**](https://mydre.org)**.** 

**Inbox Mover** processes the files into a designated target folder while automatically resolving file conflicts based on your preferences. Zip-files will be automatically extracted. It features both a modern, user-friendly graphical interface (GUI).

See also [**Inbox Mover FAQ**](https://github.com/flow4u/InboxMover/blob/main/Inbox%20Mover%20FAQ.md) for a quick read how Inbox Mover can help you.

![Inbox Mover Screenshot](https://github.com/flow4u/InboxMover/blob/main/Inbox_Mover.png)

*An expermimental TUI, Terminal User Interface, [inbox_mover_tui.py](https://github.com/flow4u/InboxMover/blob/main/inbox_mover_tui.py) is also available. Should work without any additional installation on Linux (both RDP and SSH), on Windows it requires pip install windows-curses.*


## **📜 Open Source & Customization**

**Inbox Mover** is open-source and completely free to use, modify, and distribute.

Because it is written entirely in standard Python using built-in libraries, it is highly adaptable. If you want to tailor the tool to your specific workflow, add new features, or change the interface, Large Language Models (LLMs) are quite helpful\! You can simply paste the code into your favorite AI assistant and ask for the usable changes you need.

## **🔌 Developer Plugins (Alpha)**

An alpha version of **inbox\_mover\_plugin.py** and **inbox\_mover\_plugin.R** is now available\!

These plugins act as a lightweight utility to process, extract, and log ZIP files (and standard files) from a specific source folder. They are highly flexible and can be used as a standalone script or seamlessly integrated directly into your own Python and R codebase for automated data pipelines.

## **🛠️ Installation**

To run Inbox Mover, you need **Python 3** installed on your system. The application uses Python's built-in libraries, including tkinter for the GUI, meaning no external packages (via pip) are required.

### **Windows**

#### **On Windows (1)** -- no python needed, but you must be able to download an **exe**

* See [**inbox_mover.exe**](https://github.com/flow4u/InboxMover/releases/tag/windows)


#### **On Windows (2)**

##### Prerequisit
1. **AllowList**: python.org, pypi.org, pythonhosted.org
2. Go to the official Python website: [python.org/downloads](https://www.python.org/downloads/)  
3. Download the latest Python 3 installer for Windows.  
4. Run the installer.  
5. **⚠️ CRITICAL STEP:** At the bottom of the installer window, ensure you check the box that says **"Add Python to PATH"** before clicking "Install Now".  
6. *Note: Tkinter is included by default with standard Windows Python installations.*


#### Installation
1. Download [**inbox\_mover.py**](https://github.com/flow4u/InboxMover/blob/main/inbox_mover.py).  
2. Move the inbox\_mover.py file to C:\\helper\\ *(You may need to create the "helper" folder on your C: drive if it doesn't exist yet)*.  
3. Right-click on inbox\_mover.py and select **"Create shortcut"**.  
4. Right-click the newly created shortcut and select **"Properties"**.  
5. In the Properties window, locate the **"Run"** dropdown menu, change it to **"Minimized"**, and click **OK**. *(This ensures the background command prompt stays out of your way when launching the app).*  
6. Finally, move this customized shortcut to C:\\Users\\Public\\Desktop\\. This makes the application easily accessible on the desktop for every user account on the computer\!



#### **On Linux**

##### Prerequisit
1. **AllowList**: archive.ubuntu.com

Most Linux distributions come with Python pre-installed, but tkinter often needs to be installed separately. Open your terminal and run:

2. sudo apt update  
3. sudo apt install python3 python3-tk


##### Installation

1. Download [**inbox\_mover.py**](https://github.com/flow4u/InboxMover/blob/main/inbox_mover.py).
2. You can place it in your \~/Desktop directory or in a shared location like /opt/InboxMover if multiple users need access.

*Note: The application will automatically create a permit\_configs folder in the same directory as the script to save your settings and logs.*

## **🖱️ How to Use (GUI Mode)**

To launch the graphical interface, double-click the shortcut you created on your Desktop.

### **⌨️ Keyboard Support**

Inbox Mover is optimized for speed with robust keyboard navigation:

⌨️ Keyboard Shortcuts

* **Arrows (↑ ↓ ← →)**: Navigate the sidebar queue.
* **Tab / Shift-Tab**: Cycle focus between Delete Folder, Save Rule, and PROCESS FOLDER.
* **Enter**: Trigger the focused button (indicated by ► ◄ arrows).


**UI Controls:** You can adjust the application's appearance using the buttons in the top right:

* **☀ / ☾:** Toggle between Light and Dark mode.  
* **?:** Open the formatted Markdown Help Menu.  
* **A+ / A-:** Increase or decrease the application font size.  
* **Reset View:** Snap the window back to its default size (1120x950) and default font size (11).

 
### **1\. SETTINGS WORKSPACE (VM Synchronization)**

In the sidebar, choose your workspace mode:
* Personal (Local): Keeps rules and logs on the current machine (in permit_configs).
* Team Shared: Points the app to a shared drive (like a Project drive). This ensures that if you set a rule in VM-A, it is instantly available in VM-B.

 
### **2\. PENDING QUEUE**

The sidebar shows all detected transfer- folders:
* **✓**: Folder is valid and ready to process.
* **✓ (L)**: Folder is valid, is ready to process, but was processed already and a Log is available.
* **✗**: Folder appears empty or invalid.

Use Up/Down or Left/Right arrows to cycle through the queue.

### **3\. DESTINATIONS**

The main area shows a combined view of all files in the source folder followed by the raw content of the receipt.json. Warnings appear in bold red if the receipt contains formatting errors (like trailing commas). 


### **4\. PROCESSING RULES**

* **Config ID**: Rules are saved based on the permitId found in the receipt.
* **Pattern Match**: Route files based on name (e.g., Archive*.*) if no receipt is present.
* **Target**: Where data lands.
* **Processed**: Where the original folder goes (if "Move" is selected).
* **Receipt**: Where the audit-trailed receipt.json lands.


### **5\. FILE INSPECTOR**

The main area shows a combined view of all files in the source folder followed by the raw content of the receipt.json. Warnings appear in bold red if the receipt contains formatting errors (like trailing commas).

* **Manageg Rules**: Here the pattern matching rules and config rules can be managed. See ? in the application for examples

### **6\. CREATE <RECEIPT.JSON> FILES**

Easy create your own custom receipt.json file based on your settings that work both on Windows and Linux VMs. Just add the receipt*.json to your uploads!

### **7\. Advanced Logging & Overrides**

* **Auto-Extract Checkbox:** By default, ZIP files are extracted. If you simply want to move/copy the .zip archive itself into your Target Folder, uncheck the "Auto-Extract ZIP files" box.  
* **Global Audit Logging:** Every processed file and resolved conflict is written to a machine-readable JSONL file (process\_log.jsonl). Click **'📄 View Log'** to inspect it.  
* **Local Process Logs:** When a folder is processed successfully (and not deleted), an Inbox Process.log file is dropped directly into the folder (or its new location in the Processed Folder).  
  * If this file exists, the GUI will dynamically display a "Latest:![][image1]  
    SUCCESS" label.  
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
* \--no-auto-extract: Disable automatic zip extraction and instead copy the raw zip file.

[image1]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAmwAAAAuCAYAAACVmkVrAAADQklEQVR4Xu3cXYjmUxwH8NlaeYlsMqWZZ57/PGMyW0pqSEqK3KwLEXLDtUQuhaKsXFAopbjxkrj0kmxeyktYt0Q2W17aDTcuvJRt3KzvWefsnv5py8Vqpv186tf5nd85//9z5u7Xc55mbg4AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD+o8lksjgMw6HEwel0eiDj4cRXifdqfn/bm/zn/tkTZXl5+bJxDQDgpJUmbM/8/PyZNT/SpHVrL6eJu/DY7hMvn3dFGrbrx3UAgJNWGqRnWp4GbSPxZTd/P8P2Nv8/pFm7L7FjXAcAYO5Ig1auQG/8l/qrdXyljq+l0Xs+42+TyeT0WjuYRuu8mh9eWVm5oOSpnZb5J9l3Ton2ztR3DvWKdajXrgsLC2ck/6vtqWs3JK6t+aFWz7tW6xmeKmfI+HRiTzlD6reWM9St21ZXV0/N/Ius3T6bzYZuDQBg60iDdXZprMb1Jk3Qg+3qtChNTxqge7v117u178qYRmk++Ub2PZR4NPnV3f5ryjsS+8paqZV9Q9ewJX+sb676fGlpaWG09k3L8+4X2hnq2lVtb8bL++cAALaMNDk3pZHZPa5X27P2bV8oTU+apktLnvH8zO8q+dra2lnJHy95+T3a8ZqjrO1KfNr2ZP9HyT/o1vcm/ujmGy3P3tv6d4/y39sZitJYllrJ89w9xzsTAMCmlaZmf7m+HNeLNDhPJPaVsdWy98Vu/aWWt9/EZf2NDNuy9tlc/R1c8gfavvau2Wx25VC/2SuNVJ6/OfFOuVLNfHfi87p2Z975bIk6/3F0hqPfsCX/voztWjbzXxNf1/yn0mC2vQAAm15peoZj/8Ljl6H7hqvbs7M0UWl0rivzen169Hpz+Kcpa3svLtejqe0q8zRkFyX/IfV3E7d0z7yVeDPxcHlfrT2XZw/kcy4p8/KbttKgpfZ2xjuy/mdq59bP2d+fIXvubnn5rHKGNq9/2yOJjxcXFyetDgDAJpFG7cNxDQCATWJ9ff2U6XT65LgOAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAsFX8DV7TuYPnroNBAAAAAElFTkSuQmCC>
