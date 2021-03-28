use std::io::{stdout, Write};
use std::sync::{mpsc, Arc, Mutex};
use std::{thread, time};

use crossterm::{
    cursor, event,
    style::{self, Colorize, Styler},
    terminal, ExecutableCommand, QueueableCommand,
};

const CANVAS_WIDTH: u16 = 46;
const CANVAS_HEIGHT: u16 = 46;

const TICKS_PER_SEC: u16 = 10;

const BORDER_STYLE: [char; 6] = ['│', '─', '╭', '╮', '╰', '╯'];

const APPLE: [char; 2] = ['🍎','🍏'];

#[derive(Debug, PartialEq)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
    Stop,
}
#[derive(Debug)]
struct Controller {
    should_close: bool,
    event_queue: Arc<Mutex<Vec<event::Event>>>,
    last_event: Option<event::Event>,
    snake: Snake,
}

#[derive(Debug)]
struct Snake {
    elements: Vec<CanvasSpace>,
    current_direction: Direction,
}

#[derive(Debug, Clone)]
struct CanvasSpace((u32, u32));
#[derive(Debug, Clone)]
struct TerminalSpace((u32, u32));

impl From<CanvasSpace> for TerminalSpace {
    fn from(canvas_space: CanvasSpace) -> Self {
        let (terminal_width, terminal_height) = terminal::size().unwrap();

        TerminalSpace((
            (terminal_width / 2).saturating_sub(CANVAS_WIDTH / 2) as u32 + canvas_space.0 .0 as u32 * 2 + 1,
            (terminal_height / 2).saturating_sub(CANVAS_HEIGHT / 4) as u32
                + canvas_space.0 .1 as u32 + 1,
        ))
    }
}

fn draw(writer: &mut impl Write, controller: &Controller) -> crossterm::Result<()> {
    writer.queue(terminal::Clear(terminal::ClearType::All))?;
      
    draw_borders(writer)?;
    draw_snake(writer, &controller.snake)?;

    if let Some(event) = controller.last_event {
            writer
                .queue(cursor::MoveTo(20, 40))?
                .queue(style::PrintStyledContent("Got: ".magenta()))?
                .queue(style::PrintStyledContent(
                    format!("{:?}", event).red().bold(),
                ))?;
    } else {
        let logo = include_str!("logo.txt");
        let line_len = logo.find('\n').expect("Logo has \\n");
        let (terminal_width, terminal_height) = terminal::size()?;

        for (index, line) in logo.split("\r\n").enumerate() {
            writer
                .queue(cursor::MoveTo((terminal_width / 2).saturating_sub((line_len / 6) as u16), index as u16 + (terminal_height / 2).saturating_sub(2)))?
                .queue(style::PrintStyledContent(line.dark_red()))?;
        }
    }

    writer.flush()?;

    Ok(())
}

fn draw_snake(writer: &mut impl Write, snake: &Snake) -> crossterm::Result<()> {

    for element in snake.elements.clone() {
        let position = TerminalSpace::from(element);
        writer
            .queue(cursor::MoveTo(position.0.0 as u16, position.0.1 as u16))?
            .queue(style::PrintStyledContent("██".red()))?;
    }

    Ok(())
}

fn draw_borders(writer: &mut impl Write) -> crossterm::Result<()> {
    let (terminal_width, terminal_height) = terminal::size()?;

    let left_border = (terminal_width / 2).saturating_sub(CANVAS_WIDTH / 2);
    let right_border = terminal_width / 2 + CANVAS_WIDTH / 2;

    let upper_border = (terminal_height / 2).saturating_sub(CANVAS_HEIGHT / 4);
    let lower_border = terminal_height / 2 + CANVAS_HEIGHT / 4;

    // Vertical lines
    for i in upper_border..=lower_border {
        writer
            .queue(cursor::MoveTo(left_border, i))?
            .queue(style::Print(BORDER_STYLE[0]))?
            .queue(cursor::MoveTo(right_border, i))?
            .queue(style::Print(BORDER_STYLE[0]))?;
    }

    // Horizontal lines and corners
    writer
        .queue(cursor::MoveTo(left_border, upper_border))?
        .queue(style::Print(BORDER_STYLE[2]))?
        .queue(style::Print(
            BORDER_STYLE[1]
                .to_string()
                .repeat(CANVAS_WIDTH as usize - 1),
        ))?
        .queue(style::Print(BORDER_STYLE[3]))?;

    writer
        .queue(cursor::MoveTo(left_border, lower_border))?
        .queue(style::Print(BORDER_STYLE[4]))?
        .queue(style::Print(
            BORDER_STYLE[1]
                .to_string()
                .repeat(CANVAS_WIDTH as usize - 1),
        ))?
        .queue(style::Print(BORDER_STYLE[5]))?;

    Ok(())
}

