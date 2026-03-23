# **📦 Inbox Mover: Your Personal Workspace Assistant**

Welcome\! If you work with data in this workspace, you are probably familiar with the daily routine: checking your i:\\ drive, checking your z:\\inbox, finding vaguely named folders like transfer-2026-03-18, and manually digging through them to figure out where the files need to go.

**Inbox Mover** is a simple tool built to do all of that heavy lifting for you. It scans your inboxes, figures out what the files are, unzips them if necessary, moves them to the exact folder you want, and cleans up your inbox afterwards.

Here are some common questions and scenarios to help you get the most out of it\!

## **❓ Frequently Asked Questions**

### **1\. I hate checking two different inboxes. Can I automate this?**

**YES\!** That is exactly why Inbox Mover was created.

By default, the application looks at both your i:\\ drive and your z:\\inbox at the same time. Click **↻ Scan** to aggregrate every **transfer-** folder from all locations in the master list.

### **2\. How do I share rules with my colleagues? (The Team Feature)**
This is one of Inbox Mover's most powerful features.

* **Personal Mode**: Rules are stored only on your local machine.

* **Team Shared Mode**: Select "Team Shared" in the Workspace settings and point it to a folder on a network drive. Any rule you save while in "Shared" mode is instantly available to everyone else on your team using that same network path. When you first connect to a shared folder, the app will even offer to Merge your local rules into the team database so you don't have to start from scratch.


### **3\. How do I know if a colleague already processed a folder?**
We built an automatic Paper Trail.
Whenever a folder is processed, a hidden log is created inside it. If you select a folder in the queue that has already been handled, a blue status message will appear:

*Latest: 2026-03-15 12:24:04 | User: stefan.vanaalst | Config: PROJECT_X*

You can click the **📄 View Log** button next to the folder name to see exactly what they did. In the folder tree all processed folders that are not moved nor deleted are marked with **L** for log. Top right you can easily view the log.


### **4\. I get different types of files. How do I make sure they go to the right place without me having to choose a folder every single time?**

Inbox Mover remembers your preferences so you only have to "teach" it once. It uses a tiered logic system:

* **Scenario A**: By **Config ID** (Priority): Most transfers include a receipt.json. The app reads this file to find a Project or Permit ID. If you save a rule for "Project_Alpha," any future folder with that ID will automatically load your saved paths.

* **Scenario B**: By **Pattern Matching** of Filenames: If there is no ID, you can use the Pattern Match field. Enter a wildcard like **survey_*.dwg**. Next time a folder arrives containing a file that matches that pattern, Inbox Mover will recognize it and apply your routing rules automatically.


### **5\. Most of my data arrives in ZIP files. Do I still have to right-click and "Extract All" manually?**

**Not anymore.** Inbox Mover has an **Auto-Extract ZIP files** checkbox. If you leave this checked, the tool will automatically open the ZIP file and pull all the actual files out into your Target Folder. If you actually *want* to keep the ZIP file closed and just move the .zip file itself, simply uncheck the box\!
In case of encrypted zips, a password input will pop up.

### **6\. What happens if I move a file, but a file with that exact name already exists in my Target Folder? Will I lose my data?**

Inbox Mover has built-in safety nets called **Conflict Resolution**. You can choose exactly how it behaves before you click process:

* **Keep both (add number):** It will safely save the new file as Data (1).csv so nothing is lost.  
* **Rename existing file:** It will rename your *old* file with a timestamp (so you know it's an older version), and put the new file in its place.  
* **Overwrite:** It will simply replace the old file with the new one.

### **7\. After I move the files, my inbox is still cluttered with empty transfer- folders. Can the tool clean this up?**
**Yes**. Use the FOLDER section's Post Action setting:

* **Leave**: Do nothing (folders stay in the inbox).

* **Delete**: Permanently delete the transfer- folder after successful extraction.

* **Move**: Sweep the folder into a "Processed" or "Archive" directory of your choice.
Just in case, InBox Mover also allows you to manually delete folders.


### **8\. Can I control where my files go *before* I even upload them?**

**Yes\! You can dictate exactly what Inbox Mover does by including your own receipt.json file in your upload.**

If you create a simple text file named receipt.json and place it alongside your data (either inside your ZIP file or just sitting loose in the folder), Inbox Mover will automatically read it and force the tool to follow your rules.

Here is an example of what you can type inside your custom receipt.json file, grouped logically from file extraction to final cleanup:

{  
    "permitId": "MY-PROJECT-123",  
    "auto\_extract": true,  
    "target\_folder": "C:\\\\Data\\\\My\_Project\\\\Incoming",  
    "receipt\_folder": "C:\\\\Data\\\\My\_Project\\\\Receipts",  
    "conflict\_resolution": "rename\_existing",  
    "post\_processing": "move",  
    "process\_folder": "C:\\\\Data\\\\Archive\\\\Processed"  
}

**What do these options mean?**

* "permitId": A unique name or ID for your project/dataset.  
* "auto\_extract": Set to true if you want the app to automatically unzip your files, or false to keep them zipped.  
* "target\_folder": The exact folder path where your data files should land.  
* "receipt\_folder": The folder where you want this receipt.json file to be copied and timestamped (useful for keeping an audit trail separate from the data).  
* "conflict\_resolution": Choose "overwrite", "keep\_both", or "rename\_existing".  
* "post\_processing": Choose "leave", "delete", or "move" (to clean up the inbox).  
* "process\_folder": If you chose "move", tell it where the processed transfer- folder should be archived.

If Inbox Mover detects these settings in your file, it will instantly apply them and turn the "Save Config" button Orange to let you know your custom rules are active\!

## ⌨️ Shortcuts for Power Users

Inbox Mover is designed for hands-on-keyboard speed:

* **Arrows (Up/Down or Left/Right)**: Cycle through the pending queue.

* **Tab**: Cycle focus between Open, Delete, and Process Folders. **Tab** also works in the **Delete Folder** popup.

* **Enter**: Execute the highlighted button (look for the ► arrows).



## **💡 The "Set it and Forget it" Workflow**

1. Set your **Target** and **Post Action**
2. Use Pattern Matching
3. Click 💾 Save   
4. Increase/decrease font & resize
5. Choose dark/light mode

From now on, whenever that same type of file arrives, everything will be processed accordingly. Just click **PROCESS**\!
