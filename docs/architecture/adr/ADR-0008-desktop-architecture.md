# ADR-0008: Desktop and User Experience Architecture

- **Status:** proposed; specification accepted as design direction
- **Date:** 2026-10-10

## Decision
Build a distinctive, premium desktop with familiar Windows-like usability and original visual assets/branding. Common tasks are GUI-first; the terminal is a developer/advanced tool, not a prerequisite for basic installation or launching. Plan for a shell/launcher, taskbar, window management, settings, notifications, file manager, Downloads and native package-management UI.

Do not implement a fake desktop before there is a credible graphics, input and userland foundation. Define component boundaries (display/input APIs, compositor/window manager, shell and apps) in an ADR supported by prototypes when practical.

## Privacy implications
Recent files, search history and user convenience data should be local, controllable and clearable. No hidden activity timeline, screenshot capture or cloud dependency.

## Validation plan
Test real interactions, keyboard-only navigation, errors, accessibility and file workflows in a running build. A mockup screenshot is not evidence of a working desktop.
