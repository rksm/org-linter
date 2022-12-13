use anyhow::Result;
use chrono::Duration;
use clap::Parser;
use org_processing::{ClockConflict, FileChange, OrgDocument, OrgFile};
use std::{collections::HashSet, ffi::OsString, fs, io::BufRead, str::FromStr};

#[derive(Parser)]
#[command(about = "check your org files for stranger things")]
struct CheckOrgOptions {
    #[arg(long = "duration-mismatch", default_value_t = true)]
    report_duration_mismatch: bool,
    #[arg(long = "long-duration", default_value_t = true)]
    report_long_duration: bool,
    #[arg(long = "running-clock", default_value_t = true)]
    report_running_clock: bool,
    #[arg(long = "overlapping-clocks", default_value_t = true)]
    report_overlapping_clocks: bool,
    #[arg(long = "negative-duration", default_value_t = true)]
    report_negative_duration: bool,
    #[arg(long = "zero-clocks", default_value_t = true)]
    report_zero_clocks: bool,
    #[arg(long = "clock-conflicts", default_value_t = true)]
    report_clock_conflicts: bool,
    #[arg(long = "fix-clock-conflicts", default_value_t = false)]
    fix_clock_conflicts: bool,
    #[arg(value_parser = parse_duration_from_cli)]
    long_duration: Option<Duration>,
}

fn parse_duration_from_cli(s: &str) -> Result<Duration, String> {
    if let Some((h, m)) = s.split_once(':') {
        Ok(
            Duration::hours(h.parse().map_err(|_| "cannot parse hours".to_string())?)
                + Duration::minutes(m.parse().map_err(|_| "cannot parse minutes".to_string())?),
        )
    } else {
        Err("cannot parse duration".to_string())
    }
}

struct KnownLongDuration {
    file: &'static str,
    duration: &'static str,
    title: &'static str,
}

