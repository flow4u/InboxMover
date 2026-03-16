# **Inbox Mover \- Comprehensive Test Plan (v0.9.5)**

## **1\. Introduction**

**Objective:** To verify that Inbox Mover v0.9.5 successfully processes transfer-\* folders, extracts ZIP archives, handles receipt.json updates, dynamically generates local logs, manages the modern UI, resolves file conflicts, and reliably supports keyboard navigation.

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
| UI-01 | Launch inbox\_mover.py | App opens with default size (1100x950), Dark Mode enabled. Version reads 0.9.5. Search Folders default to i:/ and z:/inbox. Modern "Card" layout is visible. |  |
| UI-02 | Click "☀" (Light Mode) | UI switches to a light color palette. Icon changes to "☾". Text and highlighted text areas remain readable. |  |
| UI-03 | Click "A+" and "A-" buttons | Font size of all labels, buttons, and text areas dynamically scales up and down. |  |
| UI-04 | Click "Reset View" | Window snaps back to exactly 1100x950 pixels and base font size resets to 11\. |  |
| UI-05 | Click "?" (Help) | Help modal opens. Text is properly formatted in Markdown, and the "Keyboard Support" section is visible near the top. |  |
| UI-06 | Press Tab, Shift-Tab, and Enter | Focus alternates visibly between "PROCESS FOLDER" and "Save Config" (with ► ◄ arrows). Pressing Enter triggers the active button. Other UI elements are correctly ignored by the Tab key. |  |
| DIR-01 | Select C:\\Test\\Search1 & Search2, click Refresh. | App auto-scans both folders. Navigation shows \[ 1 / 6 \] found folders. |  |
| NAV-01 | Cycle folders using "Next ⇨" / "⇦ Prev" or Left/Right Keys. | Card Title updates. Text area updates. Form fields clear and reload correctly to prevent state leakage. |  |

## **4\. Core Processing & Post-Actions**

| Test ID | Action | Expected Result | Status |
| :---- | :---- | :---- | :---- |
| PROC-01 | transfer-01-standard: Set Target & Receipt folders. Action: Overwrite. Post: Leave. Click "PROCESS FOLDER". | Success message. test.txt is in Target. receipt.json is in Receipt folder (timestamped). |  |
| PROC-02 | transfer-02-no-receipt: Set Target folder. Click "PROCESS FOLDER". | Success message. Extracts image.png to Target. Loose file notes.txt copied to Target. |  |
| PROC-03 | transfer-06-absolute: Set Target folder to C:\\Test\\Target. Click "PROCESS FOLDER". | absolute\_file.txt ignores Target Folder and is extracted to C:\\Test\\Absolute\\absolute\_file.txt. Missing folders auto-created. |  |
| CRE-01 | Process a valid transfer but type non-existent Target/Processed/Receipt paths manually. | The application successfully creates all missing directories automatically and moves/extracts files into them without crashing. |  |
| CON-01 | transfer-05-conflict: Set Action to "Rename existing file". Click "PROCESS". | Target folder contains the new conflict.txt AND the old file renamed with a timestamp (YYMMDD-HHMMSS\_conflict.txt). |  |

## **5\. Local Logging & Receipt Injection (v0.9.5 Features)**

| Test ID | Action | Expected Result | Status |
| :---- | :---- | :---- | :---- |
| LOG-01 | Navigate back to transfer-01-standard (which was processed in PROC-01). | UI dynamically shows a blue italic label: "Latest: ![][image1]SUCCESS | User: ![][image2]". A new "📄 Process Log" button appears. |  |
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
| CLI-01 | Open terminal. Run: python inbox\_mover.py \--cli \--help | Displays help menu with all available arguments, confirming v0.9.5. |  |
| CLI-02 | Run: python inbox\_mover.py \--cli \-s "C:\\Test\\Search1" \-t "C:\\Test\\Target" | Scans Search1. Processes folders headlessly. Console output shows folder names, Config IDs, and respects receipt overrides without GUI interaction. |  |

[image1]: <data:image/png;base64,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>

[image2]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAdMAAABBCAYAAACKJ05RAAADVElEQVR4Xu3aTahnYxwH8BkhRISr6b6d/30xV7eUuqlJLNgMdiRRLNiJZKNGYWFhQUpqZJoyo0l5y8vGa92FGJmpcWsUYzGKZDkhIzbj+5jz6Ok0C9O/9J/6fOrXeZ7f7znnLr/dc/6bNgEAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAADAGafruvXUidFodGA4AwD+gwTpJ8MeAHAahCkAjEmYAsCYhCkAjEmYAsCYhCkAjGkYptnfmDqY2j87O3v1aDTanvVbuX5arqXXni8y25Z6PfPPUx9kfXdZ1/nMzMxs9rtTh+fn5++cm5tbyvqNcjb1ber2nLks9z2f9Zv9cx5t/wYATKyuCdO1tbVzsn+7vx5N8P2Q60MZbe7PHk69+u/NJ3u3pf5K3Vr2uWc16z9KUDZn3kkIL2e2J+tfUi9kf34/25n6MfV+wnRL/4x7sj+xuLi4tT4DACZW14Rpwuzm1CPLy8tTJcwSai8Nzn7VNWGa/zCnsz+We16pvdXV1XPT+z31eNlPTU1dmOd82N9/KLWR8+fV832YlvMLtZf5XcIUgDNGG6ZVwu+OEmap62ovATfqe/fXXtbPlF5G22svAXtDf+6m2uv7JXjL2R1tP70j+Xvvtr3s96Z/tO0BwMQ6VZj23y6PZXl203ssveO5XlJ72e9P/VZeC9degvDJ9P6sr3Gb/r0lTBOq19Ze/+20BG95lfyPlZWVi7L/NfVc7QHARDtVmKa3kfB7rWltTu+b1L5+/mLC8tI+TNebcyU0v6y9XF/O5ay+X/7b/LnuiwTzfSVM86wrh71ct+Weta5/XQwAE2sYpvV1bNe8zk2wbSm98sOgrK8qwVj66T2V2qjnsn6wD8JdCwsLK+XazH4aNd9W+96+1JG2V77Tpvd9Wef8e76bAjDxhmGaMLs+va+XlpauGPSfTv+z1LP1Ve/09PQFJTAz+6j8yCjrBzJ/InU8tTPzy8u5BOLFmX3XDb6jZv9F7nu47eXcNamPy3fUzG9pZwAwkYZhCgCcJmEKAGMSpgAwJmEKAGNKmK53J3+Be2A4AwAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAgP/Z3xYexYDXgHyAAAAAAElFTkSuQmCC>