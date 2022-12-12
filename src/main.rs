use anyhow::Result;
use chrono::{prelude::*, Duration};
use chrono_tz::Tz;
use clap::Parser;
use orgize::{elements::Title, Org};
use std::{borrow::Cow, ffi::OsString, fs, path::Path, str::FromStr};

#[derive(Parser)]
struct CheckOrgOptions {
    #[arg(long = "duration-mismatch", default_value_t = true)]
    report_duration_mismatch: bool,
    #[arg(long = "long-duration", default_value_t = true)]
    report_long_duration: bool,
    #[arg(long = "running-clock", default_value_t = true)]
    report_running_clock: bool,
    #[arg(long = "overlapping-clocks", default_value_t = true)]
    report_overlapping_clocks: bool,
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
    KnownLongDuration {file: "coscreen.org",duration: "10:41", title: "implement messaging on top of electrons window messaging api"},
    KnownLongDuration {file: "coscreen.org",duration: "8:23", title: "mojave user gets extra \"coscreen helper\" permission request"},
    KnownLongDuration {file: "coscreen.org",duration: "9:18", title: "single window picking"},
    KnownLongDuration {file: "coscreen.org",duration: "8:04", title: "[node-wrtc] capture window content"},
    KnownLongDuration {file: "coscreen.org",duration: "9:48", title: "[node-wrtc] capture window content"},
    KnownLongDuration {file: "coscreen.org",duration: "9:44", title: "i420 yuv conversion"},
    KnownLongDuration {file: "coscreen.org",duration: "11:29", title: "ACTIVE profiling support for coscreen native :remote-control:windows:"},
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
    KnownLongDuration {file: "projects.org", duration: "11:30", title: "[[file:~/projects/rust/homeautomation][homeautomation framework]]"},
    KnownLongDuration {file: "projects.org", duration: "9:19", title: "[[file:~/projects/rust/homeautomation][homeautomation framework]]"},
];

fn main() -> Result<()> {
    let opts = CheckOrgOptions::parse();

    let org_dir = std::env::home_dir().unwrap().join("org");

    let org_files = fs::read_dir(org_dir)?
        .into_iter()
        .filter_map(|file| {
            let file = file.ok()?;
            // println!("{file:?}");
            if file.file_type().ok()?.is_file()
                && file.path().extension() == Some(&OsString::from_str("org").ok()?)
            {
                Some(file.path())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    // let file = "c:/Users/robert/org/coscreen.org";

    let parsed = org_files
        .iter()
        .map(|file| fs::read_to_string(file).map(Org::parse_string))
        .collect::<std::io::Result<Vec<_>>>()?;

    let mut clocks = if opts.report_overlapping_clocks {
        Some(Vec::new())
    } else {
        None
    };

    for (file, org) in org_files.iter().zip(&parsed) {
        check_org(file, org, &opts, &mut clocks);
    }

    if let Some(clocks) = clocks {
        for (i, (start, end, file, title)) in clocks.iter().enumerate() {
            for (j, (start2, end2, file2, title2)) in clocks.iter().enumerate() {
                if i != j {
                    if start2 > start && start2 < end {
                        println!("OVERLAPPING TIME");
                        println!("  [{start}-{end}] {file:?} {title:?}");
                        println!("  [{start2}-{end2}] {file2:?} {title2:?}");
                    }
                }
            }
        }
    }

    Ok(())
}

fn check_org<'a>(
    file: impl AsRef<Path>,
    org: &'a Org,
    opts: &CheckOrgOptions,
    clocks: &mut Option<Vec<(DateTime<Tz>, DateTime<Tz>, String, String)>>,
) {
    let file = file.as_ref();
    let file_name = file.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let mut current_title = None;

    macro_rules! title {
        () => {
            if let Option::<&orgize::elements::Title>::Some(t) = &current_title {
                &t.raw
            } else {
                &Cow::Borrowed("")
            }
        };
    }

    for x in org.iter() {
        match x {
            orgize::Event::Start(el) => {
                match el {
                    orgize::Element::Clock(clock) => match clock {
                        orgize::elements::Clock::Closed {
                            start,
                            end,
                            duration,
                            ..
                        } => {
                            let (start, end) = start_end(start, end);
                            let d = end - start;
                            let actual_duration = duration_string(d);
                            if let Some(clocks) = clocks {
                                clocks.push((
                                    start,
                                    end,
                                    file_name.to_string(),
                                    title!().to_string(),
                                ));
                            }

                            if opts.report_duration_mismatch && duration != &actual_duration {
                                println!("[{file_name}] DURATION STRING DOES NOT MATCH: {:?} ({duration} vs {actual_duration})", title!());
                            };

                            if opts.report_long_duration {
                                let long_duration =
                                    opts.long_duration.unwrap_or_else(|| Duration::hours(10));
                                if d > long_duration {
                                    let allowed = KNOWN_LONG_DURATIONS.iter().any(|k| {
                                        file_name == k.file
                                            && current_title
                                                .as_ref()
                                                .map(|t| t.raw == k.title)
                                                .unwrap_or(false)
                                            && k.duration == duration
                                    });
                                    if !allowed {
                                        println!(
                                        "[{file_name}] LONG DURATION: {actual_duration} in {:?}",title!()
                                    );
                                    }
                                }
                            }
                        }
                        orgize::elements::Clock::Running { .. } => {
                            if opts.report_running_clock {
                                println!("[{file_name}] RUNNING CLOCK {:?}", title!());
                            }
                        }
                    },
                    orgize::Element::Title(title) => {
                        current_title = Some(title);
                    }

                    _ => {}
                };
            }
            _ => {}
        }
    }
}

fn start_end(
    start: &orgize::elements::Datetime,
    end: &orgize::elements::Datetime,
) -> (DateTime<Tz>, DateTime<Tz>) {
    let start: NaiveDateTime = start.into();
    let end: NaiveDateTime = end.into();
    if start > end {
        eprintln!("start/end in wrong order?");
    }
    let tz = if start
        < NaiveDateTime::parse_from_str("2019-05-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap()
    {
        chrono_tz::US::Pacific
    } else {
        chrono_tz::Europe::Berlin
    };
    let start = start.and_local_timezone(tz).unwrap();
    let end = end.and_local_timezone(tz).unwrap();
    (start, end)
}

fn duration_string(d: Duration) -> String {
    let hours = d.num_hours();
    let minutes = d.num_minutes() - hours * 60;
    format!("{hours}:{minutes:0>2}")
}
