use anyhow::Error;
use crossterm::event::{read, Event, KeyCode, KeyModifiers};
use crossterm::{self, cursor, terminal, ExecutableCommand};
use portable_pty::{Child, CommandBuilder, NativePtySystem, PtySize, PtySystem};
use std::io::{stderr, stdin, stdout, Read, Stderr, Stdin, Stdout, Write};
use std::sync::mpsc::channel;

const BUFF_SIZE: usize = 4096;

fn pipe(from: &mut dyn Read, to: &mut dyn Write) -> Result<(), std::io::Error> {
    let mut buff: [u8; BUFF_SIZE] = [0; BUFF_SIZE];
    loop {
        let len = from.read(&mut buff)?;
        to.write(&buff[..len])?;
        to.flush()?;
    }
}

fn term_init(out: &mut dyn Write) -> Result<(), std::io::Error> {
    out.execute(cursor::Hide)?;
    terminal::enable_raw_mode()?;
    Ok(())
}

fn term_deinit(out: &mut dyn Write) -> Result<(), std::io::Error> {
    terminal::disable_raw_mode()?;
    out.execute(cursor::Show)?;
    Ok(())
}

fn spawn(
    cols: u16,
    rows: u16,
    cmd: CommandBuilder,
) -> Result<
    (
        Box<dyn Read + Send>,
        Box<dyn Write + Send>,
        Box<dyn Child + Send>,
    ),
    Error,
> {
    let pty_system = NativePtySystem::default();
    let pair = pty_system.openpty(PtySize {
        cols,
        rows,
        pixel_width: 20,
        pixel_height: 20,
    })?;
    let reader = pair.master.try_clone_reader()?;
    let writer = pair.master.take_writer()?;
    let child = pair.slave.spawn_command(cmd)?;
    Ok((reader, writer, child))
}

fn main() {
    let mut err = stderr();

    //let (tx, rx) = channel::<Result<(), std::io::Error>>();

    let size = terminal::size().unwrap();

    let cmd = {
        let mut args = std::env::args_os().skip(1);
        let mut cmd = CommandBuilder::new(args.next().unwrap());
        cmd.args(args);
        cmd
    };

    let (mut reader, mut writer, mut child) = spawn(size.0, size.1, cmd).unwrap();

    term_init(&mut err).unwrap();

    std::thread::spawn(move || {
        let mut stdin = stdin();
        pipe(&mut stdin, &mut writer).unwrap();
    });

    std::thread::spawn(move || {
        let mut stderr = stderr();
        pipe(&mut reader, &mut stderr).unwrap();
    });

    let status = child.wait().unwrap();

    term_deinit(&mut err).unwrap();

    println!("child status: {:?}", status);
}
