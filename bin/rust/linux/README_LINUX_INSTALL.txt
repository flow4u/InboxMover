========================================================================
INBOX MOVER (LINUX) - INSTALLATION & DESKTOP SHORTCUT GUIDE
========================================================================

Linux does not use Windows Defender real-time scanning or Windows
download blocking (Unblock-File), so native Linux executables start
immediately once they have execute permissions.

However, if you downloaded the binaries onto a slow network share
(such as /mnt/...), copying them to a local directory like /opt/InboxMover
prevents network read delays on startup.

------------------------------------------------------------------------
STEP 1: Copy binaries to a local folder & make them executable
------------------------------------------------------------------------
Open a terminal in the folder where you downloaded the files and run:

sudo mkdir -p /opt/InboxMover
sudo cp inbox-mover-rust-gui inbox-mover-rust-tui inbox-mover-rust /opt/InboxMover/
sudo chmod +x /opt/InboxMover/inbox-mover-rust*

(Or for single-user use in your own home folder, without sudo:)
mkdir -p ~/bin
cp inbox-mover-rust-gui inbox-mover-rust-tui inbox-mover-rust ~/bin/
chmod +x ~/bin/inbox-mover-rust*

------------------------------------------------------------------------
STEP 2: Launch Inbox Mover
------------------------------------------------------------------------
/opt/InboxMover/inbox-mover-rust-gui       # Desktop GUI
/opt/InboxMover/inbox-mover-rust-tui       # Terminal / SSH UI
/opt/InboxMover/inbox-mover-rust --cli     # Headless CLI batch mode

Note on shared configs (permit_configs):
If multiple users share /opt/InboxMover and should share the same local
permit_configs folder, ensure it is writable, or create a writable folder:
  sudo mkdir -p /opt/InboxMover/permit_configs
  sudo chmod 777 /opt/InboxMover/permit_configs

------------------------------------------------------------------------
STEP 3: Optional - Add an Application Menu / Desktop Shortcut
------------------------------------------------------------------------
To add "Inbox Mover" to the application menu for all users (requires the
icon and .desktop file from the repository's assets/ folder):

sudo cp assets/inbox_mover.png /usr/share/icons/hicolor/256x256/apps/inbox_mover.png
sudo sed 's|^Exec=.*|Exec=/opt/InboxMover/inbox-mover-rust-gui|' assets/inbox-mover.desktop | \
  sudo tee /usr/share/applications/inbox-mover.desktop > /dev/null

To also place a shortcut on your Desktop:
cp /usr/share/applications/inbox-mover.desktop ~/Desktop/
chmod +x ~/Desktop/inbox-mover.desktop
========================================================================
