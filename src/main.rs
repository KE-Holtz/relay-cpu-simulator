mod assembly;
use assembly::{Instruction, Opcode, Operand};
use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    prelude::Style,
    style::{Color, Stylize},
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget, Wrap},
};
use std::{collections::VecDeque, fmt::Display};
use std::{fs, io};
use std::{str::FromStr, time::Duration};

use crate::{
    ScrollType::{Natural, UserDefined},
    Speed::{Hertz, Instant, InstantPaused, Paused},
};

const HISTORY_SIZE: usize = 10_000;
const MAX_PROGRAM_LENGTH: usize = 256;
const ADDRESSES_IN_RAM: u8 = 32;

fn main() -> io::Result<()> {
    let args = Args::parse();

    let file_contents = fs::read_to_string(args.source).expect("Error: Unable to read source file");

    match parse_instructions(&file_contents) {
        Ok(instructions) => {
            if args.check {
                println!("Source code is OK!");
            } else {
                ratatui::run(|terminal| App::new(instructions, args.speed).run(terminal))?;
            }
        }
        Err(error_messages) => {
            for message in error_messages {
                println!("{}", message);
            }
        }
    }
    Ok(())
}

fn parse_instructions(source: &str) -> Result<[Instruction; MAX_PROGRAM_LENGTH], Vec<String>> {
    let noop = Instruction {
        opcode: Opcode::NOOP,
        operand: Operand::Constant(0u8),
    };
    let mut instructions = [noop; MAX_PROGRAM_LENGTH];
    let mut error_lines: Vec<String> = Vec::new();

    for (index, instruction_str) in source.lines().enumerate() {
        match instruction_str.parse() {
            Ok(instruction) => instructions[index] = instruction,
            Err(e) => error_lines.push(format!("Error at line {}: {}", index + 1, e)),
        }
    }

    if error_lines.is_empty() {
        Ok(instructions)
    } else {
        Err(error_lines)
    }
}

///An assembly simulator for @cockateil's relay CPU
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Assembly file to run
    source: String,

    /// Check the assembly file for errors
    #[arg(short, long, action = clap::ArgAction::SetTrue)]
    check: bool,

    #[arg(short, long, default_value = "INSTANT")]
    speed: Speed,
}

#[derive(Debug)]
struct App {
    memory: Memory,
    history: VecDeque<Memory>,
    instructions: [Instruction; MAX_PROGRAM_LENGTH],
    exit: bool,
    source_code_scroll: ScrollType,
    speed: Speed,
}

#[derive(Debug)]
enum ScrollType {
    Natural(u16),
    UserDefined(u16),
}

impl ScrollType {
    fn value(&self) -> u16 {
        match self {
            Natural(value) | UserDefined(value) => *value,
        }
    }
}

#[derive(Debug, Clone)]
enum Speed {
    Hertz(f64),
    Paused(f64),
    Instant,
    InstantPaused,
}

impl Speed {
    fn is_paused(&self) -> bool {
        matches!(self, Paused(_) | InstantPaused)
    }

    fn duration(&self) -> Duration {
        match self {
            Speed::Hertz(hz) => Duration::from_secs_f64(1.0 / hz),
            Paused(hz) => Duration::from_secs_f64(1.0 / hz),
            Instant => Duration::ZERO,
            InstantPaused => Duration::ZERO,
        }
    }
}

impl FromStr for Speed {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("INSTANT") {
            return Ok(Speed::Instant);
        }

        let value: f64 = s
            .parse()
            .map_err(|_| format!("{} is not a number or INSTANT", s))?;

        if !value.is_finite() || value <= 0.0 {
            return Err(format!("Speed must be a positive number, got '{}'", s));
        }

        Ok(Speed::Paused(value))
    }
}

impl Display for Speed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Hertz(hz) => format!("{} Hz", hz),
                Paused(hz) => format!("{} Hz, paused", hz),
                Instant => String::from("Unlimited Hz"),
                InstantPaused => String::from("Unlimited Hz, paused"),
            }
        )
    }
}

