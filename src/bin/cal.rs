#![deny(warnings)]

extern crate extra;
extern crate coreutils;
extern crate extrautils;
extern crate termion;

use std::env;
use std::io::{stdout, StdoutLock, stderr, Stderr, Write};
use std::process::exit;
use std::time::{SystemTime, UNIX_EPOCH};
use coreutils::{ArgParser, get_time_tuple};
use extra::option::OptionalExt;
use termion::color;

use extrautils::{CalendarDay, CalendarWeek};


const MAN_PAGE: &'static str = r#"
NAME
   cal - display a calendar

SYNOPSIS
   cal [options] [[[day] month] year]
   cal [options] <timestamp>

DESCRIPTION
   displays  a  simple  calendar.  If there's no agruments specified, it will display current date

   The month may be specified as a number (1-12).
   months accepts also 0 (previous year december) and 13 (next year january)

OPTIONS
   -1, --one
      Display single month output.  (default)

   -3, --three
      Display three months spanning the date.

   -n, --months number
      Display number of months, starts with current date.

   -y, --year
      Display a calendar for the whole year.

   -Y, --twelve
      Display a calendar for the next twelve months.

   -m, -monday
      Monday as first day

   -s, -sunday
      Sunday as first day

   -h, --help
       display this help and exit
AUTHOR
    Marcin Niemira
"#; /* @MANEND */

fn print_calendar(parser: &ArgParser, active_date: CalendarDay, stdout: &mut StdoutLock, stderr: &mut Stderr) {
    //let sunday_first = parser.found("sunday"); sunday as default
    let monday_first = parser.found("monday");

    let weekdays_names;
    let weekdays_complement;
    if monday_first {
        weekdays_complement = 0;
        weekdays_names = "Mo Tu We Th Fr Sa Su";
    } else {
        weekdays_complement = 1;
        weekdays_names = "Su Mo Tu We Th Fr Sa";
    }

    let months_range: Vec<_>;
    if parser.found("twelve") {
        months_range = (0..12).collect();
    } else if parser.found("year") || parser.args.len() > 0 {
        let active_month = active_date.month - 1;
        months_range = (-active_month..(12 - active_month)).collect();
    } else if parser.found("three") {
        months_range = (-1..2).collect();
    } else if parser.found("months") || parser.found(&'n') {
        let n = parser.get_opt("months")
            .expect("can't get param months/n")
            .parse::<i64>()
            .expect("can't convert param months/n into integer");
        months_range = (0..n).collect();
    } else {
        months_range = (0..1).collect();
    }


    for months_range_max3 in months_range.chunks(3) {
        let mut calendar_string = ["".to_owned(), "".to_owned(), "".to_owned(), "".to_owned(), "".to_owned(), "".to_owned(), "".to_owned(), "".to_owned()].to_vec();
        for month_shift in months_range_max3 {
            let mut some_day = CalendarDay::new(active_date.year, active_date.month, 1);
            some_day.shift_month(*month_shift);

            let mut some_week = CalendarWeek::new();

            // complement weeks with dummy days [_,_,_,1,2,3,4]

            match some_day.weekday +weekdays_complement {
                0 => {},
                e @ 1...7 => {
                    for _ in 1..e {
                        some_week.days.push(CalendarDay::new(some_day.year, some_day.month, -1));
                    }
                },
                q => panic!("shouldn't happen: {:?}", q),
            }
            for day_num in some_day.get_month_days().iter() {
                some_week.days.push(CalendarDay::new(some_day.year, some_day.month, (*day_num) as i64));
            }
            for _ in some_week.days.len()..37 {
                some_week.days.push(CalendarDay::new(some_day.year, some_day.month, -1));
            }


            let month_name = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
            calendar_string[0].push_str(&format!("{:>11} {:<12}", month_name[(some_day.month - 1) as usize], some_day.year));
            calendar_string[1].push_str(&format!("{:<23}", weekdays_names));


            for (week_num, week) in some_week.days.chunks(7).enumerate() {
                let mut calendar_row = "".to_owned();

                for day in week {
                    if day.day > 0 {
                        if day.day == active_date.day && day.month == active_date.month && day.year == active_date.year {
                            calendar_row.push_str(&format!("{}{}", color::Bg(color::White), color::Fg(color::Black)));
                        }
                        calendar_row.push_str(&format!("{:>2}", day.day));
                        calendar_row.push_str(&format!("{}{} ", color::Bg(color::Reset), color::Fg(color::Reset)));
                    } else {
                        calendar_row.push_str(&format!("{:>1} ", "  "));
                    }
                }
                calendar_string[week_num + 2].push_str(&format!("{:<21}", &calendar_row));
                calendar_string[week_num + 2].push_str("  ");
            }
        }

        stdout.write(calendar_string.join("\n").as_bytes()).try(stderr);
        stdout.write("\n\n".as_bytes()).try(stderr);
    }
}

fn get_year_from_args(arg: &String) -> i64 {
    arg.parse::<i64>().expect("not a valid year")
}

fn get_month_from_args(arg: &String) -> i64 {
    arg.parse::<i64>().expect("not a valid month")
}

fn get_day_from_args(arg: &String) -> i64 {
    arg.parse::<i64>().expect("not a valid day")
}

fn main() {
    let stdout = stdout();
    let mut stdout = stdout.lock();
    let mut stderr = stderr();

    let mut parser = ArgParser::new(8)
        .add_flag(&["h", "help"])
        .add_flag(&["1", "one"])
        .add_flag(&["3", "three"])
        .add_flag(&["m", "monday"])
        .add_flag(&["s", "sunday"])
        .add_flag(&["Y", "twelve"])
        .add_flag(&["y", "year"])
        .add_opt("n", "months");

    parser.parse(env::args());

    if parser.found("help") {
        stdout.write(MAN_PAGE.as_bytes()).try(&mut stderr);
        stdout.flush().try(&mut stderr);
        exit(0);
    }

    //today's date
    let time = SystemTime::now();
    let duration = time.duration_since(UNIX_EPOCH).try(&mut stderr);
    let ts = duration.as_secs() as i64;
    let tz_offset = 0;

    let (mut year, mut month, mut day, _, _, _) = get_time_tuple(ts, tz_offset);
    if parser.args.len() > 0 {
        match parser.args.len() {
            1 => {
                year = get_year_from_args(&parser.args[0]);
            }
            2 => {
                year = get_year_from_args(&parser.args[1]);
                month = get_month_from_args(&parser.args[0]);
            }
            3 => {
                year = get_year_from_args(&parser.args[2]);
                month = get_month_from_args(&parser.args[1]);
                day = get_day_from_args(&parser.args[0]);
            }
            _ => {
                panic!("To many arguments!.\nUse --help to read man");
            }
        }
    }

    let active_date = CalendarDay::new(year, month as i64, day as i64);

    print_calendar(&parser, active_date, &mut stdout, &mut stderr);

}
