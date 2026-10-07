mod assembly;
use assembly::{Instruction, Opcode, Operand};
use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::Stylize,
    symbols::border,
    text::Line,
    widgets::{Block, Widget},
};
use std::{fs, io};

fn main() -> io::Result<()> {
    let args = Args::parse();

    let file_contents = fs::read_to_string(args.source).expect("Error: Unable to read source file");

    if !args.check {
        ratatui::run(|terminal| App::default().run(terminal))?;
    }

    Ok(())
}

fn parse_instructions(instructions: &str) -> Result<[Instruction; 256], Vec<&str>> {
    let error_lines: Vec<&str> = Vec::new();

    for (line_number, instruction) in instructions.lines().enumerate() {
        let line_number = line_number + 1;
    }

    let nothing = Instruction {
        opcode: Opcode::NOOP,
        operand: Operand::Constant(0u8),
    };
    return Ok([nothing; 256]);
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

#[derive(Default, Debug)]
struct App {
    cpu: CPU,
    exit: bool,
}

impl App {
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
        let register_a_inner = register_a_block.inner(area);
        register_a_block.render(register_a_area, buf);

        let register_b_title = Line::from(" Register B ".bold());
        let register_b_block = Block::bordered()
            .title(register_b_title.centered())
            .border_set(border::THICK);
        let register_b_inner = register_b_block.inner(area);
        register_b_block.render(register_b_area, buf);

        //===== Render RAM =====
        let card_x_title = Line::from(" Card X ".bold());
        let card_x_block = Block::bordered()
            .title(card_x_title.centered())
            .border_set(border::THICK);
        let card_x_inner = card_x_block.inner(area);
        card_x_block.render(card_x_area, buf);

        let card_y_title = Line::from(" Card Y ".bold());
        let card_y_block = Block::bordered()
            .title(card_y_title.centered())
            .border_set(border::THICK);
        let card_y_inner = card_y_block.inner(area);
        card_y_block.render(card_y_area, buf);
    }
}

// TODO: add I/O
// TODO: Get max to figure out what Mem is
#[derive(Copy, Clone, Debug)]
struct CPU {
    A: u8,
    B: u8,
    Mem: u8,
    ProgramCounter: u8,
    RAM: [u8; 32],
    Instructions: [Option<Instruction>; 256],
}

impl Default for CPU {
    fn default() -> Self {
        Self {
            A: 0,
            B: 0,
            Mem: 0,
            ProgramCounter: 0,
            RAM: [0u8; 32],
            Instructions: [None; 256],
        }
    }
}
