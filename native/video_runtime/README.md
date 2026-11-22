# Native video runtime

This crate defines the immutable frame contract used by the SMS runtime and desktop host.

## Frame delivery

- `VideoFrame` owns immutable pixel storage and carries dimensions, pixel format, stride and sequence.
- `FrameMailbox` is a single-slot latest-frame mailbox. A newer completed frame may replace an older frame not yet consumed by the host.

The active desktop presentation path is Slint 1.18 with WGPU 30 and Vulkan on Windows x64. The GUI consumes completed frames; it never advances the emulated machine or the MachineScheduler.

Presentation backpressure, VSync and GPU work must never modify emulated time. The runtime publishes completed frames independently of the graphics API.
