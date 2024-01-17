# Changelog

## Unreleased

### Added

- Add `NodeEventEmitter` component to emit events from nodes
- Add `TextNode` component to store the text of a node
- Add `ChoiceNode` component to store the choices of a node
- Add `JoinNode` component for the join node kind
- Add `LeaveNode` component for the leave node kind
- Add `StartNode` and `EndNode` components for the start and end node kinds
- Add `with_component` and `empty_node` methods to `TalkBuilder`
- Add `RefireNodeRequest` event to refire a node's events

### Changed

- Set `CurrentNode` visibility to public
- Builder adds the XNode components with the methods
- Builder now adds the components to the node entities instead of checking NodeKind
- `NextActionRequest` renamed to `NextNodeRequest`
- `ChooseActionRequest` renamed to `ChooseNodeRequest`

### Removed

- Component derive on `NodeKind`

## 0.4.0 - 2024-02-02

### Added
- `TalkCommands` to spawn the dialogue graphs
- Docs folder with an mdbook
- `Talk` component to store the current talk state 
- The `aery` dependency for the entity relationships
- `TalkBuilder` to build a talk programmatically 

### Changed

- TalkNodeKind renamed to NodeKind
- Remodeled the graph as many entities with the `FollowedBy` relationship between nodes and `PerfomedBy` between actors and nodes
- `RawTalk` is now `TalkData`
- Moved several validation checks to the ron loader

### Removed

- The `petgraph` dependency
- `TalkerBundle`
- `CurrentText`, `CurrentNodeKind`, `CurrentActors`, `CurrentChoices` components

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