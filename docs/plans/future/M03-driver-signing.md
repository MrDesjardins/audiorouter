# Set-aside M03 driver and signing track

Status: set aside indefinitely by explicit user cost decision on 2026-09-19
(originally deferred by scope decision on 2026-09-17). This is a business
decision, not a temporary technical blocker: the user does not have budget
for a production code-signing credential, and this project will not pursue
one. Do not describe this track as "in progress," "next," or "pending" in any
other document; it is set aside until the user explicitly reopens it with a
funded signing plan.

AudioRouter-owned virtual endpoints require a production-signed, Secure-Boot/
Memory-Integrity-compatible Windows driver. That signing and trusted
installation path costs money (an EV/driver-signing credential and,
typically, a Microsoft hardware dashboard submission) that is not available.
The project's supported and complete virtual-routing strategy is instead
binding to already-installed third-party virtual endpoints — VoiceMeeter
Banana and VB-Cable — as ordinary `physicalInput`/`physicalOutput` graph
nodes, exactly like a physical device. This is not a workaround or a partial
substitute for a future managed bus; it is the product's permanent virtual-
routing design for this project. See
[the "Existing virtual input/output" library entries](../../spec/09-interface.md)
and [06-virtual-devices.md](../../spec/06-virtual-devices.md) for the
supported path.

Resume only if the user later supplies budget and explicit instruction for a
production signing route and credentials, a signed x64 package, an isolated
qualification machine or equivalent approved test environment, and an agreed
rollback plan. Then resume M03 loaded-driver/PortCls bridge, endpoint
lifecycle, identity, ownership, recovery, and clean-machine evidence before
claiming M08 release readiness. Setting this track aside does not waive
VDEV-01 through VDEV-12 or SEC-08 as normative requirements of a future
signed track; it does mean they are explicitly **not** part of this
project's v1 completion target, per the user's 2026-09-19 decision.
