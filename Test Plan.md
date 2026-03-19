# **Inbox Mover \- Comprehensive Test Plan (v0.10.1)**

## **1\. Introduction**

**Objective:** To verify that Inbox Mover v0.10.1 successfully processes transfer-\* folders, extracts ZIP archives, handles receipt.json updates (both inside zips and loose files), dynamically generates local logs, manages the modern UI, resolves file conflicts, handles pattern matching, and reliably supports keyboard navigation.

**Scope:** UI functionality, core extraction logic, folder scanning, configuration management, absolute path logic, dynamic overrides, local/global logging, receipt injection, pattern matching heuristics, and CLI execution.

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
2. transfer-02-no-receipt: Add a ZIP file containing image.png (NO receipt.json) and a loose text file outside the zip (backup\_db.sql).  
3. transfer-03-empty: Leave completely empty.  
4. transfer-04-overrides: Add a ZIP file containing receipt.json with overrides: {"permitId": "PERMIT-OVR", "target\_folder": "C:\\\\Test\\\\Target\\\\Overridden", "post\_processing": "move"}.  
5. transfer-07-loose-receipt: Add a loose raw text file (data.csv) and a loose receipt.json ({"permitId": "PERMIT-LOOSE"}). **Do not add a ZIP file.**

**Dummy Data (Inside C:\\Test\\Search2):**

1. transfer-05-conflict: Add a ZIP file containing conflict.txt and receipt.json ({"permitId": "PERMIT-B"}). Copy conflict.txt into C:\\Test\\Target beforehand.  
2. transfer-06-absolute: Add a ZIP file with an absolute internal path (C:\\Test\\Absolute\\absolute\_file.txt) and a receipt.json.

## **3\. Graphical User Interface (GUI) Tests**

| Test ID | Action | Expected Result | Status |
| :---- | :---- | :---- | :---- |
| UI-01 | Launch inbox\_mover.py | App opens with default size (1120x950), Dark Mode enabled. Version reads 0.10.1. Modern "Card" layout is visible. |  |
| UI-02 | Click "☀" (Light Mode) | UI switches to a light color palette. Icon changes to "☾". |  |
| UI-03 | Click "A+" and "A-" buttons | Font size dynamically scales up and down. |  |
| UI-04 | Click "Reset View" | Window snaps back to exactly 1120x950 pixels and base font size resets to 11\. |  |
| UI-05 | Click "?" (Help) | Help modal opens formatted in Markdown. Keyboard support and Auto-Match Pattern sections are visible. |  |
| UI-06 | Press Tab, Shift-Tab, and Enter | Focus alternates visibly between "PROCESS FOLDER" and "Save Config" (with ► ◄ arrows). Enter triggers active button. |  |
| DIR-01 | Select C:\\Test\\Search1 & Search2, click Refresh. | App auto-scans both folders. Navigation shows \[ 1 / 7 \] found folders. |  |

## **4\. Core Processing & Post-Actions**

| Test ID | Action | Expected Result | Status |
| :---- | :---- | :---- | :---- |
| PROC-01 | transfer-01-standard: Set Target & Receipt folders. Action: Overwrite. Post: Leave. Click "PROCESS FOLDER". | Success message. test.txt is in Target. receipt.json is in Receipt folder. |  |
| PROC-02 | transfer-02-no-receipt: Set Target folder. Click "PROCESS FOLDER". | Success message. Extracts image.png to Target. Loose file backup\_db.sql copied to Target simultaneously. |  |
| PROC-03 | transfer-06-absolute: Set Target folder to C:\\Test\\Target. Click "PROCESS FOLDER". | absolute\_file.txt ignores Target Folder and is extracted to C:\\Test\\Absolute\\absolute\_file.txt. |  |
| PROC-04 | transfer-07-loose-receipt: Set Target/Receipt folders. Click "PROCESS FOLDER". | UI reads Config ID from loose receipt correctly. data.csv is copied. Loose receipt is timestamped, moved, and updated with logs safely. |  |
| CRE-01 | Process a valid transfer but type non-existent Target paths manually. | The application successfully creates all missing directories automatically. |  |
| CON-01 | transfer-05-conflict: Set Action to "Rename existing file". Click "PROCESS". | Target folder contains new conflict.txt AND the old file renamed with a timestamp. |  |

## **5\. Local Logging & Receipt Injection**

| Test ID | Action | Expected Result | Status |
| :---- | :---- | :---- | :---- |
| LOG-01 | Navigate back to transfer-01-standard. | UI dynamically shows a blue italic label: "Latest: \[Timestamp\] SUCCESS | User: \[name\]". |
| LOG-02 | Click "📄 Process Log". | Modal opens displaying the human-readable Inbox Process.log. |  |
| LOG-03 | Check the Target Receipt folder from PROC-01. | JSON structure valid. New "processing\_logs" array safely injected. |  |

## **6\. Configuration Saving & Overrides**

| Test ID | Action | Expected Result | Status |
| :---- | :---- | :---- | :---- |
| CFG-01 | Navigate to transfer-01-standard (PERMIT-A). Set unique settings. Click "Save Config". | Success message. PERMIT-A.json is created. |  |
| CFG-02 | Modify Target Folder manually in UI. | "Save Config" button immediately turns Orange (Save Config \*). |  |
| CFG-03 | Navigate to transfer-03-empty. | Config ID is "DEFAULT". "PROCESS" button is disabled because folder is empty. |  |
| CFG-04 | Navigate to transfer-04-overrides. | UI automatically updates to match the receipt keys. Button is Orange indicating receipt overrides. |  |
| CFG-05 | Click "⚙ Manage" next to Config ID. | Modal opens. You can select, edit, and save updates to PERMIT-A, or delete it. |  |

## **7\. Auto-Match Pattern Heuristics**

| Test ID | Action | Expected Result | Status |
| :---- | :---- | :---- | :---- |
| PAT-01 | Navigate to transfer-02-no-receipt (which contains backup\_db.sql). Enter backup\*.sql in the "Auto-Match Pattern" field. Change Target Folder to C:\\Test\\Backups. Click "Save Config". | Success prompt confirms saved for pattern 'backup\*.sql'. Pattern saved in patterns.json. |  |
| PAT-02 | Refresh the application and navigate back to transfer-02-no-receipt. | The app detects backup\_db.sql matches the saved pattern. Target Folder automatically populates with C:\\Test\\Backups. Auto-Match Pattern field populates with backup\*.sql. |  |
| PAT-03 | Navigate to transfer-01-standard (PERMIT-A). | Ensure Target Folder loads PERMIT-A config, ignoring any pattern logic because Config ID takes precedence. |  |
| PAT-04 | While viewing a pattern (e.g., backup\*.sql), click the "🗑 Delete" button next to the input field. | Success message. Pattern clears, settings fall back to DEFAULT. Button becomes disabled. |  |
| PAT-05 | Click "⚙ Manage" next to the Pattern field. | Pattern manager modal opens. Patterns can be created, edited, and deleted. |  |

## **8\. Global Audit Logging & CLI**

| Test ID | Action / Command | Expected Result | Status |
| :---- | :---- | :---- | :---- |
| GLB-01 | Click "📄 View Log" in the top right. | A text editor modal opens process\_log.jsonl. |  |
| GLB-02 | Click "🗑 Clear Log" then "📄 View Log". | A prompt says "Log file is empty". |  |
| CLI-01 | Open terminal. Run: python inbox\_mover.py \--cli \--help | Displays help menu with all available arguments, confirming v0.10.1. |  |
| CLI-02 | Run CLI process command. | Scans folder and processes headlessly. |  |

