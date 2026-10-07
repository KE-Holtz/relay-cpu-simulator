use std::{num::ParseIntError, str::FromStr};
use strum::EnumString;

#[repr(u8)]
#[derive(Copy, Clone, Debug, EnumString)]
pub enum Opcode {
    SUMA = 0b0000u8,
    SUMC = 0b0001u8, // Not in use/do nothing
    SUBA = 0b0010u8,
    SUBC = 0b0011u8,
    SFTL = 0b0100u8,
    SFTR = 0b0101u8,
    PSHA = 0b0110u8,
    PSHB = 0b0111u8,
    PULA = 0b1000u8,
    PULB = 0b1001u8,
    GTLA = 0b1010u8,
    GTLC = 0b1011u8,
    GTZA = 0b1100u8,
    GTZC = 0b1101u8,
    NOOP = 0b1110u8,
    STOP = 0b1111u8,
}

impl Opcode {
    fn takes_constant(&self) -> bool {
        matches!(
            self,
            Opcode::SUMC | Opcode::SUBC | Opcode::GTLC | Opcode::GTZC
        )
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Operand {
    Address(u8),
    Constant(u8),
}

#[derive(Copy, Clone, Debug)]
pub struct Instruction {
    pub opcode: Opcode,
    pub operand: Operand,
}

pub enum ParseInstructionError {
    InvalidFormat,
    InvalidOpcode,
    InvalidOperand(ParseIntError),
}

impl From<ParseIntError> for ParseInstructionError {
    fn from(value: ParseIntError) -> Self {
        Self::InvalidOperand(value)
    }
}

impl From<strum::ParseError> for ParseInstructionError {
    fn from(value: strum::ParseError) -> Self {
        Self::InvalidOpcode
    }
}

impl FromStr for Instruction {
    type Err = ParseInstructionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.trim().split_whitespace();
        let opcode_str = parts.next().ok_or(ParseInstructionError::InvalidFormat)?;
        let operand_str = parts.next().ok_or(ParseInstructionError::InvalidFormat)?;

        if parts.next().is_some() {
            return Err(ParseInstructionError::InvalidFormat);
        }

        let opcode: Opcode = opcode_str.parse()?;
        let operand_value: u8 = u8::from_str_radix(s, 2)?;

        let operand = if opcode.takes_constant() {
            Operand::Constant(operand_value)
        } else {
            Operand::Address(operand_value)
        };

        Ok(Instruction { opcode, operand })
    }
}
