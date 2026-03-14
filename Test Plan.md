# **Inbox Mover \- Comprehensive Test Plan**

## **1\. Introduction**

**Objective:** To verify that Inbox Mover successfully processes transfer-\* folders, extracts ZIP archives, handles receipt.json correctly, resolves file conflicts, manages post-processing actions, and respects saved configurations in both GUI and CLI modes.

**Scope:** UI functionality, core extraction logic, folder scanning, configuration management, conflict resolution, post-processing, and CLI execution.

## **2\. Test Environment Setup**

Before beginning the tests, set up the following directory structure and dummy files on your local machine:

**Directories:**

* C:\\Test\\Search1  
* C:\\Test\\Search2  
* C:\\Test\\Target  
* C:\\Test\\Processed  
* C:\\Test\\Receipts

**Dummy Data (Inside C:\\Test\\Search1):**

1. Create folder: transfer-01-standard.  
   * Inside, add a ZIP file (data.zip).  
   * Inside the ZIP, include a test.txt file and a receipt.json file containing: {"permitId": "PERMIT-A"}.  
2. Create folder: transfer-02-no-receipt.  
   * Inside, add a ZIP file containing image.png (NO receipt.json).  
   * Add a loose text file outside the zip: notes.txt.  
3. Create folder: transfer-03-empty.  
   * Leave this folder completely empty.

**Dummy Data (Inside C:\\Test\\Search2):**

1. Create folder: transfer-04-conflict.  
   * Inside, add a ZIP file containing conflict.txt and receipt.json with {"permitId": "PERMIT-B"}.  
   * *Pre-condition:* Copy conflict.txt into C:\\Test\\Target before testing.

## **3\. Graphical User Interface (GUI) Tests**

### **3.1. Launch & Basic UI**

| Test ID | Action | Expected Result | Status |
| :---- | :---- | :---- | :---- |
| UI-01 | Launch inbox\_mover.py | App opens with default size (800x650), Dark Mode enabled. |  |
| UI-02 | Click "Toggle Light Mode" | UI switches to a light color palette. Text remains readable. |  |
| UI-03 | Click "A+" and "A-" buttons | Font size of all labels, buttons, and text areas increases/decreases. |  |
| UI-04 | Click "? Help" | Help window opens displaying instructions matching the README. |  |

### **3.2. Directory Navigation**

| Test ID | Action | Expected Result | Status |
| :---- | :---- | :---- | :---- |
| DIR-01 | Click "Browse..." next to Search Folder 1 and select C:\\Test\\Search1. | Path populates the field. App auto-scans and updates the UI to show transfer-01-standard. |  |
| DIR-02 | Click "Open" next to Search Folder 1\. | System File Explorer opens to C:\\Test\\Search1. |  |
| DIR-03 | Fill in Target, Processed, and Receipt folder paths. | Paths are accepted in the UI. |  |
| DIR-04 | Enter C:\\Test\\Search2 in Search Folder 2 and click "↻ Refresh". | App scans both Search 1 and 2\. Nav indicator updates to show total found folders. |  |

### **3.3. Folder Browsing**

| Test ID | Action | Expected Result | Status |
| :---- | :---- | :---- | :---- |
| NAV-01 | Use "Next ⇨" and "⇦ Prev" to cycle folders. | UI updates to show current folder name, Config ID, and Source Folder Content. |  |
| NAV-02 | Navigate to transfer-01-standard. | Config ID shows "PERMIT-A". receipt.json contents are visible in the text area. "PROCESS" button is enabled. |  |
| NAV-03 | Navigate to transfer-02-no-receipt. | Config ID shows "DEFAULT". Text area lists files in the folder and notes NO receipt found. "PROCESS" button is enabled. |  |
| NAV-04 | Navigate to transfer-03-empty. | Config ID shows "DEFAULT". Text area says "\<Folder is empty\>". "PROCESS" button is disabled. |  |

