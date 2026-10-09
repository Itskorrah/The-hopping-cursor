# Test the shared desk in 0.1.2

Use the private repository's [0.1.2 test release](https://github.com/Itskorrah/The-hopping-cursor/releases/tag/v0.1.2-test.1).
Sign into GitHub with the account that can access this repository.

1. On Windows fully **Quit** the old app using its tray menu. Download
   `The-Hopping-Cursor-0.1.2-windows-x64-test.zip` under **Assets**, extract it,
   and run `The Hopping Cursor_0.1.2_x64-setup.exe`. Keep existing settings;
   do not delete the configuration or unpair. The installer is unsigned.
2. Confirm **0.1.2** in both sidebars. Older 0.1.1 apps use a different protocol
   and cannot connect to this update. The local Mac app is in
   `~/Applications/The Hopping Cursor.app`; quit before replacing it.
3. Resume on both. On Mac arrange Mac left, Windows **Primary** middle and
   Windows secondary right. Use **Use OS display order** if needed, then
   **Save layout → Share arrangement**. On Windows open **Workspace**, review
   the positions and click **Apply arrangement**. Save/reset unsaved edits first.
4. Start with blank TextEdit/Notepad documents. Move the Mac trackpad right:
   it should enter the middle display, then the right display. Return left to
   Mac. Check only the destination cursor is visible and clicks go there.
5. Move the physical Windows mouse to claim that computer, then move left
   from its primary display into Mac. Try typing/clicking there. Check the Mac
   trackpad works again when you move it. Try each keyboard while its computer
   is idle: typing should follow the visible shared cursor.
6. Check **Diagnostics** on both for controller, destination and intended cursor
   visibility. Visibility state is not proof of actual Windows rendering.
   If anything sticks, **Ctrl+Alt+Esc** or **Bring input here** reclaims locally;
   **Pause** stops sharing. Test a held modifier/button, disconnect and reconnect.
7. Enable **Preferences → File clipboard** on both. Copy a harmless small file
   in Finder, accept its transfer on Windows, wait for completion, then paste
   into an Explorer folder. Repeat Explorer→Finder. Try two files, a duplicate
   name, decline, cancel, disabled file permission, and copying something newer
   while consent is pending. Newer clipboard contents must survive. Folders
   must be zipped; this is copy, not cut/move. Native edge dragging is unfinished.

Report which direction failed, both versions, controller/destination shown,
which screen edge and OS scaling. Do not send sensitive text or clipboard data.
Existing app transfers are automated-tested, but their physical behavior and
native clipboard interoperability are not accepted until these checks pass.
