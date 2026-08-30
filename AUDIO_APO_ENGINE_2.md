# Flaw Loud v1.1.1 RC — APO Compatibility Engine 2.0

This release rebuilds the External EQ / Equalizer APO signal path after a real-world recording showed severe flattening when a hot APO set was combined with Flaw's previous loudness stages.

## New APO signal order

External EQ / APO -> APO Input Guard 2.0 -> 72 Hz HPF -> bass-aware detector -> transparent compression -> transient/clarity recovery -> light saturation -> clean loudness recovery -> CleanGuard.

## APO Input Guard 2.0

- Fixed Base Headroom remains user-adjustable (0–18 dB; 8–12 dB recommended for aggressive sets).
- Automatic trim measures the actual incoming floating-point peak.
- The detector also accounts for Flaw Loud's own input/preamp gain before deciding how much trim is required.
- Fast attack catches hot sets; slow release avoids pumping between words.
- Target pre-dynamics peak is approximately -5.7 dBFS.
- Loudness recovery restores only part of the reserved headroom after the clean stages instead of feeding it back into clipping.

## Bass-aware compression

A 235 Hz low-frequency detector is removed from the compressor sidechain only. The audible bass is not removed. This prevents 100–500 Hz heavy APO boosts from pinning the whole compressor and making speech sound covered.

## Clarity preservation

When APO 2.0 is active:
- lower maximum compressor ratio;
- slower, more transparent compression envelope;
- reduced parallel pressure;
- stronger transient recovery;
- drastically lower added Body;
- less destructive HarshGuard behavior;
- much lighter De-Esser;
- slightly stronger air/detail recovery;
- saturation and overdrive are heavily reduced;
- output loudness is recovered with clean gain and limiter-aware feedback.

## Live telemetry

Dashboard and Settings expose:
- APO Total Trim;
- APO Input state (SAFE / GUARDED / HOT);
- Bass Guard percentage;
- Auto Input Guard trim;
- Clarity Recovery status.

## First test recommendation

1. Enable the same Equalizer APO set that previously sounded bad.
2. Enable `APO COMPATIBILITY ENGINE 2.0`.
3. Start Base Headroom around 10 dB.
4. Speak at the same loudness as before.
5. Watch `APO TOTAL TRIM`, `APO INPUT`, `BASS GUARD`, `COMP GR`, and `LIMITER GR`.
6. If the external set is extremely boosted, the automatic trim should add protection without requiring manual knob changes.

No Connected Platform, updater, account, moderation or report behavior was intentionally changed in this audio-focused RC.
