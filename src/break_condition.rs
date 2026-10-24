use crate::Memory;
use std::str::FromStr;

pub struct BreakCondition(Box<dyn Fn(&Memory) -> bool>);

pub enum ParseBreakConditionError {} //TODO

impl std::fmt::Display for ParseBreakConditionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl FromStr for BreakCondition {
    type Err = ParseBreakConditionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        todo!()
    }
}
