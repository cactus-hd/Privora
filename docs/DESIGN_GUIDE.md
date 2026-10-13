# Privora UI/UX direction (v0.3)

## Direction

Privora is a quiet, privacy-focused utility, not a promotional website. The interface uses a dark ink background, low-saturation violet as the primary accent, clear neutral text, and restrained green/red only for meaningful success or failure states. The intended feeling is controlled and dependable, not cyberpunk or alarmist.

## What changed in v0.3

- Consistent larger body text and control sizing.
- More whitespace between concepts, with page title + explanatory subtitle.
- Clearer hierarchy between the selected content, package identity, and optional license policies.
- Explicit recipient label, validity start, expiry, local unlock limit and private note.
- Searchable license history and status labels for scheduled, available, expired, and locally exhausted policies.
- Empty-state actions that lead to the next useful step.
- Vector icons painted by egui rather than Unicode glyphs that depend on the installed font.

## Taste-Skill research notes

The linked [taste-skill](https://github.com/Leonxlnx/taste-skill) is primarily aimed at landing pages, portfolios and redesigns. Its own README/skill says dashboards, data tables and multi-step product UI are out of scope. For Privora, its audit-first approach, explicit design direction, restraint, and anti-template checks are useful; applying its web-specific rules wholesale to a native `egui` utility would be a mismatch.

## Pre-release visual check

Test the window at 960x640 and 1280x820 or larger. Check all five pages, long paths, non-English filenames, empty lists, error banners, narrow window resizing, text scaling, and the whole create/unlock workflow. Do not judge only the Home page. The current environment did not have Rust installed, so the visual QA must be performed on a Windows build machine.
