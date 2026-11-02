//! Opt-in warm-cache comparison of checked and ordinary opens on 400 MiB.
//! Creates only a new disposable target subdirectory; never opens user data.
use std::error::Error;
use std::time::Instant;

use audiorouter_storage::Storage;

fn storage_error(error: audiorouter_storage::StorageError) -> std::io::Error {
    std::io::Error::other(format!("{error:?}"))
}

fn main() -> Result<(), Box<dyn Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target")
        .join(format!("review-storage-open-{}", std::process::id()));
    std::fs::create_dir(&root)?;
    let database = root.join("synthetic.sqlite");
    let storage = Storage::open(&database).map_err(storage_error)?;
    let bytes = vec![0x5a; 50 * 1024 * 1024];
    for index in 0..8 {
        storage
            .store_audio_media(
                &format!("synthetic-{index}"),
                "fixture.wav",
                "wav",
                &bytes,
                None,
            )
            .map_err(storage_error)?;
    }
    drop(storage);
    let mut report = String::from("round,mode,milliseconds\n");
    for round in 0..6 {
        let modes = if round % 2 == 0 {
            [true, false]
        } else {
            [false, true]
        };
        for checked in modes {
            let start = Instant::now();
            let storage = if checked {
                Storage::open_checked(&database).map_err(storage_error)?
            } else {
                Storage::open(&database).map_err(storage_error)?
            };
            drop(storage);
            let elapsed = start.elapsed().as_secs_f64() * 1000.0;
            let mode = if checked { "checked" } else { "ordinary" };
            let row = format!("{round},{mode},{elapsed:.3}\n");
            print!("{row}");
            report.push_str(&row);
        }
    }
    let storage = Storage::open(&database).map_err(storage_error)?;
    for index in [0, 7] {
        let media = storage
            .load_audio_media(&format!("synthetic-{index}"))
            .map_err(storage_error)?
            .ok_or("missing fixture")?;
        if media.2 != bytes {
            return Err("fixture bytes changed".into());
        }
    }
    std::fs::write(root.join("timing.csv"), report)?;
    println!(
        "400 MiB synthetic media; warm OS cache, alternating order. Evidence: {}",
        root.display()
    );
    Ok(())
}
