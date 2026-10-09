# Native edge drag: research and implementation boundary

Status: **not implemented**. Automatic file copy/paste in 0.1.2 is a separate,
usable route. Existing app-window drops do not satisfy Finder/Explorer edge drag.

The drag payload belongs to an OS drag session, not to the mouse stream.
[Microsoft's Shell transfer guidance](https://learn.microsoft.com/en-us/windows/win32/shell/datascenarios)
requires OLE data objects/drop targets, with CF_HDROP for existing files and
file descriptors/streams for promised files. [DoDragDrop](https://learn.microsoft.com/en-us/windows/win32/api/ole2/nf-ole2-dodragdrop)
uses source cancellation and destination feedback within the local drag loop.
[Apple's NSDraggingSession](https://developer.apple.com/documentation/appkit/nsdraggingsession)
and [file promise APIs](https://developer.apple.com/documentation/appkit/supporting-drag-and-drop-through-file-promises)
require AppKit-owned source/destination lifecycles. Sending a held left button
across the LAN creates none of these data objects. Access to another app's
active drag is not equivalent to reading the general clipboard.

Next implementation must first validate OS-owned edge drop targets on physical
machines. Build a small receiving edge surface that can obtain Finder file URLs /
Explorer IDataObject, with explicit opt-in and a copy-only operation. Preserve
source cancellation; never send delete/move feedback. Route a bounded manifest
and file streams through existing encrypted bulk admission/consent. On the other
OS create an actual destination drag session on its UI thread, after verified
staging or with reviewed asynchronous promise lifecycles. Network callbacks must
not block an OLE STA or AppKit run loop, nor the input hooks. Invisible parking
windows used for cursor ownership are not registered file drop targets.

Acceptance needs Finder↔Explorer in both directions: held crossing, destination
folder drop, Escape, return to source, consent denial, disconnect, slow transfers,
multiple files, collisions, disk full, cloud placeholders, malicious paths and
cleanup. Test source files remain intact, destination bytes match, feedback is
honest, and both cursors/held buttons recover. A prototype can start with regular
files only; folders, file promises and asynchronous providers require further
work. This pass does not claim an untested shell integration as complete.