fn handle_events(controller: &mut Controller) {
    if let Ok(ref mut queue) = controller.event_queue.as_ref().lock() {
        while let Some(e) = queue.pop() {
            match e {
                event::Event::Key(event) => {
                    if event.code == event::KeyCode::Char('q') {
                        controller.should_close = true;
                    }
                    controller.last_event = Some(event::Event::Key(event));
                }
                event::Event::Mouse(event) => {
                    controller.last_event = Some(event::Event::Mouse(event))
                }
                event::Event::Resize(x, y) => {
                    //controller.last_event = Some(event::Event::Resize(x, y))
                    ()
                }
            }
        }
    }
}

fn continue_game_logic(controller: &mut Controller) {
    let snake = &mut controller.snake;

    match controller.last_event {
        Some(event::Event::Key(keyevent)) => {
            match keyevent.code {
                event::KeyCode::Up => snake.current_direction = Direction::Up,
                event::KeyCode::Down => snake.current_direction = Direction::Down,
                event::KeyCode::Left => snake.current_direction = Direction::Left,
                event::KeyCode::Right => snake.current_direction = Direction::Right,
                _ => (),
            }
        },
        _ => (),
    }

    if snake.current_direction != Direction::Stop {        
        let first_element = snake.elements.get(0).expect("First element should exist.").clone();
    
        snake.elements.rotate_right(1);
    
        let new_first_element = snake.elements.get_mut(0).expect("First element should exist.");
    
        *new_first_element = first_element;
    
        let (ref mut x, ref mut y) = new_first_element.0;
    
        match snake.current_direction {
            Direction::Left => *x -= 1,
            Direction::Right => *x += 1,
            Direction::Up => *y -= 1,
            Direction::Down => *y += 1,
            _ => (),
        }
    }
    
}

fn main() -> crossterm::Result<()> {
    let mut stdout = stdout();

    terminal::enable_raw_mode()?;
    //stdout.execute(event::EnableMouseCapture)?;
    stdout
        .execute(terminal::EnterAlternateScreen)?
        .execute(cursor::Hide)?;

    let mut game_controller = Controller {
        should_close: false,
        event_queue: Arc::new(Mutex::new(Vec::new())),
        last_event: None,
        snake: Snake {
            elements: vec![CanvasSpace(((CANVAS_WIDTH/4) as u32, (CANVAS_HEIGHT/4) as u32))],
            current_direction: Direction::Stop,
        },
    };

    let event_queue = Arc::clone(&game_controller.event_queue);
    let _ = thread::spawn(move || -> crossterm::Result<()> {
        loop {
            if event::poll(time::Duration::from_millis(100))? {
                let event = event::read()?;
                while let Ok(ref mut queue) = event_queue.as_ref().lock() {
                    queue.push(event);
                    break;
                }
            }
        }
    });

    // Create a sync channel with bound 0 so that it is absolutely synchronous.
    let (tick_tx, tick_rx) = mpsc::sync_channel(0);

    let _ = thread::spawn(move || loop {
        thread::sleep(time::Duration::from_millis(1000 / TICKS_PER_SEC as u64));
        tick_tx.send(()).expect("Could not send tick signal.");
    });

    for _ in tick_rx {
        handle_events(&mut game_controller);
        continue_game_logic(&mut game_controller);
        draw(&mut stdout, &game_controller)?;

        if game_controller.should_close {
            break;
        }
    }

    stdout
        .execute(terminal::LeaveAlternateScreen)?
        .execute(cursor::Show)?;
        //.execute(event::DisableMouseCapture)?;
    terminal::disable_raw_mode()
}
