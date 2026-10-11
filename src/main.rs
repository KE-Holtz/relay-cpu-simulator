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
    symbols::{border, line},
    text::{self, Line, Text},
    widgets::{Block, Paragraph, Widget},
};
use std::{cmp::max, fmt::format};
use std::{fs, io};

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
                ratatui::run(|terminal| App::new(instructions).run(terminal))?;
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

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Assembly file to run
    source: String,

    /// Check the assembly file for errors
    #[arg(short, long, action = clap::ArgAction::SetTrue)]
    check: bool,
}

#[derive(Debug)]
struct App {
    cpu: CPU,
    exit: bool,
    source_code_scroll: Option<u8>, //When Some(), user defined scroll, go to that value. When None,
                                    //autoscroll
}

impl App {
    fn new(instructions: [Instruction; MAX_PROGRAM_LENGTH]) -> Self {
        Self {
            cpu: CPU {
                instructions,
                ..CPU::default()
            },
            exit: false,
            source_code_scroll: None,
        }
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame));
            self.handle_events()?;
        }

        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            // it's important to check that the event is a key press event as
            // crossterm also emits key release and repeat events on Windows.
            // This basically just filters for keypresses
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Down => self.cpu.program_counter += 1,
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        //===== Overarching box =====
        let title = Line::from(" Relay CPU Simulator ".bold());
        let block = Block::bordered()
            .title(title.centered())
            .border_set(border::THICK);
        let app_area = block.inner(area);
        block.render(area, buf);

        //===== Split into source veiwer and mem/debug =====
        let [left, right] =
            Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(80)])
                .areas(app_area);

        //===== Create source veiwer =====
        let source_title = Line::from(" Instructions ".bold());
        let source_block = Block::bordered()
            .title(source_title.centered())
            .border_set(border::THICK);
        let source_area = source_block.inner(left);
        source_block.render(left, buf);

        let mut lines = vec![];
        for (index, instruction) in self.cpu.instructions.iter().enumerate() {
            let line_number = index + 1;
            let instruction_line = if index == usize::from(self.cpu.program_counter) {
                Line::from(format!("{:03} | {}", line_number, instruction))
                    .style(Style::default().fg(Color::Red))
            } else {
                Line::from(format!("{:03} | {}", line_number, instruction))
            };
            lines.push(instruction_line);
        }
        let text = Text::from(lines);

        let midpoint = source_area.height / 2;
        let scroll_amount = match self.source_code_scroll {
            Some(amount) => u16::from(amount),
            None => {
                if midpoint > u16::from(self.cpu.program_counter) {
                    0
                } else {
                    u16::from(self.cpu.program_counter) - midpoint
                }
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
            .border_set(border::THICK);
        let debug_inner_area = debug_block.inner(area);
        debug_block.render(debug_tools_area, buf);

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
            .border_set(border::THICK);
        let register_a_inner = register_a_block.inner(register_a_area);
        register_a_block.render(register_a_area, buf);

        let register_a_paragraph =
            Paragraph::new(format!("{:08b}", self.cpu.register_a)).centered();
        register_a_paragraph.render(register_a_inner, buf);

        let register_b_title = Line::from(" Register B ".bold());
        let register_b_block = Block::bordered()
            .title(register_b_title.centered())
            .border_set(border::THICK);
        let register_b_inner = register_b_block.inner(register_b_area);
        register_b_block.render(register_b_area, buf);

        let register_b_paragraph =
            Paragraph::new(format!("{:08b}", self.cpu.register_b)).centered();
        register_b_paragraph.render(register_b_inner, buf);

        //===== Render RAM =====
        let card_x_title = Line::from(" Card X ".bold());
        let card_x_block = Block::bordered()
            .title(card_x_title.centered())
            .border_set(border::THICK);
        let card_x_inner = card_x_block.inner(card_x_area);
        card_x_block.render(card_x_area, buf);

        let card_y_title = Line::from(" Card Y ".bold());
        let card_y_block = Block::bordered()
            .title(card_y_title.centered())
            .border_set(border::THICK);
        let card_y_inner = card_y_block.inner(card_y_area);
        card_y_block.render(card_y_area, buf);

        let mut card_x_lines = vec![];
        let mut card_y_lines = vec![];
        for (address, value) in self.cpu.ram.iter().enumerate() {
            let line = if matches!(address, 14 | 15 | 30 | 31) {
                Line::from(format!("{:08b}: {:08b}", address, value))
                    .style(Style::default().fg(Color::Red))
            } else {
                Line::from(format!("{:08b}: {:08b}", address, value))
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
#[derive(Copy, Clone, Debug)]
struct CPU {
    register_a: u8,
    register_b: u8,
    mem: u8,
    program_counter: u8,
    ram: [u8; 32],
    instructions: [Instruction; MAX_PROGRAM_LENGTH],
}

impl Default for CPU {
    fn default() -> Self {
        Self {
            register_a: 0,
            register_b: 0,
            mem: 0,
            program_counter: 0,
            ram: [0u8; 32],
            instructions: [Instruction::default(); MAX_PROGRAM_LENGTH],
        }
    }
}
