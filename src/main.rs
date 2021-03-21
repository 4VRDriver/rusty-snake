use std::io::{stdout, Write};
use std::sync::mpsc;
use std::thread;

use crossterm::{
    cursor, event,
    style::{self, Colorize, Styler},
    terminal, ExecutableCommand, QueueableCommand,
};

fn draw<W>(writer: &mut W, event_rx: &mpsc::Receiver<event::Event>) -> crossterm::Result<()>
where
    W: Write,
{
    terminal::enable_raw_mode()?;
    //w.execute(event::EnableMouseCapture)?;
    writer.execute(terminal::EnterAlternateScreen)?;
    writer.execute(terminal::Clear(terminal::ClearType::All))?;

    for event in event_rx.iter() {
        let i = terminal::size()?.1-1;
        writer
            .queue(cursor::MoveTo(15, i))?
            //s.queue(terminal::Clear(terminal::ClearType::CurrentLine))?
            .queue(style::PrintStyledContent("Got: ".magenta()))?
            .queue(style::PrintStyledContent(format!("{:?}", event).red().bold()))?
            .queue(terminal::ScrollUp(1))?;
        writer.flush()?;
    }


    writer.execute(terminal::LeaveAlternateScreen)?;
    //w.execute(event::DisableMouseCapture)?;
    terminal::disable_raw_mode()
}

fn main() -> crossterm::Result<()> {
    let mut stdout = stdout();

    let (event_tx, event_rx) = mpsc::channel();

    let event_handler = thread::spawn(move || -> crossterm::Result<()> {
        loop {
            match event::read()? {
                event::Event::Key(event) => {
                    if event.code == event::KeyCode::Char('q') {
                        return Ok(());
                    }
                    event_tx
                        .send(event::Event::Key(event))
                        .expect("Unable to send event.");
                }
                event::Event::Mouse(event) => event_tx
                    .send(event::Event::Mouse(event))
                    .expect("Unable to send event."),
                _ => (),
            }
        }
    });

    draw(&mut stdout, &event_rx)?;

    event_handler
        .join()
        .expect("The event sender thread has panicked")
}
