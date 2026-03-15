# **Inbox Mover \- Comprehensive Test Plan (v0.9)**

## **1\. Introduction**

**Objective:** To verify that Inbox Mover v0.9 successfully processes transfer-\* folders, extracts ZIP archives, handles receipt.json updates, dynamically generates local logs, manages the new modern UI, and resolves file conflicts.

**Scope:** UI functionality, core extraction logic, folder scanning, configuration management, absolute path logic, dynamic overrides, local/global logging, receipt injection, and CLI execution.

## **2\. Test Environment Setup**

Before beginning the tests, set up the following directory structure and dummy files on your local machine:

**Directories:**

* C:\\Test\\Search1  
* C:\\Test\\Search2  
* C:\\Test\\Target  
* C:\\Test\\Processed  
* C:\\Test\\Receipts  
* C:\\Test\\Absolute

**Dummy Data (Inside C:\\Test\\Search1):**

1. transfer-01-standard: Add a ZIP file containing test.txt and receipt.json ({"permitId": "PERMIT-A"}).  
2. transfer-02-no-receipt: Add a ZIP file containing image.png (NO receipt.json) and a loose text file outside the zip (notes.txt).  
3. transfer-03-empty: Leave completely empty.  
4. transfer-04-overrides: Add a ZIP file containing receipt.json with overrides: {"permitId": "PERMIT-OVR", "target\_folder": "C:\\\\Test\\\\Target\\\\Overridden", "post\_processing": "move"}.

**Dummy Data (Inside C:\\Test\\Search2):**

1. transfer-05-conflict: Add a ZIP file containing conflict.txt and receipt.json ({"permitId": "PERMIT-B"}). Copy conflict.txt into C:\\Test\\Target beforehand.  
2. transfer-06-absolute: Add a ZIP file with an absolute internal path (C:\\Test\\Absolute\\absolute\_file.txt) and a receipt.json.

## **3\. Graphical User Interface (GUI) Tests**

| Test ID | Action | Expected Result | Status |
| :---- | :---- | :---- | :---- |
| UI-01 | Launch inbox\_mover.py | App opens with default size (1100x950), Dark Mode enabled. Version reads 0.9. Search Folders default to i:/ and z:/inbox. Modern "Card" layout is visible. |  |
| UI-02 | Click "☀" (Light Mode) | UI switches to a light color palette. Icon changes to "☾". Text and highlighted text areas remain readable. |  |
| UI-03 | Click "A+" and "A-" buttons | Font size of all labels, buttons, and text areas dynamically scales up and down. |  |
| UI-04 | Click "Reset View" | Window snaps back to exactly 1100x950 pixels and base font size resets to 11\. |  |
| UI-05 | Click "?" (Help) | Help modal opens. Text is properly formatted in Markdown (bolding, headers, and custom backgrounds for code snippets). |  |
| DIR-01 | Select C:\\Test\\Search1 & Search2, click Refresh. | App auto-scans both folders. Navigation shows \[ 1 / 6 \] found folders. |  |
| NAV-01 | Cycle folders using "Next ⇨" / "⇦ Prev". | Card Title updates. Text area updates. Form fields clear and reload correctly to prevent state leakage. |  |

## **4\. Core Processing & Post-Actions**

| Test ID | Action | Expected Result | Status |
| :---- | :---- | :---- | :---- |
| PROC-01 | transfer-01-standard: Set Target & Receipt folders. Action: Overwrite. Post: Leave. Click "PROCESS FOLDER". | Success message. test.txt is in Target. receipt.json is in Receipt folder (timestamped). |  |
| PROC-02 | transfer-02-no-receipt: Set Target folder. Click "PROCESS FOLDER". | Success message. Extracts image.png to Target. Loose file notes.txt copied to Target. |  |
| PROC-03 | transfer-06-absolute: Set Target folder to C:\\Test\\Target. Click "PROCESS FOLDER". | absolute\_file.txt ignores Target Folder and is extracted to C:\\Test\\Absolute\\absolute\_file.txt. Missing folders auto-created. |  |
| CRE-01 | Process a valid transfer but type non-existent Target/Processed/Receipt paths manually. | The application successfully creates all missing directories automatically and moves/extracts files into them without crashing. |  |
| CON-01 | transfer-05-conflict: Set Action to "Rename existing file". Click "PROCESS". | Target folder contains the new conflict.txt AND the old file renamed with a timestamp (YYMMDD-HHMMSS\_conflict.txt). |  |

## **5\. Local Logging & Receipt Injection (v0.9 Features)**

| Test ID | Action | Expected Result | Status |
| :---- | :---- | :---- | :---- |
| LOG-01 | Navigate back to transfer-01-standard (which was processed in PROC-01). | UI dynamically shows a blue italic label: "Latest: \[Timestamp\] SUCCESS | User: \[name\]". A new "📄 Process Log" button appears. |
| LOG-02 | Click "📄 Process Log". | A modal opens displaying the human-readable Inbox Process.log file from that folder, with color-coded success and action lines. |  |
| LOG-03 | Check the Target Receipt folder from PROC-01. Open the extracted timestamped receipt.json in a text editor. | The JSON structure is valid. At the bottom, a new "processing\_logs" array has been safely injected containing the extraction details. |  |
| LOG-04 | Process transfer-04-overrides with Post Action: "Move". | The folder moves to Processed Folder. Inbox Process.log is generated successfully inside its *new* location in the Processed Folder. |  |

## **6\. Configuration Saving & Overrides**

| Test ID | Action | Expected Result | Status |
| :---- | :---- | :---- | :---- |
| CFG-01 | Navigate to transfer-01-standard (PERMIT-A). Set unique target/post-action settings. Click "Save Config". | Success message. PERMIT-A.json is created in permit\_configs. Button returns to standard state. |  |
| CFG-02 | Modify Target Folder manually in UI. | "Save Config" button immediately turns Orange (Save Config \*) indicating unsaved changes. |  |
| CFG-03 | Navigate to transfer-03-empty. | Config ID is "DEFAULT". Form correctly loads DEFAULT config baseline (or remains blank if default is empty). "PROCESS" button is disabled because folder is empty. |  |
| CFG-04 | Navigate to transfer-04-overrides. | UI automatically updates to match the receipt keys (target\_folder and post\_processing are changed). Button is Orange indicating receipt overrides. |  |

## **7\. Global Audit Logging & CLI**

| Test ID | Action / Command | Expected Result | Status |
| :---- | :---- | :---- | :---- |
| GLB-01 | Click "📄 View Log" in the top right. | A text editor modal opens process\_log.jsonl. Shows all previous successful extractions, capturing detailed paths and conflict resolutions. |  |
| GLB-02 | Click "🗑 Clear Log" then "📄 View Log". | A prompt says "Log file is empty". The .jsonl file content was cleared. |  |
| CLI-01 | Open terminal. Run: python inbox\_mover.py \--cli \--help | Displays help menu with all available arguments, confirming v0.9. |  |
| CLI-02 | Run: python inbox\_mover.py \--cli \-s "C:\\Test\\Search1" \-t "C:\\Test\\Target" | Scans Search1. Processes folders headlessly. Console output shows folder names, Config IDs, and respects receipt overrides without GUI interaction. |  |

