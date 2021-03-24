use std::io::{stdout, Write};
use std::sync::{mpsc::{self, TryRecvError}, Mutex, Arc};
use std::{thread, time};

use crossterm::{
    cursor, event,
    style::{self, Colorize, Styler},
    terminal, ExecutableCommand, QueueableCommand,
};

const CANVAS_WIDTH: u16 = 50;
const CANVAS_HEIGHT: u16 = 50;

const TICKS_PER_SEC: u16 = 15;

#[derive(Debug)]
struct Controller {
    should_close: bool,
    event_queue: Arc<Mutex<Vec<event::Event>>>,
    last_event: event::Event,
}

fn draw(writer: &mut impl Write, controller: &Controller) -> crossterm::Result<()> {

    writer.execute(terminal::Clear(terminal::ClearType::All))?;

    writer
        .queue(cursor::MoveTo(20, 2))?
        .queue(style::PrintStyledContent("Got: ".magenta()))?
        .queue(style::PrintStyledContent(format!("{:?}", controller.last_event).red().bold()))?;

    draw_borders(writer)?;
    writer.flush()?;

    Ok(())
}

fn draw_borders(writer: &mut impl Write) -> crossterm::Result<()> {
    let (terminal_width, terminal_height) = terminal::size()?;

    // Vertical lines
    for i in (terminal_height / 2 - CANVAS_HEIGHT / 4)..=(terminal_height / 2 + CANVAS_HEIGHT / 4) {
        writer
            .queue(cursor::MoveTo(terminal_width / 2 - CANVAS_WIDTH / 2, i))?
            .queue(style::Print("│"))?
            .queue(cursor::MoveTo(terminal_width / 2 + CANVAS_WIDTH / 2, i))?
            .queue(style::Print("│"))?;
    }

    // Horizontal lines and corners
    writer
        .queue(cursor::MoveTo(
            terminal_width / 2 - CANVAS_WIDTH / 2,
            terminal_height / 2 - CANVAS_HEIGHT / 4,
        ))?
        .queue(style::Print("╭"))?
        .queue(style::Print("─".repeat(CANVAS_WIDTH as usize - 1)))?
        .queue(style::Print("╮"))?;

    writer
        .queue(cursor::MoveTo(
            terminal_width / 2 - CANVAS_WIDTH / 2,
            terminal_height / 2 + CANVAS_HEIGHT / 4,
        ))?
        .queue(style::Print("╰"))?
        .queue(style::Print("─".repeat(CANVAS_WIDTH as usize - 1)))?
        .queue(style::Print("╯"))?;

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
                    controller.last_event = event::Event::Key(event);
                },
                event::Event::Mouse(event) => controller.last_event = event::Event::Mouse(event),
                event::Event::Resize(x, y) => controller.last_event = event::Event::Resize(x, y),
            }
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
        last_event: event::Event::Resize(0, 0),
    };

    let (terminate_event_tx, terminate_event_rx) = mpsc::channel();

    let event_queue = Arc::clone(&game_controller.event_queue);
    let event_handler = thread::spawn(move || -> crossterm::Result<()> {
        loop {
            if event::poll(time::Duration::from_millis(100))? {
                let event = event::read()?;
                while let Ok(ref mut queue) = event_queue.as_ref().lock() {
                    queue.push(event);
                    break;
                }
            }

            match terminate_event_rx.try_recv() {
                Ok(_) | Err(TryRecvError::Disconnected) => {
                    break Ok(());
                }
                Err(TryRecvError::Empty) => {}
            }
        }
    });

    let (tick_tx, tick_rx) = mpsc::channel();
    let (terminate_tick_tx, terminate_tick_rx) = mpsc::channel();

    let tick_handler = thread::spawn(move || loop {
        thread::sleep(time::Duration::from_millis(1000 / TICKS_PER_SEC as u64));
        tick_tx.send(()).ok();

        match terminate_tick_rx.try_recv() {
            Ok(_) | Err(TryRecvError::Disconnected) => {
                break;
            }
            Err(TryRecvError::Empty) => {}
        }
    });

    for _ in tick_rx {
        handle_events(&mut game_controller);
        draw(&mut stdout, &game_controller)?;

        if game_controller.should_close {
            break;
        }
    }

    terminate_tick_tx.send(()).ok();
    terminate_event_tx.send(()).ok();

    event_handler
        .join()
        .expect("The event sender thread has panicked")?;

    tick_handler
        .join()
        .expect("The tick sender thread has panicked");

    stdout
        .execute(terminal::LeaveAlternateScreen)?
        .execute(cursor::Show)?;
    //writer.execute(event::DisableMouseCapture)?;
    terminal::disable_raw_mode()
}