## **4\. Core Processing & Logic Tests**

### **4.1. Standard Extraction**

| Test ID | Action | Expected Result | Status |
| :---- | :---- | :---- | :---- |
| PROC-01 | Navigate to transfer-01-standard. Set Target & Receipt folders. Action: Overwrite. Post: Leave. Click "PROCESS". | Success message. test.txt is in Target folder. receipt.json is in Receipt folder, renamed with a timestamp (e.g., YYMMDD-HHMMSS-receipt.json). transfer-01 remains in Search1. |  |

### **4.2. "No Receipt" Fallback Mode**

| Test ID | Action | Expected Result | Status |
| :---- | :---- | :---- | :---- |
| PROC-02 | Navigate to transfer-02-no-receipt. Set Target folder. Click "PROCESS". | Success message. Zip extracts image.png to Target. Loose file notes.txt is copied to Target. |  |

### **4.3. Conflict Resolution**

| Test ID | Action | Expected Result | Status |
| :---- | :---- | :---- | :---- |
| CON-01 | Pre-condition: conflict.txt exists in Target. Navigate to transfer-04-conflict. Set Action to "Overwrite". Click "PROCESS". | Original conflict.txt in Target is replaced by the new file from the zip. |  |
| CON-02 | Restore pre-condition. Set Action to "Keep both (add number)". Click "PROCESS". | Target folder contains both conflict.txt and conflict (1).txt. |  |
| CON-03 | Restore pre-condition. Set Action to "Rename existing file". Click "PROCESS". | Target folder contains the new conflict.txt AND the old file renamed with a timestamp (e.g., YYMMDD-HHMMSS\_conflict.txt). |  |

### **4.4. Post-Processing Actions**

| Test ID | Action | Expected Result | Status |
| :---- | :---- | :---- | :---- |
| POST-01 | Process a valid transfer folder with Post Action: "Delete the files". | The entire transfer-xxx folder is completely removed from the Search folder. |  |
| POST-02 | Process a valid transfer folder with Post Action: "Move the files to Processed Folder". | The entire transfer-xxx folder is moved to C:\\Test\\Processed. |  |
| POST-03 | Process a folder with "Move". Place a folder with the *exact same name* in the Processed folder, and process again. | The second folder is moved and renamed by appending \_1 (e.g., transfer-xxx\_1) to prevent overwriting. |  |

## **5\. Configuration Saving & Loading Tests**

| Test ID | Action | Expected Result | Status |
| :---- | :---- | :---- | :---- |
| CFG-01 | Navigate to transfer-01-standard (PERMIT-A). Set unique target/post-action settings. Click "Save Config". | Success message. A file PERMIT-A.json is created in the permit\_configs folder. |  |
| CFG-02 | Restart the application. | Search folders are pre-filled from last session. App auto-scans. |  |
| CFG-03 | Navigate to transfer-01-standard. | The unique target/post-action settings saved in CFG-01 are automatically loaded into the UI. |  |

## **6\. Command-Line Interface (CLI) Tests**

*Open a terminal/command prompt and navigate to the folder containing inbox\_mover.py.*

| Test ID | Command / Action | Expected Result | Status |
| :---- | :---- | :---- | :---- |
| CLI-01 | python inbox\_mover.py \--cli \--help | Displays help menu with all available arguments. |  |
| CLI-02 | python inbox\_mover.py \--cli \-s "C:\\Test\\Search1" \-t "C:\\Test\\Target" | Scans Search1. Processes folders. Output shows folder names, Config IDs, and success/skip statuses. |  |
| CLI-03 | Create transfer-05 with PERMIT-C. Save config in GUI to Move to C:\\Test\\Processed. Run: python inbox\_mover.py \--cli \-s "C:\\Test\\Search1" \-t "C:\\Test\\Target" | The CLI outputs that it loaded saved configuration for PERMIT-C. Folder is moved to Processed, ignoring the \-t default argument. |  |

