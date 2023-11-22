#[macro_use]
extern crate log;

pub mod screen;
use anyhow::{bail, Context};
use rogue_gym_core::{error::GameResult, input::InputCode, GameConfig, RunTime};
use rogue_gym_uilib::{process_reaction, Screen, Transition};
use screen::{RawTerm, TermScreen};
use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use termion::event::Key;
use termion::input::TermRead;

fn setup_screen(
    config: GameConfig,
    is_default: bool,
) -> GameResult<(TermScreen<RawTerm>, RunTime)> {
    let mut screen = TermScreen::from_raw(config.width, config.height)?;
    screen.welcome()?;
    if is_default {
        screen.default_config()?;
    }
    let mut runtime = config.build()?;
    thread::sleep(Duration::from_secs(1));
    screen.dungeon(&mut runtime)?;
    screen.status(&runtime.player_status())?;
    Ok((screen, runtime))
}

pub fn play_game(config: GameConfig, is_default: bool) -> GameResult<RunTime> {
    debug!("devui::play_game config: {:?}", config);
    let (mut screen, mut runtime) = setup_screen(config, is_default)?;
    let stdin = io::stdin();
    // let's receive keyboard inputs(our main loop)
    let mut pending = false;
    'outer: for keys in stdin.keys() {
        screen.clear_notification()?;
        let key = keys.context("in play_game")?;
        if pending {
            if runtime.is_cancel(key.into())? {
                pending = screen.display_msg()?;
            }
            continue;
        }
        let res = runtime.react_to_key(key.into());
        let res = match res {
            Ok(r) => r,
            Err(e) => {
                // STUB
                screen.message(format!("{}", e))?;
                continue;
            }
        };
        for reaction in res {
            let result =
                process_reaction(&mut screen, &mut runtime, reaction).context("in play_game")?;
            match result {
                Transition::Exit => break 'outer,
                Transition::None => {}
            }
        }
        pending = screen.display_msg()?;
    }
    screen.clear_screen()?;
    Ok(runtime)
}

pub fn show_replay(config: GameConfig, replay: Vec<InputCode>, interval_ms: u64) -> GameResult<()> {
    debug!("devui::show_replay config: {:?}", config);
    let (tx, rx) = mpsc::channel();
    let replay_thread = thread::spawn(move || {
        let res = show_replay_(config, replay, interval_ms, rx);
        if let Err(e) = res {
            eprintln!("Error in viewer: {}", e);
        }
    });
    let stdin = io::stdin();
    for key in stdin.keys() {
        let key = key.context("in show_replay")?;
        let mut end = false;
        let res = match key {
            Key::Char('E') | Key::Char('Q') | Key::Char('e') | Key::Char('q') | Key::Esc => {
                end = true;
                tx.send(ReplayInst::End)
            }
            Key::Char('p') => tx.send(ReplayInst::Pause),
            Key::Char('s') => tx.send(ReplayInst::Start),
            _ => continue,
        };
        if let Err(e) = res {
            eprintln!("Error in viewer: {}", e);
        }
        if end {
            break;
        }
    }
    replay_thread.join().unwrap();
    Ok(())
}

#[derive(Clone, Copy, Debug)]
enum ReplayInst {
    Pause,
    Start,
    End,
}

fn show_replay_(
    config: GameConfig,
    mut replay: Vec<InputCode>,
    interval_ms: u64,
    rx: mpsc::Receiver<ReplayInst>,
) -> GameResult<()> {
    let (mut screen, mut runtime) = setup_screen(config, false)?;
    let mut sleeping = false;
    replay.reverse();
    loop {
        match rx.try_recv() {
            Ok(ReplayInst::Start) => sleeping = false,
            Ok(ReplayInst::Pause) => sleeping = true,
            Ok(ReplayInst::End) => break,
            Err(mpsc::TryRecvError::Disconnected) => bail!("devui::show_replay disconnected!"),
            Err(mpsc::TryRecvError::Empty) => {}
        }
        thread::sleep(Duration::from_millis(interval_ms));
        if sleeping {
            continue;
        }
        let input = match replay.pop() {
            Some(x) => x,
            None => continue,
        };
        let res = runtime.react_to_input(input);
        let res = match res {
            Ok(r) => r,
            Err(e) => {
                screen.message(format!("{}", e))?;
                continue;
            }
        };
        let left_turns = replay.len();
        if left_turns == 0 {
            screen.message(format!("--Press q or e to exit--"))?;
        } else {
            screen.message(format!("{} turns left", replay.len()))?;
        }
        for reaction in res {
            let result =
                process_reaction(&mut screen, &mut runtime, reaction).context("in show_replay")?;
            match result {
                Transition::Exit => return Ok(()),
                Transition::None => {}
            }
        }
    }
    screen.clear_screen()
}
