# JotR

JotR is a lightweight, work-in-progress sticky note app for macOS, built with SwiftUI and Rust. It is designed for quickly jotting down notes, lists, reminders, or anything else that does not need a permanent home in a full-featured notes app.

Notes are meant to be quick, temporary, and easy to discard once they are no longer useful.

> JotR is currently in very early development. Right now, only basic note CRUD functionality is implemented.

## Project Structure

* `Jotr/` - macOS application and SwiftUI interface
* `jotr-core/` - Rust application core

See `GETTING_STARTED.md` for development notes.

## Roadmap

* [x] Basic note CRUD
* [ ] Local persistent storage with SQLite
* [ ] Basic desktop interface
* [ ] Note organization and management
* [ ] Self-hosted note syncing
* [ ] Multi-device synchronization
* [ ] iMessage integration for creating notes by text
* [ ] Web interface for accessing notes remotely
* [ ] Authentication and user accounts for remote access

## Tech

### Current

* **Rust** - core application logic
* **Swift / SwiftUI** - native macOS application and interface

### Planned / Considering

* **SQLite** - local persistent storage
* **Self-hosted database** - remote storage and synchronization
* **Web technologies** - future browser-based interface

## Status

JotR is currently being developed as a personal project with a focus on local-first functionality. Self-hosted syncing and remote access are planned for later development, and the architecture may change as the project grows.
