use std::fs::{File, OpenOptions};
use std::io::prelude::*;

use anyhow::{bail, Context};
use clap::ArgMatches;
use rogue_gym_core::{error::GameResult, json_to_inputs, read_file, GameConfig};
use rogue_gym_devui::{play_game, show_replay};

const DEFAULT_INTERVAL_MS: u64 = 500;

fn main() -> GameResult<()> {
    let args = parse_args();
    let (mut config, is_default) = get_config(&args)?;
    if let Some(seed) = args.value_of("seed") {
        config.seed = Some(seed.parse().context("Failed to parse seed!")?);
    }
    setup_logger(&args)?;
    if let Some(replay_arg) = args.subcommand_matches("replay") {
        let fname = replay_arg.value_of("file").unwrap();
        let replay = read_file(fname).context("Failed to read replay file!")?;
        let replay = json_to_inputs(&replay)?;
        let mut interval = DEFAULT_INTERVAL_MS;
        if let Some(inter) = replay_arg.value_of("interval") {
            interval = inter.parse().context("Failed to parse 'interval' arg!")?;
        }
        show_replay(config, replay, interval)
    } else {
        let runtime = play_game(config, is_default)?;
        if let Some(save_file) = args.value_of("save") {
            let s = runtime.saved_inputs_as_json()?;
            let mut file = File::create(save_file)?;
            file.write_all(s.as_bytes())?;
        }
        Ok(())
    }
}

fn get_config(args: &ArgMatches) -> GameResult<(GameConfig, bool)> {
    let file_name = match args.value_of("config") {
        Some(fname) => fname,
        None => {
            return Ok((GameConfig::default(), true));
        }
    };
    if !file_name.ends_with(".json") {
        bail!("Only .json file is allowed as configuration file")
    }
    let f = read_file(file_name).context("in get_config")?;
    Ok((GameConfig::from_json(&f)?, false))
}

fn parse_args<'a>() -> ArgMatches<'a> {
    clap::App::new("rogue-gym developper ui")
        .version("0.1.0")
        .author("Yuji Kanagawa <yuji.kngw.80s.revive@gmail.com>")
        .about("play rogue-gym as human")
        .arg(
            clap::Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("CONFIG")
                .help("Sets your config json file")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("log")
                .short("l")
                .long("log")
                .value_name("LOG")
                .help("Enable logging to log file")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("filter")
                .short("f")
                .long("filter")
                .value_name("FILTER")
                .help("Set up log level")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("seed")
                .short("s")
                .long("seed")
                .value_name("SEED")
                .help("Set seed")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("save")
                .long("save")
                .value_name("SAVE")
                .help("save replay file")
                .takes_value(true),
        )
        .subcommand(
            clap::SubCommand::with_name("replay")
                .about("Show replay by json file")
                .version("0.1")
                .arg(
                    clap::Arg::with_name("file")
                        .short("f")
                        .long("file")
                        .required(true)
                        .value_name("FILE")
                        .help("replay json file")
                        .takes_value(true),
                )
                .arg(
                    clap::Arg::with_name("interval")
                        .short("i")
                        .long("interval")
                        .value_name("INTERVAL")
                        .help("Interval in replay mode")
                        .takes_value(true),
                ),
        )
        .get_matches()
}

fn setup_logger(args: &ArgMatches) -> GameResult<()> {
    if let Some(file) = args.value_of("log") {
        let level = args.value_of("filter").unwrap_or("debug");
        let level = convert_log_level(level).unwrap_or(log::LevelFilter::Debug);
        fern::Dispatch::new()
            .format(|out, message, record| {
                out.finish(format_args!(
                    "{}[{}][{}] {}",
                    chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                    record.target(),
                    record.level(),
                    message
                ))
            })
            .level(level)
            .chain(
                OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(file)
                    .context("error in getting log file")?,
            )
            .apply()
            .context("error in setup_log")?;
    }
    Ok(())
}

fn convert_log_level(s: &str) -> Option<log::LevelFilter> {
    use log::LevelFilter::*;
    let s = s.to_lowercase();
    match &*s {
        "off" | "o" => Some(Off),
        "error" | "e" => Some(Error),
        "warn" | "w" => Some(Warn),
        "info" | "i" => Some(Info),
        "debug" | "d" => Some(Debug),
        "trace" | "t" => Some(Trace),
        _ => None,
    }
}
