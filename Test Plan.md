# **Inbox Mover \- Comprehensive Test Plan (v0.10)**

## **1\. Introduction**

**Objective:** To verify that Inbox Mover v0.10 successfully processes transfer-\* folders, extracts ZIP archives, handles receipt.json updates, dynamically generates local logs, manages the modern UI, resolves file conflicts, handles pattern matching, and reliably supports keyboard navigation.

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

**Dummy Data (Inside C:\\Test\\Search2):**

1. transfer-05-conflict: Add a ZIP file containing conflict.txt and receipt.json ({"permitId": "PERMIT-B"}). Copy conflict.txt into C:\\Test\\Target beforehand.  
2. transfer-06-absolute: Add a ZIP file with an absolute internal path (C:\\Test\\Absolute\\absolute\_file.txt) and a receipt.json.

## **3\. Graphical User Interface (GUI) Tests**

| Test ID | Action | Expected Result | Status |
| :---- | :---- | :---- | :---- |
| UI-01 | Launch inbox\_mover.py | App opens with default size (1120x950), Dark Mode enabled. Version reads 0.10. Modern "Card" layout is visible. |  |
| UI-02 | Click "☀" (Light Mode) | UI switches to a light color palette. Icon changes to "☾". |  |
| UI-03 | Click "A+" and "A-" buttons | Font size dynamically scales up and down. |  |
| UI-04 | Click "Reset View" | Window snaps back to exactly 1120x950 pixels and base font size resets to 11\. |  |
| UI-05 | Click "?" (Help) | Help modal opens formatted in Markdown. Keyboard support and Auto-Match Pattern sections are visible. |  |
| UI-06 | Press Tab, Shift-Tab, and Enter | Focus alternates visibly between "PROCESS FOLDER" and "Save Config" (with ► ◄ arrows). Enter triggers active button. |  |
| DIR-01 | Select C:\\Test\\Search1 & Search2, click Refresh. | App auto-scans both folders. Navigation shows \[ 1 / 6 \] found folders. |  |

## **4\. Core Processing & Post-Actions**

| Test ID | Action | Expected Result | Status |
| :---- | :---- | :---- | :---- |
| PROC-01 | transfer-01-standard: Set Target & Receipt folders. Action: Overwrite. Post: Leave. Click "PROCESS FOLDER". | Success message. test.txt is in Target. receipt.json is in Receipt folder. |  |
| PROC-02 | transfer-02-no-receipt: Set Target folder. Click "PROCESS FOLDER". | Success message. Extracts image.png to Target. Loose file backup\_db.sql copied to Target. |  |
| PROC-03 | transfer-06-absolute: Set Target folder to C:\\Test\\Target. Click "PROCESS FOLDER". | absolute\_file.txt ignores Target Folder and is extracted to C:\\Test\\Absolute\\absolute\_file.txt. |  |
| CRE-01 | Process a valid transfer but type non-existent Target paths manually. | The application successfully creates all missing directories automatically. |  |
| CON-01 | transfer-05-conflict: Set Action to "Rename existing file". Click "PROCESS". | Target folder contains new conflict.txt AND the old file renamed with a timestamp. |  |

## **5\. Local Logging & Receipt Injection**

| Test ID | Action | Expected Result | Status |
| :---- | :---- | :---- | :---- |
| LOG-01 | Navigate back to transfer-01-standard. | UI dynamically shows a blue italic label: "Latest: ![][image1]SUCCESS | User: ![][image2]". |  |
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
| CLI-01 | Open terminal. Run: python inbox\_mover.py \--cli \--help | Displays help menu with all available arguments, confirming v0.10. |  |
| CLI-02 | Run CLI process command. | Scans folder and processes headlessly. |  |

[image1]: <data:image/png;base64,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>

[image2]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAfQAAABBCAYAAAAnplb4AAADaUlEQVR4Xu3bPYhcVRgG4E3AXxSDuIizP7Ozu7pmRZtVC7XSMuKiRhDUwmgTAlYGFUHQwr9CLKKFCMbYKIEgRiUggmiRUsGgRNDCylIUtbDQ98veA5dhEmIlOzwPHO497z33TPneO8zMzAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAADAf7a4uHjXcDj8p8bS0tLm+HUAYBuoQk+RPzOeAwDbiEIHgCmg0AFgCih0AJgCCh0ApoBCB4ApMF7o6+vrFyY7PBwOv8vxpdnZ2cty/lbGsYyvurU7elvMDAaDSyuv6xknc98TOR7PuLetyfmBjM8yjmbtrqx5KscPMj+V8fHGxsYFye7O+XvdPkczv6H/OQDAWYwXeor08Yz7MvYNt/6f/unCwsKgrmXdg5XlnvW2PtnFGV/WurW1tcsT7egeCGrdHbUmx42Md7LPSrdnlfht3edd02Uns8/+ur/LT2ccaZ8DAJzDhEI/1pX06zn/s0q4t3bveKFn/mplo9ForZc9m/HX6urqRTXPXq/kntuT3d+V90O9ta3QD7Wsy78dKnQAOD/jhd6kTL/O+GQsezvjpzYfDAZXVXHXG3p/XfY8kfzzflaSvZHxe3293rKsfaQKPQ8Ot7Rsfn5+tSv5fS0DAM5hUqHnzXq2K9QnW5aSvSTzX7P+xZZlvqfW9e+vt/tkf+T4fMua5Kdz/4f9rL6KT/5LTnf2sueS/V0PDL2lAMDZTCr0ZA9UUafEb+xlZ96ks/b65eXl63L+Qiv02qOty5v2PS0bjUa35vxA5b237jPzJvMfs+e7vWhn5j8kP16T7HM4n3dF7zoAMG5SoadM3xz2vlrvskMZ39d5lWzOd+e+XTn+lvneylPgV2f+TRX3ysrKQo5HMkZ1LWv3V55iv7btWQ8HleX+h1s2Nzc3X1nGY/VAMFb2AMAkkwq9vhZPoR7sZynXm4Zbfzurv6/taXnO76w895zIeD9v6Dfn+HOyL7Lvo21dzp/O+KjNu2wz606l5K8cy19e2vrl/GvezgHgPEwqdABgm1HoADAFFDoATAGFDgBToAq9+1V5/SVtc/w6AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA/A/+BerVzqlj4ef1AAAAAElFTkSuQmCC>