use std::io::{stdin, stdout, Stdout, Write};

use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};
use termion::screen::{AlternateScreen, IntoAlternateScreen};
use termion::{clear, cursor};

use super::epub::{Chapter, Epub};
use super::log;
use super::Result;

fn redraw(
    screen: &mut AlternateScreen<RawTerminal<Stdout>>,
    lines: &Vec<&str>,
    scroll: usize,
    size: (u16, u16),
) -> Result<()> {
    write!(screen, "{}{}", clear::All, cursor::Goto(1, 1))?;

    let (term_width, term_height) = size;
    let mut y = 1;
    for line in lines.iter().skip(scroll) {
        write!(screen, "{}{}\r\n", cursor::Goto(1, y as u16), line)?;
        
        let line_height = (line.len() as u16 + term_width - 1) / term_width;
        if y + line_height as usize > term_height as usize {
            break;
        }
        y += line_height as usize;
    }

    screen.flush()?;

    Ok(())
}

pub fn read_ebook(ebook: &mut Epub) -> Result<()> {
    // Wrap raw terminal with alternate screen
    let mut screen = stdout().into_raw_mode()?.into_alternate_screen()?;
    write!(screen, "{}{}", clear::All, cursor::Goto(1, 1)).unwrap();
    let stdin = stdin();
    let keys = stdin.keys();

    let mut chp_num = 7;
    let mut lines: Vec<&str> = ebook.read_chapter(chp_num)?.lines().collect();
    let mut scroll = 0;
    let mut last_size = termion::terminal_size()?;

    write!(screen, "{}", termion::cursor::Hide).unwrap();

    log!("Number lines: {}", lines.len());

    redraw(&mut screen, &lines, scroll, last_size)?;

    for key in keys {
        let current_size = termion::terminal_size()?;
        if current_size != last_size {
            last_size = current_size;
            redraw(&mut screen, &lines, scroll, current_size)?;
            continue;
        }

        match key? {
            Key::Char('q') => {
                break;
            }
            Key::Char('n') => {
                chp_num += 1;
                lines = ebook.read_chapter(chp_num)?.lines().collect();
                scroll = 0;
            }
            Key::Char('p') if chp_num > 0 => {
                chp_num -= 1;
                lines = ebook.read_chapter(chp_num)?.lines().collect();
                scroll = 0;
            }
            Key::Up if scroll > 0 => {
                scroll -= 1;
                log!("Scrolled up. New scroll position: {}", scroll);
            }
            Key::Down if scroll < lines.len().saturating_sub(current_size.1 as usize - 3) => {
                scroll += 1;
                log!("Scrolled down. New scroll position: {}", scroll);
            }
            _ => {
                continue;
            }
        }
        redraw(&mut screen, &lines, scroll, current_size)?;
    }

    write!(screen, "{}", cursor::Show).unwrap();
    Ok(())
}
