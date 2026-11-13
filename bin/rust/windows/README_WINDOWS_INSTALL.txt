========================================================================
INBOX MOVER (WINDOWS) - FAST STARTUP & DESKTOP SHORTCUT GUIDE
========================================================================

If starting Inbox Mover on Windows (especially on a VM / myDRE) feels slow,
Windows Defender is scanning the downloaded executable on every launch, and
Windows may have marked the downloaded .exe files as "blocked".

Follow these 4 simple steps to make Inbox Mover start instantly and make it
available on the Desktop for all users on the computer.

------------------------------------------------------------------------
STEP 1: Copy the files to C:\Helpers
------------------------------------------------------------------------
1. Create a folder named:
      C:\Helpers
2. Copy all downloaded Windows files into C:\Helpers:
      - inbox-mover-rust-gui.exe   (Recommended Desktop GUI)
      - inbox-mover-rust-tui.exe   (Terminal UI, optional)
      - inbox-mover-rust.exe       (All-in-One CLI/TUI/GUI, optional)

------------------------------------------------------------------------
STEP 2: Open PowerShell as Administrator
------------------------------------------------------------------------
1. Press the Windows key (or click the Start button).
2. Type:  PowerShell
3. Right-click "Windows PowerShell" in the search results.
4. Click "Run as administrator" (click "Yes" if Windows asks permission).

------------------------------------------------------------------------
STEP 3: Speed up startup (Defender Exclusion + Unblock files)
------------------------------------------------------------------------
Copy and paste the commands below into the Administrator PowerShell window
and press Enter:

New-Item -ItemType Directory -Force -Path "C:\Helpers" | Out-Null
Add-MpPreference -ExclusionPath "C:\Helpers"
Unblock-File -Path "C:\Helpers\*.exe"

(If you prefer to unblock each executable individually, run:)
Unblock-File -Path "C:\Helpers\inbox-mover-rust-gui.exe"
Unblock-File -Path "C:\Helpers\inbox-mover-rust-tui.exe"
Unblock-File -Path "C:\Helpers\inbox-mover-rust.exe"

What this does:
- Add-MpPreference -ExclusionPath "C:\Helpers"
    Stops Windows Defender from re-scanning the .exe on every launch.
- Unblock-File -Path "C:\Helpers\*.exe"
    Removes the Windows internet-download lock ("Mark of the Web").

------------------------------------------------------------------------
STEP 4: Create a Public Desktop shortcut (for all users)
------------------------------------------------------------------------
Option A - Copy & Paste in PowerShell (Fastest):
Paste this into PowerShell and press Enter to create the shortcut in
C:\Users\Public\Desktop:

$WshShell = New-Object -ComObject WScript.Shell
$Shortcut = $WshShell.CreateShortcut("C:\Users\Public\Desktop\Inbox Mover.lnk")
$Shortcut.TargetPath = "C:\Helpers\inbox-mover-rust-gui.exe"
$Shortcut.WorkingDirectory = "C:\Helpers"
$Shortcut.Description = "Inbox Mover (Material 3 GUI)"
$Shortcut.Save()

Option B - Right-Click Manually in File Explorer:
1. Open C:\Helpers in File Explorer.
2. Right-click "inbox-mover-rust-gui.exe" -> "Create shortcut"
   (on Windows 11, click "Show more options" first if needed).
3. Rename the shortcut to "Inbox Mover".
4. Move the shortcut to:
      C:\Users\Public\Desktop\

Done! Every user on the machine will now see "Inbox Mover" on their Desktop,
and the application will launch quickly.
========================================================================