impl App {
    fn new(instructions: [Instruction; MAX_PROGRAM_LENGTH], speed: Speed) -> Self {
        Self {
            memory: Memory {
                ..Memory::default()
            },
            history: VecDeque::with_capacity(HISTORY_SIZE),
            instructions,
            exit: false,
            source_code_scroll: Natural(0),
            speed,
        }
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
            if !self.speed.is_paused() {
                self.step();
            }
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_events(&mut self) -> io::Result<()> {
        if self.speed.is_paused() || event::poll(self.speed.duration())? {
            match event::read()? {
                // it's important to check that the event is a key press event as
                // crossterm also emits key release and repeat events on Windows.
                // This basically just filters for keypresses
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    self.handle_key_event(key_event)
                }
                _ => {}
            };
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Down => self.scroll_down(),
            KeyCode::Up => self.scroll_up(),
            KeyCode::Left => self.decrease_speed(),
            KeyCode::Right => self.increase_speed(),
            KeyCode::Char('n') => self.step(),
            KeyCode::Char(' ') => self.toggle_paused(),
            KeyCode::Char('b') => self.undo(),
            _ => {}
        }
    }

    fn step(&mut self) {
        //Make scrolling automatic again. The value doesn't really mean a lot it just needs to be
        //stored so manual scrolling can start where natural left off.
        if !self.memory.halted {
            self.source_code_scroll = Natural(self.source_code_scroll.value());
            self.history.push_back(self.memory.clone()); //Save the current state to history
            self.memory
                .step(self.instructions[usize::from(self.memory.program_counter)]);
        }
    }

    fn undo(&mut self) {
        if let Some(prev) = self.history.pop_back() {
            self.memory = prev;
        }
    }

    fn scroll_up(&mut self) {
        self.source_code_scroll = UserDefined(self.source_code_scroll.value().saturating_sub(1))
    }

    fn scroll_down(&mut self) {
        self.source_code_scroll = UserDefined(self.source_code_scroll.value().saturating_add(1))
    }

    fn increase_speed(&mut self) {
        self.speed = match self.speed {
            Instant => Instant,
            InstantPaused => InstantPaused,
            Paused(hz) => Paused(if hz < 1.0 { hz + 0.1 } else { hz + 1.0 }),
            Hertz(hz) => Hertz(if hz < 1.0 { hz + 0.1 } else { hz + 1.0 }),
        }
    }

    fn decrease_speed(&mut self) {
        self.speed = match self.speed {
            Instant => Instant,
            InstantPaused => InstantPaused,
            Paused(hz) => Paused(if hz <= 1.0 {
                (hz - 0.1).max(0.1)
            } else {
                (hz - 1.0).max(0.1)
            }),
            Hertz(hz) => Hertz(if hz <= 1.0 {
                (hz - 0.1).max(0.1)
            } else {
                (hz - 1.0).max(0.1)
            }),
        }
    }

