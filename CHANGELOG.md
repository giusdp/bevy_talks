# Changelog

## 0.3.1 - 2023-11-04

### Changed

- Update to Bevy 0.12 with new asset system

## 0.3.0 - 2023-09-09

### Added

- Add `TalkerBundle`
- Add `CurrentText`, `CurrentNodeKind`, `CurrentActors`, `CurrentChoices` components to access Talk data
- Load actor image assets from the RawTalk in the loader as asset dependencies
- InitTalkRequest event to initialize/restart Talker components

### Changed

- Rename Screenplay to Talk
- Make Talk API methods private
- Use NodeIndex directly instead of ActionID to identify nodes
- Restructure folder layout
- Use RonTalk, RonActor, RonChoice to parse RON files and transform them into the "Raw" structs


### Removed

- action id to node index map in Talk
- ActionIds usage in nodes