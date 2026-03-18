# **📦 Inbox Mover: Your Personal Workspace Assistant**

Welcome\! If you work with data in this workspace, you are probably familiar with the daily routine: checking your i:\\ drive, checking your z:\\inbox, finding vaguely named folders like transfer-2026-03-18, and manually digging through them to figure out where the files need to go.

**Inbox Mover** is a simple tool built to do all of that heavy lifting for you. It scans your inboxes, figures out what the files are, unzips them if necessary, moves them to the exact folder you want, and cleans up your inbox afterwards.

Here are some common questions and scenarios to help you get the most out of it\!

## **❓ Frequently Asked Questions**

### **1\. I hate checking two different inboxes. Do I still have to do that?**

**No\!** That is exactly why Inbox Mover was created.

By default, the application looks at both your i:\\ drive and your z:\\inbox at the same time. When you click the **↻ Refresh** button, it rounds up every single transfer- folder it finds in both places and puts them into one easy-to-read list for you to click through.

### **2\. I get different types of files. How do I make sure they go to the right place without me having to choose a folder every single time?**

Inbox Mover is smart—it remembers your preferences\! You only have to teach it once. It uses two methods to know where your files belong:

* **Scenario A: By File Name (Pattern Matching)**  
  Let's say you frequently receive database backups that always start with Archive. You can type Archive\*.\* into the tool's **Auto-Match Pattern** box, set your Target Folder to C:\\My\_Backups, and click **Save Config**.  
  *Next time a transfer folder arrives containing a file named Archive\_March.zip, Inbox Mover will instantly recognize it and have your backup folder already selected\!*  
* **Scenario B: By Project/Permit (Config IDs)**  
  Often, data arrives with an official "receipt" attached to it (a file called receipt.json). Inbox Mover reads this receipt to see which project or permit it belongs to. If you tell the tool that "Project A" belongs in the "Config Files" folder and save it, it will remember that rule forever.

### **3\. Most of my data arrives in ZIP files. Do I still have to right-click and "Extract All" manually?**

**Not anymore.** Inbox Mover has an **Auto-Extract ZIP files** checkbox. If you leave this checked, the tool will automatically open the ZIP file and pull all the actual files out into your Target Folder. If you actually *want* to keep the ZIP file closed and just move the .zip file itself, simply uncheck the box\!

### **4\. What happens if I move a file, but a file with that exact name already exists in my Target Folder? Will I lose my data?**

Inbox Mover has built-in safety nets called **Conflict Resolution**. You can choose exactly how it behaves before you click process:

* **Keep both (add number):** It will safely save the new file as Data (1).csv so nothing is lost.  
* **Rename existing file:** It will rename your *old* file with a timestamp (so you know it's an older version), and put the new file in its place.  
* **Overwrite:** It will simply replace the old file with the new one.

### **5\. After I move the files, my inbox is still cluttered with empty transfer- folders. Can the tool clean this up?**

**Yes\!** Look at the **Post Processing** section on the screen.

Instead of choosing "Leave the files in place," you can choose:

* **Delete the files:** Once Inbox Mover successfully moves your data, it will completely delete the messy transfer- folder from your inbox.  
* **Move to Processed Folder:** It will sweep the transfer- folder into a separate "Archive" or "Processed" folder of your choosing, keeping your main inbox completely clean but preserving a backup.

### **6\. I share this workspace with colleagues. How do I know if someone else already processed a folder?**

We built in an automatic paper trail\!

Whenever a folder is processed using Inbox Mover, it drops a hidden log file inside that folder. If you click on a folder that has already been handled, Inbox Mover will instantly show you a blue message on the screen saying exactly **when** it was processed, and **which user** clicked the button (e.g., *Latest: 2026-03-15 12:24:04 SUCCESS | User: jsmith*).

You can even click the **📄 Process Log** button to see exactly which files they moved and where they put them\!

### **💡 Quick Summary: The "Set it and Forget it" Workflow**

1. Open Inbox Mover.  
2. Click **Next ⇨** to look at an arriving folder.  
3. Choose where you want it to go, how to handle conflicts, and whether to delete the folder afterwards.  
4. Click **Save Config**.  
5. Click **PROCESS FOLDER**.

From now on, whenever that same type of file arrives, steps 3 and 4 are done for you automatically. Just click **PROCESS**\!

Try it yourself using the instructions in [**README.md**](https://github.com/flow4u/InboxMover/blob/main/README.md)