#[rustfmt::skip]
const KNOWN_LONG_DURATIONS: &[KnownLongDuration] = &[
    KnownLongDuration {file:"clockin.org", duration: "12:59", title: "privacy setup"},
    KnownLongDuration {file:"clockin.org", duration: "9:10", title: "ClojureD"},
    KnownLongDuration {file:"clockin.org", duration: "11:04", title: "Testing wasm"},
    KnownLongDuration {file:"clockin.org", duration: "10:08", title: "emacs python setup"},
    KnownLongDuration {file:"clockin.org", duration: "9:47", title: "[[file:books.org][organizing my books]]"},
    KnownLongDuration {file:"clockin.org", duration: "10:04", title: "Testing live reload with rust [[https://fasterthanli.me/articles/so-you-want-to-live-reload-rust][So you want to live-reload Rust - fasterthanli.me]]"},
    KnownLongDuration {file:"clockin.org", duration: "9:22", title: "blog post: how does bevy component query work?"},
    KnownLongDuration {file:"clockin.org", duration: "8:08", title: "blog post: setting up a Rust web / wasm project like it's 2022"},

    KnownLongDuration {file: "coscreen.org",duration: "9:30", title: "Create objective means to profile and determine end to end latency that users perceive when interacting with our user interface."},
    KnownLongDuration {file: "coscreen.org",duration: "10:41", title: "DONE implement messaging on top of electrons window messaging api"},
    KnownLongDuration {file: "coscreen.org",duration: "8:23", title: "mojave user gets extra \"coscreen helper\" permission request"},
    KnownLongDuration {file: "coscreen.org",duration: "9:18", title: "single window picking"},
    KnownLongDuration {file: "coscreen.org",duration: "8:04", title: "[node-wrtc] capture window content"},
    KnownLongDuration {file: "coscreen.org",duration: "9:48", title: "[node-wrtc] capture window content"},
    KnownLongDuration {file: "coscreen.org",duration: "9:44", title: "i420 yuv conversion"},
    KnownLongDuration {file: "coscreen.org",duration: "11:29", title: "ACTIVE profiling support for coscreen native"},
    KnownLongDuration {file: "coscreen.org",duration: "11:47", title: "Learning about GTK & libwebrtc screen capturing"},
    KnownLongDuration {file: "coscreen.org",duration: "8:13", title: "sending libwebrtc screen capture to browser"},
    KnownLongDuration {file: "coscreen.org",duration: "14:36", title: "testing native client with rust"},
    KnownLongDuration {file: "coscreen.org",duration: "9:02", title: "testing native client with rust"},
    KnownLongDuration {file: "coscreen.org",duration: "11:03", title: "fix oauth"},
    KnownLongDuration {file: "coscreen.org",duration: "9:46", title: "remote control for full desktop / display capturing macos"},
    KnownLongDuration {file: "coscreen.org",duration: "8:46", title: "remote control for full desktop / display capturing macos"},
    KnownLongDuration {file: "coscreen.org",duration: "9:51", title: "setup"},
    KnownLongDuration {file: "coscreen.org",duration: "8:28", title: "admin.coscreen.org: retention stats"},
    KnownLongDuration {file: "coscreen.org",duration: "11:21", title: "REVIEW Capture CPU/System and actual screen resolution info statistics to Cloudwatch at a regular interval. :Beta1.1:"},
    KnownLongDuration {file: "coscreen.org",duration: "11:14", title: "REVIEW Capture CPU/System and actual screen resolution info statistics to Cloudwatch at a regular interval. :Beta1.1:"},
    KnownLongDuration {file: "coscreen.org",duration: "8:53", title: "[[https://docs.google.com/spreadsheets/d/1ovnzpuIW7bY0Fexc8HDpfGdtni3ZXWKJqN7y5pviCZ8/edit#gid=0][Till's metrics]]"},
    KnownLongDuration {file: "coscreen.org",duration: "9:53", title: "[admin panel] Better reporting on teams & team activity"},
    KnownLongDuration {file: "coscreen.org",duration: "8:15", title: "call 2.0 refactoring"},
    KnownLongDuration {file: "coscreen.org",duration: "8:01", title: "invite link copying can take very long"},
    KnownLongDuration {file: "coscreen.org",duration: "8:39", title: "create & make use of @coscreen/backend"},
    KnownLongDuration {file: "coscreen.org",duration: "8:34", title: "[[file:~/projects/coscreen/coscreen-backend-rs][coscreen-backend-rs]]"},
    KnownLongDuration {file: "coscreen.org",duration: "14:23", title: "user stats with rust"},
    KnownLongDuration {file: "coscreen.org",duration: "8:07", title: "building an API prototype"},
    KnownLongDuration {file: "coscreen.org",duration: "14:48", title: "alerts based on firebase audit logs"},

    KnownLongDuration {file: "google.org", duration: "13:03", title: "[[https://drive.google.com/corp/drive/u/0/folders/1B1TWxkV-1Al8xX5KDu4l-tJlxWBsrgoV][Defcon 26 videos davidtomaschik@]]"},

    KnownLongDuration {file: "haskell.org", duration: "10:00", title: "coding-challenges"},

    KnownLongDuration {file: "private.org", duration: "9:43", title: "tax return 2020"},

    KnownLongDuration {file: "codium.org", duration: "8:27", title: "Codium go backend for genx realizer"},

    KnownLongDuration {file: "projects.org", duration: "8:18", title: "rust hot reloading"},
    KnownLongDuration {file: "projects.org", duration: "10:27", title: "[2020-02-21] Rust twitter fetch followers / [[file:~/projects/rust/star_counter][rust/star_counter]]"},
    KnownLongDuration {file: "projects.org", duration: "19:06", title: "twitter yet again 2020-11-29"},
    KnownLongDuration {file: "projects.org", duration: "9:55", title: "twitter yet again 2020-11-29"},
    KnownLongDuration {file: "projects.org", duration: "11:41", title: "twitter yet again 2020-11-29"},
    KnownLongDuration {file: "projects.org", duration: "8:21", title: "[2021-05-21] [[file:~/projects/python/twitter-viz/twipycli][python/twitter-viz/twipycli]] - twitter analysis with python"},
    KnownLongDuration {file: "projects.org", duration: "8:30", title: "[2022-04-30] twitter analysis one more time [[file:~/projects/rust/twitter-analyzer][rust/twitter-analyzer]]"},
    KnownLongDuration {file: "projects.org", duration: "21:09", title: "[2022-04-30] twitter analysis one more time [[file:~/projects/rust/twitter-analyzer][rust/twitter-analyzer]]"},
    KnownLongDuration {file: "projects.org", duration: "10:26", title: "lynn datenauswertung"},
    KnownLongDuration {file: "projects.org", duration: "10:21", title: "playing around with lisp twitter api via chirp / common lisp"},
    KnownLongDuration {file: "projects.org", duration: "10:57", title: "[[file:~/projects/rust/fritz-homeautomation][fritz rust app]]"},
    KnownLongDuration {file: "projects.org", duration: "12:49", title: "[[file:~/projects/rust/homeautomation][homeautomation framework]]"},
];

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    let opts = CheckOrgOptions::parse();

    #[allow(deprecated)]
    let org_dir = std::env::home_dir().unwrap().join("org");

    let files = fs::read_dir(&org_dir)?
        .into_iter()
        .filter_map(|file| {
            let file = file.ok()?;
            if file.file_type().ok()?.is_file()
                && file.path().extension() == Some(&OsString::from_str("org").ok()?)
            {
                Some(file.path())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    // let files = vec![std::path::PathBuf::from("/Users/robert/org/clockin.org")];
    // let files = vec![org_dir.join("test.org")];

    let org_files = files
        .iter()
        .map(OrgFile::from_file)
        .collect::<Result<Vec<_>>>()?;

    let docs = org_files.iter().map(|ea| ea.document()).collect::<Vec<_>>();

    // check docs
    for doc in &docs {
        check_org(doc, &opts);
    }

    // clock conflicts

    if opts.report_clock_conflicts {
        println!("finding clock conflicts...");
        for conflict in ClockConflict::find_conflicts(&docs) {
            println!("{}", conflict.report());
        }
    } else if opts.fix_clock_conflicts {
        let mut skipped = HashSet::new();
        'outer: loop {
            let org_files = files
                .iter()
                .map(OrgFile::from_file)
                .collect::<Result<Vec<_>>>()?;
            let docs = org_files.iter().map(|ea| ea.document()).collect::<Vec<_>>();
            for conflict in ClockConflict::find_conflicts(&docs) {
                let hash = conflict.hashme();
                if skipped.contains(&hash) {
                    continue;
                }
                println!("{}", conflict.report());
                let resolutions = conflict.resolution_options();
                let options = resolutions
                    .iter()
                    .enumerate()
                    .map(|(i, resolution)| (i, resolution.explanation()))
                    .collect::<Vec<_>>();

                println!("Select resolution:");

                for (i, expl) in options {
                    println!("  {i}) {expl}");
                }
                let mut stdin = std::io::stdin().lock();
                let selected = loop {
                    let mut input = String::new();
                    stdin.read_line(&mut input).expect("readline");
                    match input.trim().parse::<usize>() {
                        Ok(i) if i < resolutions.len() => break i,
                        _ => println!("invalid input"),
                    };
                };
                let resolution = resolutions.get(selected).expect("get resolution");
                let changes = conflict.resolve(*resolution);
                if !changes.is_empty() {
                    FileChange::apply(changes)?;
                    continue 'outer;
                } else {
                    skipped.insert(hash);
                }
            }

            break;
        }
    }

    Ok(())
}

fn check_org(doc: &OrgDocument, opts: &CheckOrgOptions) {
    let file_name = doc.file_name();
    let long_duration = opts.long_duration.unwrap_or_else(|| Duration::hours(10));

    for clock in &doc.clocks {
        let duration_string_raw = clock.duration_string.unwrap_or("");
        let duration_string = clock.duration_formatted();
        let headline = &doc.headlines[clock.parent];
        let title = headline.title;
        let line = clock.line;

        if opts.report_duration_mismatch && !clock.matches_duration() {
            println!("[{file_name}:{line}] DURATION STRING DOES NOT MATCH: {title:?} ({duration_string_raw} vs {duration_string})");
        };

        if opts.report_long_duration && clock.duration() > long_duration {
            let allowed = KNOWN_LONG_DURATIONS.iter().any(|k| {
                file_name.ends_with(k.file) && title == k.title && k.duration == duration_string
            });
            if !allowed {
                println!("[{file_name}:{line}] LONG DURATION: {duration_string} in {title:?}",);
            }
        }

        if opts.report_running_clock && clock.is_running() {
            println!("[{file_name}:{line}] RUNNING CLOCK {title:?}");
        }

        if opts.report_negative_duration && clock.duration() < Duration::zero() {
            println!("[{file_name}:{line}] NEGATIVE DURATION {title:?}: {duration_string}");
        }

        if opts.report_zero_clocks && clock.duration() == Duration::zero() && !clock.is_running() {
            println!("[{file_name}:{line}] ZERO DURATION {title:?}: {duration_string}");
        }
    }
}
