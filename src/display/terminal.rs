//! Terminal display. Talks are printed in a terminal stdout

use crate::display::TalkDisplay;
use bevy::prelude::Component;

/// Terminal display. Talks are printed in a terminal stdout
#[derive(Component, Default)]
pub struct TerminalDisplay {}
