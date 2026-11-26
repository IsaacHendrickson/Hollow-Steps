//! Prints a room as text: the room preview (`docs/room-tools.md`). A dev tool, not part of the game.
//!
//! ```text
//! cargo run --example room_preview -- first
//! cargo run --example room_preview -- assets/rooms/room_3.ron --cell 16
//! ```
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use hollow_steps::room_preview::{self, load, Ruler};

const USAGE: &str = "usage: room_preview <room> [--cell N] [--assets DIR]\n\
  <room>       first, second, boss, hub, a number or room_<number>, or the path of a room file\n\
  --cell N     pixels per character (default: the smallest of 4, 8, 16, 32, 64 that keeps the\n\
               widest room of the assets within 200 characters)\n\
  --assets DIR the assets folder (default: the one next to this crate, or the room file's own)";

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(text) => {
            print!("{text}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(2)
        }
    }
}

struct Args {
    room: String,
    cell: Option<f32>,
    assets: Option<PathBuf>,
}

fn parse(args: Vec<String>) -> Result<Args, String> {
    let (mut room, mut cell, mut assets) = (None, None, None);
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let (flag, inline) = match arg.split_once('=') {
            Some((flag, value)) if flag.starts_with("--") => {
                (flag.to_owned(), Some(value.to_owned()))
            }
            _ => (arg.clone(), None),
        };
        let mut value = |name: &str| {
            inline
                .clone()
                .or_else(|| args.next())
                .ok_or_else(|| format!("{name} needs a value\n{USAGE}"))
        };
        match flag.as_str() {
            "--cell" => {
                let text = value("--cell")?;
                cell = Some(
                    text.parse::<f32>()
                        .map_err(|_| format!("--cell {text:?} is not a number"))?,
                );
            }
            "--assets" => assets = Some(PathBuf::from(value("--assets")?)),
            "--help" | "-h" => return Err(USAGE.to_owned()),
            other if other.starts_with("--") => {
                return Err(format!("unknown option {other}\n{USAGE}"))
            }
            _ if room.is_none() => room = Some(arg),
            _ => return Err(format!("one room at a time, not {arg:?}\n{USAGE}")),
        }
    }
    let room = room.ok_or_else(|| format!("which room?\n{USAGE}"))?;
    Ok(Args { room, cell, assets })
}

fn run(args: Vec<String>) -> Result<String, String> {
    let args = parse(args)?;
    let looks_like_path = args.room.contains(['/', '\\']) || args.room.ends_with(".ron");
    let (id, assets) = if looks_like_path {
        let (id, own) = load::id_of_path(Path::new(&args.room))?;
        (id, args.assets.unwrap_or(own))
    } else {
        let assets = args
            .assets
            .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("assets"));
        (load::parse_id(&assets, &args.room)?, assets)
    };
    let loaded = load::load_one(&assets, id)?;
    let scene = loaded.scene();
    let ruler = Ruler::shipped(&assets)?;
    let cell = match args.cell {
        Some(cell) => cell,
        // The widest room decides, so every room is drawn at the same scale.
        None => room_preview::default_cell(
            load::load_all(&assets).map_or(loaded.room.bounds.w, |rooms| load::widest(&rooms)),
        ),
    };
    let cell = room_preview::check_cell(&scene, cell)?;
    Ok(room_preview::render(&scene, &ruler, cell))
}
