# org-linter

Checks your emacs org files. Currently mostly focused on soundness of org clocks.

## Usage

```
$ org-linter --help
Checks your org files for stranger things. Currently mostly focused on soundness of org clocks.

Usage: org-linter [OPTIONS]

Options:
      --report-long-durations          Report about clocks with a long duration. [default: true]
      --long-duration <LONG_DURATION>  Duration used for --report-long-durations. HH:MM format. [default: 10:00]
      --duration-mismatch              Report clocks whose duration is incorrect. [default: true]
      --report-running-clock           Report the clocks that have no end timestamp. [default: false]
      --negative-duration              Report clocks having a negative duration, i.e. the end timestamp is more recent than start. [default: true]
      --zero-clocks                    Report clocks whose start and end timestamp is the same. [default: true]
      --clock-conflicts                Report clock conflicts, i.e. clocks that overlap. [default: false]
      --fix-clock-conflicts            Interactively fix conflicted clocks. Goes through the clocks one by one and allows you to choose a resolution. [default: false]
      --org-dir <ORG_DIR>              The org directory that contains the org files. [default: /Users/robert.krahn/org]
      --recursive                      Recursively find .org files in --org-dir. [default: true]
      --org-file <ORG_FILES>           Specify individual org files to lint. Overrides --org-dir.
  -h, --help                           Print help information
```
