use std::io::{stdout, Write};
use std::sync::{mpsc::{self, TryRecvError}, Mutex, Arc};
use std::{thread, time};

use crossterm::{
    cursor, event,
    style::{self, Colorize, Styler},
    terminal, ExecutableCommand, QueueableCommand,
};

const CANVAS_WIDTH: u16 = 46;
const CANVAS_HEIGHT: u16 = 46;

const TICKS_PER_SEC: u16 = 20;

const BORDER_STYLE: [char; 6] = ['│', '─', '╭', '╮', '╰', '╯'];

#[derive(Debug)]
struct Controller {
    should_close: bool,
    event_queue: Arc<Mutex<Vec<event::Event>>>,
    last_event: Option<event::Event>,
}

fn draw(writer: &mut impl Write, controller: &Controller) -> crossterm::Result<()> {

    writer.execute(terminal::Clear(terminal::ClearType::All))?;

    if let Some(event) = controller.last_event {
        writer
            .queue(cursor::MoveTo(20, 2))?
            .queue(style::PrintStyledContent("Got: ".magenta()))?
            .queue(style::PrintStyledContent(format!("{:?}", event).red().bold()))?;
    }

    draw_borders(writer)?;
    writer.flush()?;

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
        .queue(cursor::MoveTo(
            left_border,
            upper_border,
        ))?
        .queue(style::Print(BORDER_STYLE[2]))?
        .queue(style::Print(BORDER_STYLE[1].to_string().repeat(CANVAS_WIDTH as usize - 1)))?
        .queue(style::Print(BORDER_STYLE[3]))?;

    writer
        .queue(cursor::MoveTo(
            left_border,
            lower_border,
        ))?
        .queue(style::Print(BORDER_STYLE[4]))?
        .queue(style::Print(BORDER_STYLE[1].to_string().repeat(CANVAS_WIDTH as usize - 1)))?
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
                },
                event::Event::Mouse(event) => controller.last_event = Some(event::Event::Mouse(event)),
                event::Event::Resize(x, y) => controller.last_event = Some(event::Event::Resize(x, y)),
            }
        }
    }
}

fn main() -> crossterm::Result<()> {
    let mut stdout = stdout();

    terminal::enable_raw_mode()?;
    stdout.execute(event::EnableMouseCapture)?;
    stdout
        .execute(terminal::EnterAlternateScreen)?
        .execute(cursor::Hide)?;

    let mut game_controller = Controller {
        should_close: false,
        event_queue: Arc::new(Mutex::new(Vec::new())),
        last_event: None,
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
        draw(&mut stdout, &game_controller)?;

        if game_controller.should_close {
            break;
        }
    }

    stdout
        .execute(terminal::LeaveAlternateScreen)?
        .execute(cursor::Show)?;
    stdout.execute(event::DisableMouseCapture)?;
    terminal::disable_raw_mode()
}