    fn toggle_paused(&mut self) {
        self.speed = match self.speed {
            Speed::Hertz(hz) => Paused(hz),
            Speed::Paused(hz) => Hertz(hz),
            Instant => InstantPaused,
            InstantPaused => Instant,
        };
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let color = if self.memory.halted {
            Color::Red
        } else if self.speed.is_paused() {
            Color::Blue
        } else {
            Color::Green
        };

        //===== Overarching box =====
        let title = Line::from(" Relay CPU Simulator ".bold());
        let block = Block::bordered()
            .title(title.centered())
            .border_set(border::THICK)
            .fg(color);
        let app_area = block.inner(area);
        block.render(area, buf);

        //===== Split into source veiwer and mem/debug =====
        let [left, right] =
            Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(80)])
                .areas(app_area);

        //===== Create source veiwer =====
        let source_title = Line::from(" Instructions ".bold());
        let source_title_bottom = Line::from(
            if self.memory.halted {
                format!(" {}, halted", self.speed)
            } else {
                format!(" {} ", self.speed)
            }
            .bold(),
        );

        let source_block = Block::bordered()
            .title(source_title.centered())
            .title_bottom(source_title_bottom.centered())
            .border_set(border::THICK)
            .fg(color);
        let source_area = source_block.inner(left);
        source_block.render(left, buf);

        let mut lines = vec![];
        for (index, instruction) in self.instructions.iter().enumerate() {
            let instruction_line = if index == usize::from(self.memory.program_counter) {
                let inline_message = if self.memory.halted {
                    "(Halted)"
                } else if self.speed.is_paused() {
                    "(Runs on step)"
                } else {
                    "(Running)"
                };
                Line::from(format!("{:03} | {} {}", index, instruction, inline_message))
                    .fg(Color::Red)
            } else {
                Line::from(format!("{:03} | {}", index, instruction)).fg(Color::White)
            };
            lines.push(instruction_line);
        }
        let text = Text::from(lines);

        let midpoint = source_area.height / 2;
        let scroll_amount = match self.source_code_scroll {
            UserDefined(amount) => amount,
            Natural(_) => {
                let amount = u16::from(self.memory.program_counter).saturating_sub(midpoint);
                self.source_code_scroll = Natural(amount);
                amount
            }
        };
        let paragraph = Paragraph::new(text).scroll((scroll_amount, 0));
        paragraph.render(source_area, buf);

        //===== Split right side into registers, ram, and debug =====
        let [register_area, ram_area, debug_tools_area] = Layout::vertical([
            Constraint::Percentage(10),
            Constraint::Percentage(80),
            Constraint::Percentage(10),
        ])
        .areas(right);

        //===== Render debug block =====
        let debug_title = Line::from(" Debug Tools ".bold());
        let debug_block = Block::bordered()
            .title(debug_title.centered())
            .border_set(border::THICK)
            .fg(color);
        let debug_inner_area = debug_block.inner(debug_tools_area);
        debug_block.render(debug_tools_area, buf);
        let debug_para = Paragraph::new(
            format!("Scroll: <Up>/<Down> | Step: n | Toggle pause: <Space> | Adjust speed: <Left>/<Right> | Undo ({} stored states): b", self.history.len())
        ).fg(Color::White).wrap(Wrap { trim: true });
        debug_para.render(debug_inner_area, buf);

        //===== Split RAM and Register areas into left and right =====
        let [card_x_area, card_y_area] =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .areas(ram_area);
        let [register_a_area, register_b_area] =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .areas(register_area);

        //===== Render registers =====
        let register_a_title = Line::from(" Register A ".bold());
        let register_a_block = Block::bordered()
            .title(register_a_title.centered())
            .border_set(border::THICK)
            .fg(color);
        let register_a_inner = register_a_block.inner(register_a_area);
        register_a_block.render(register_a_area, buf);

        let register_a_paragraph = Paragraph::new(format!("{:08b}", self.memory.register_a))
            .centered()
            .fg(Color::White);
        register_a_paragraph.render(register_a_inner, buf);

        let register_b_title = Line::from(" Register B ".bold());
        let register_b_block = Block::bordered()
            .title(register_b_title.centered())
            .border_set(border::THICK)
            .fg(color);
        let register_b_inner = register_b_block.inner(register_b_area);
        register_b_block.render(register_b_area, buf);

        let register_b_paragraph = Paragraph::new(format!("{:08b}", self.memory.register_b))
            .centered()
            .fg(Color::White);
        register_b_paragraph.render(register_b_inner, buf);

        //===== Render RAM =====
        let card_x_title = Line::from(" Card X ".bold());
        let card_x_block = Block::bordered()
            .title(card_x_title.centered())
            .border_set(border::THICK)
            .fg(color);
        let card_x_inner = card_x_block.inner(card_x_area);
        card_x_block.render(card_x_area, buf);

        let card_y_title = Line::from(" Card Y ".bold());
        let card_y_block = Block::bordered()
            .title(card_y_title.centered())
            .border_set(border::THICK)
            .fg(color);
        let card_y_inner = card_y_block.inner(card_y_area);
        card_y_block.render(card_y_area, buf);

        let mut card_x_lines = vec![];
        let mut card_y_lines = vec![];
        for (address, value) in self.memory.ram.iter().enumerate() {
            let line = if matches!(address, 14 | 15 | 30 | 31) {
                Line::from(format!("{:08b}: {:08b}", address, value))
                    .style(Style::default().fg(Color::Red))
            } else {
                Line::from(format!("{:08b}: {:08b}", address, value)).fg(Color::White)
            };

            if address < usize::from(ADDRESSES_IN_RAM / 2) {
                card_x_lines.push(line);
            } else {
                card_y_lines.push(line);
            }
        }
        let card_x_text = Text::from(card_x_lines);
        let card_y_text = Text::from(card_y_lines);

        let card_x_paragraph = Paragraph::new(card_x_text).centered();
        let card_y_paragraph = Paragraph::new(card_y_text).centered();

        card_x_paragraph.render(card_x_inner, buf);
        card_y_paragraph.render(card_y_inner, buf);
    }
}

