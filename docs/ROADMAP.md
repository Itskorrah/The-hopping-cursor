# From alpha to release

The implemented engine is ready for controlled two-computer testing. This list
orders the remaining work by its effect on a reliable product. It does not imply
that unfinished features currently work.

1. **Validate native input on hardware.** Complete PLATFORM_TESTING.md for
   Windows→Windows, Mac→Mac, and both directions of Windows↔Mac. Include Retina,
   Windows scaling, offset displays, held drags/modifiers, keyboard layouts,
   permission denial, disconnects, sleep, and multi-hour use. Fix failures before
   broadening the feature surface. Measure end-to-end latency and throughput.
2. **Validate and finish display/session reliability.** Hotplug polling, monitor UUID/interface IDs, profile reconciliation, rename
   advertisement refresh, single-instance reopening, and Keychain/DPAPI migration
   are implemented. Validate on physical docks, identical monitors, virtual
   displays, permission changes, locked Keychains, and Windows account recovery.
   Cross-port identity persistence remains driver/hardware dependent.
3. **Finish native file interoperability.** Explicit Explorer CF_HDROP / Finder
   file-URL clipboard actions now use consented streamed transfers. Validate these
   across applications, opt-in automatic verified file batches now have encrypted integration checks.
   Next add Explorer OLE drag objects and Finder file promises with real OS drag lifecycles.
   Preserve recipient trust, explicit policy, duplicate protection, cancellation,
   and temporary cleanup. Test OS drag lifecycle and destination applications;
   app-window drops alone do not satisfy native edge dragging or file clipboard.
4. **Add OS power and session adapters.** Lock/wake notifications, deliberate
   synchronization controls, and supported brightness APIs with restoration on
   stop/disconnect. Protected input/desktop checks, permission loss, disabled Mac taps and timing-gap
   recovery now release ownership conservatively. Full OS lock/wake notifications
   and synchronized actions remain unfinished.
5. **Broaden keyboard and game support.** Configurable physical modifier mapping,
   media/vendor keys, international layout coverage, and true relative/raw mouse
   input. Define cursor-lock escape behavior and test actual fullscreen games.
6. **Harden and package.** Expand the bounded parser fuzz campaign and bulk-pressure
   regressions into sustained protocol/adversarial resource testing,
   external security review, signed Windows installers, notarized Mac packages,
   and verified signed updates. Secure desktop/UAC control requires separate
   privileged Windows-service design and review.
7. **Prepare a commercial product if desired.** Final name/trademark checks,
   license policy and offline activation, release/support process, and a truthful
   website based on measured capabilities. The alpha has no paid tiers or limits
   on the number of paired computers.
