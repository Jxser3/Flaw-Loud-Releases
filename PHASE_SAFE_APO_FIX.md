# Flaw Loud v1.1.2 RC — Phase-Safe APO Capture

This hotfix targets the hollow / buried sound heard with some Equalizer APO sets.

## Root cause found in the supplied recording
The two recorded channels contain almost the same signal but offset by roughly 3.5 ms. When those channels are summed to mono, deep comb-filter cancellations appear through the voice band. The previous engine averaged all capture channels, which can reproduce this problem when a stereo capture/APO chain has unequal channel latency.

## Fix
- Capture channels are no longer blindly averaged.
- Phase-Safe Capture selects one coherent input channel.
- Automatic channel selection uses energy + hysteresis and only switches when another channel is clearly dominant.
- DSP still outputs a mono-coherent signal duplicated to the selected output endpoint channels.
- APO Input Guard 2.0 remains enabled and unchanged.

## Recommended Equalizer APO routing
Apply APO to the microphone/capture endpoint feeding Flaw Loud. Avoid applying the same set again to the VB-CABLE endpoint, or you can process the signal twice.
