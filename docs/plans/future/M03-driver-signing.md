# Deferred M03 driver and signing track

Status: deferred by explicit scope decision on 2026-09-17.

AudioRouter-owned virtual endpoints are paused while the project lacks the
production signing and trusted installation path required for Windows Secure
Boot and Memory Integrity environments. The current execution track uses the
already-installed VB-Cable/Voicemeeter endpoints and physical WASAPI devices.

Resume only when all of the following are available: an approved production
signing route and credentials, a signed x64 package, an isolated qualification
machine or equivalent approved test environment, and an agreed rollback plan.
Then resume M03 loaded-driver/PortCls bridge, endpoint lifecycle, identity,
ownership, recovery, and clean-machine evidence before claiming M08 release
readiness. This deferral does not waive VDEV-01 through VDEV-12 or SEC-08.