// TODO: add I/O
// TODO: Get max to figure out what Mem is
#[derive(Clone, Debug)]
struct Memory {
    register_a: u8,
    register_b: u8,
    mem: u8,
    program_counter: u8,
    ram: [u8; 32],
    halted: bool,
}

impl Default for Memory {
    fn default() -> Self {
        Self {
            register_a: 0,
            register_b: 0,
            mem: 0,
            program_counter: 0,
            ram: [0u8; 32],
            halted: false,
        }
    }
}

impl Memory {
    fn step(&mut self, instruction: Instruction) {
        let inrement_counter = self.execute_instruction(instruction);

        if inrement_counter {
            self.program_counter += 1;
        }
    }

    //Returns true if the program_counter should be incremented after the instruction is run
    fn execute_instruction(&mut self, instruction: Instruction) -> bool {
        match instruction.opcode {
            Opcode::SUMA => self.register_a += self.register_b, //TODO: Determine overflow behaviour
            Opcode::SUMC => self.register_a += instruction.operand.value(),
            Opcode::SUBA => self.register_a -= self.register_b, //TODO: Determine underflow behaviour
            Opcode::SUBC => self.register_a -= instruction.operand.value(),
            Opcode::SFTL => self.ram[usize::from(instruction.operand.value())] <<= 1,
            Opcode::SFTR => self.ram[usize::from(instruction.operand.value())] >>= 1,
            Opcode::PSHA => self.ram[usize::from(instruction.operand.value())] = self.register_a,
            Opcode::PSHB => self.ram[usize::from(instruction.operand.value())] = self.register_b,
            Opcode::PULA => self.register_a = self.ram[usize::from(instruction.operand.value())],
            Opcode::PULB => self.register_b = self.ram[usize::from(instruction.operand.value())],
            Opcode::GTLA => {
                self.program_counter = self.ram[usize::from(instruction.operand.value())];
                return false;
            }
            Opcode::GTLC => {
                self.program_counter = instruction.operand.value();
                return false;
            }
            Opcode::GTZA => {
                if self.register_a == 0 {
                    self.program_counter = self.ram[usize::from(instruction.operand.value())];
                    return false;
                }
            }
            Opcode::GTZC => {
                if self.register_a == 0 {
                    self.program_counter = instruction.operand.value();
                    return false;
                }
            }
            Opcode::NOOP => (), //Do nothing
            Opcode::STOP => {
                self.halted = true;
                return false;
            }
        }
        true
    }
}
