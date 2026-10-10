# Desktop Experience Principles

**Status:** product principles; this is not a UI implementation or a final visual specification.

## Product feel

Aim for a distinctive, premium, calm desktop with the familiarity and discoverability expected from a mainstream PC operating system. Use original branding, icons, assets, typography choices and interaction details. Do not copy proprietary Windows artwork or imply Microsoft affiliation.

## Core workflows

- Launch, switch and arrange apps without a terminal.
- Browse, search, copy, move, rename, delete and recover files in a graphical file manager.
- Download files in a browser and find them predictably in Downloads.
- Double-click a supported application file to invoke its registered runtime, with an understandable security and compatibility flow.
- Install, update, repair and uninstall native apps through a GUI.
- Change defaults, permissions, startup apps, updates, privacy and accessibility in a unified settings experience.
- Use the core OS offline and with a local account.

## Interaction quality

- Keyboard and mouse support, predictable keyboard shortcuts, visible focus, keyboard navigation, readable contrast and scalable layouts.
- Use consistent spacing, typography, motion and control semantics through a design system rather than one-off values.
- Make destructive or privileged actions obvious and reversible when possible.
- Errors explain what happened, whether data is safe, and the next available action.
- Respect reduced-motion preferences and avoid animations that harm responsiveness.

## Privacy-sensitive design

- No “recent activity” or usage history by default beyond narrowly scoped user-visible convenience features.
- Recent-file lists should be user-controllable, local, clearable and excluded from telemetry.
- No automatic screenshots, keystroke capture, hidden AI indexing, or cloud account requirement.
- Explain any optional online service and its data flows before opt-in.

## Development order

Do not implement the desktop before there is a credible graphics/input/userland foundation. UX specifications and prototypes can be developed separately, but don't present a mockup as a working OS feature.
