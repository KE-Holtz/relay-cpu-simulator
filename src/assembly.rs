use std::{num::ParseIntError, str::FromStr};
use strum::{Display, EnumString};

#[repr(u8)]
#[allow(clippy::upper_case_acronyms)]
#[derive(Copy, Clone, Debug, EnumString, Display)]
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
    pub fn takes_constant(&self) -> bool {
        matches!(
            self,
            Opcode::SUMC | Opcode::SUBC | Opcode::GTLC | Opcode::GTZC
        )
    }
}

//TODO: is this struct needed? make sure that it at least keeps addresses valid
#[derive(Copy, Clone, Debug)]
pub enum Operand {
    Address(u8),
    Constant(u8),
}

impl Operand {
    pub fn value(&self) -> u8 {
        match self {
            Self::Address(value) | Self::Constant(value) => *value,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Instruction {
    pub opcode: Opcode,
    pub operand: Operand,
}

impl Default for Instruction {
    fn default() -> Self {
        Self {
            opcode: Opcode::NOOP,
            operand: Operand::Constant(0),
        }
    }
}

pub enum ParseInstructionError {
    InvalidFormat,
    UnkownOpcode,
    InvalidOperand(ParseIntError),
}

impl From<ParseIntError> for ParseInstructionError {
    fn from(value: ParseIntError) -> Self {
        Self::InvalidOperand(value)
    }
}

impl From<strum::ParseError> for ParseInstructionError {
    fn from(_value: strum::ParseError) -> Self {
        Self::UnkownOpcode
    }
}

impl std::fmt::Display for ParseInstructionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFormat => write!(f, "Invalid Format"),
            Self::UnkownOpcode => write!(f, "Invalid Opcode"),
            Self::InvalidOperand(error) => write!(f, "Error parsing Operand ({})", error),
        }
    }
}

impl FromStr for Instruction {
    type Err = ParseInstructionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split_whitespace();
        let opcode_str = parts.next().ok_or(ParseInstructionError::InvalidFormat)?;
        let operand_str = parts.next().ok_or(ParseInstructionError::InvalidFormat)?;

        if parts.next().is_some() {
            return Err(ParseInstructionError::InvalidFormat);
        }

        let opcode: Opcode = opcode_str.parse()?;
        let operand_value: u8 = u8::from_str_radix(operand_str, 2)?;

        let operand = if opcode.takes_constant() {
            Operand::Constant(operand_value)
        } else {
            Operand::Address(operand_value)
        };

        Ok(Instruction { opcode, operand })
    }
}

impl std::fmt::Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.operand {
            Operand::Address(value) | Operand::Constant(value) => {
                write!(f, "{} {:08b}", self.opcode, value) //:08b makes it display as 8 digits of binary
            }
        }
    }
}

impl Instruction {
    pub fn base(&self, base: &Base) -> String {
        match base {
            Base::Binary => format!("{} {:08b}", self.opcode, self.operand.value()),
            Base::Decimal => format!("{} {}", self.opcode, self.operand.value()),
            Base::Hex => format!("{} 0x{:02x}", self.opcode, self.operand.value()),
        }
    }
}
#[derive(Debug)]
pub enum Base {
    Binary,
    Decimal,
    Hex,
}

impl Base {
    pub fn next(&self) -> Self {
        match self {
            Base::Binary => Base::Decimal,
            Base::Decimal => Base::Hex,
            Base::Hex => Base::Binary,
        }
    }
}
