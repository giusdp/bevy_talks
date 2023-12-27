use aery::prelude::*;

pub mod builder;
pub mod command;

/// The relationship of the dialogue nodes.
/// It needs to be Poly because the choice nodes can have multiple branches.
#[derive(Relation)]
#[aery(Recursive, Poly)]
pub struct FollowedBy;
