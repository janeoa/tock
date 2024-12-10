Cortex-M33 Architecture
======================

Architecture support for Cortex-M33 devices. 
Based on the Cortex-m4f for the FPU support.
Which is largely only re-exports the
correct functions from the Cortex-M crate.

_Note:_ Mainline Tock does not currently have any support for hard FPUs.
This is currently a direct clone of the cortexm4. However, chips wil FPUs
should point to this arch crate to pick up hard float support when it is
integrated.
