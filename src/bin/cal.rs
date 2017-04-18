//#![deny(warnings)]

extern crate extra;
extern crate coreutils;
extern crate termion;

use std::env;
use std::io::{stdout, StdoutLock, stderr, Stderr, Write};
use std::process::exit;
use std::time::{SystemTime, UNIX_EPOCH};
use coreutils::{ArgParser, get_time_tuple};
use extra::option::OptionalExt;
use termion::color;

const MAN_PAGE: &'static str = r#"
NAME
   cal - display a calendar

SYNOPSIS
   cal [options] [[[day] month] year]
   cal [options] <timestamp>

DESCRIPTION
   cal  displays  a  simple  calendar.  If no arguments are specified, the current month is dis‐
   played.

   The month may be specified as a number (1-12), as a month name or  as  an  abbreviated  month
   name according to the current locales.

OPTIONS
   -1, --one
      Display single month output.  (This is the default.)

   -3, --three
      Display three months spanning the date.

   -monday
       monday as first day of week

   -sunday
       sunday as first day of week

    -h
    --help
        display this help and exit
"#; /* @MANEND */

#[derive(Debug, Copy, Clone)]
struct CalendarDay {
    year: i64,
    month: i8,
    day: i8,
    leap: bool,
    weekday: i8,
}

struct CalendarWeek {
    week_num: i8,
    month: String,
    days: Vec<CalendarDay>,
    mon_first: bool,
}

impl CalendarDay {
    pub fn new(year: i64, month: i8, day: i8) -> CalendarDay {
        let leap = (year%4==0 && year%400==0) || (year%4==0 && year%100!=0); // https://en.wikipedia.org/wiki/Leap_year#Algorithm
        let dummy_active_date = CalendarDay { year: year, month: month, day: day, leap: leap, weekday: -1};
        let weekday = get_week_day(get_days_since_epoch(dummy_active_date));
        CalendarDay {year: year, month: month, day: day, leap: leap, weekday: weekday}
    }
}

impl CalendarWeek {
    pub fn new() -> CalendarWeek {
        CalendarWeek {month : "".to_owned(), mon_first: false, week_num: -1, days: Vec::new() }
    }
}

fn print_calendar(active_date: CalendarDay, stdout: &mut StdoutLock, stderr: &mut Stderr) {

    let mut calendar_string = ["".to_owned(), "".to_owned(), "".to_owned(), "".to_owned(), "".to_owned(), "".to_owned()].to_vec();


    //for month_shift in -1..2 {
    for month_shift in 0..1 {
        let mut some_day = CalendarDay::new(active_date.year, active_date.month + month_shift, 1);

        let mut cw = CalendarWeek::new();
        for _ in 0..some_day.weekday {
            cw.days.push(CalendarDay::new(some_day.year, some_day.month, 0));
        }
        for day_num in get_month_days(some_day).iter() {
            cw.days.push(CalendarDay::new(some_day.year, some_day.month, (*day_num) as i8));
        }
        for _ in cw.days.len()..37 {
            cw.days.push(CalendarDay::new(some_day.year, some_day.month, 0));
        }

        for (week_num, week) in cw.days.chunks(7).enumerate() {
            let mut calendar_row = "".to_owned();
            for day in week {
                if day.day > 0 {
                    if day.day == active_date.day && day.month == active_date.month {
                        calendar_row.push_str(&format!("{}{}", color::Bg(color::White), color::Fg(color::Black)));
                    }
                    calendar_row.push_str(&format!("{:>5}({}) ", day.day, day.weekday));
                    calendar_row.push_str(&format!("{}{}", color::Bg(color::Reset), color::Fg(color::Reset)));
                } else {
                    calendar_row.push_str(&format!("{:>5} ", ""));
                }
            }
            calendar_string[week_num].push_str(&calendar_row);
            calendar_string[week_num].push_str("\t");
        }
    }
    stdout.write(calendar_string.join("\n").as_bytes()).try(stderr);
    stdout.write("\n".as_bytes()).try(stderr);
}


fn get_year_from_args(arg: &String) -> i64 {
    arg.parse::<i64>().expect("not a valid year")
}

fn get_month_from_args(arg: &String) -> i64 {
    let month = arg.parse::<i64>().expect("not a valid month");
    match month {
        1...12 => month,
        _ => panic!("month is an number 1 .. 12 "),
    }
}

fn get_day_from_args(arg: &String) -> i64 {
    let day = arg.parse::<i64>().expect("not a valid day");
    match day {
        1...31 => day,
        _ => panic!("day is an number 1 .. 31"),
    }
}

fn get_week_day(days_since_epoch: i64) -> i8 {
    // 0 sun
    match ((days_since_epoch+3)%7) as i8 {
        day_num @ 0...6 => day_num,
        day_num @ -6...-1 => 4-day_num.abs(),
        _ => panic!("can't determine weekday"),
    }
}

fn get_month_days(cal_date: CalendarDay) -> Vec<i64> {
    // range (1..3) produces 1,2, so months has ranges january has range 1..32
    if cal_date.year == 1752 && cal_date.month == 9 {
        return [1, 2, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30].to_vec();
    } else if cal_date.month == 4 || cal_date.month == 6 || cal_date.month == 9 || cal_date.month == 11 {
        return (1..31).collect();
    } else if cal_date.month == 1 || cal_date.month == 3 ||  cal_date.month == 5 || cal_date.month == 7 || cal_date.month == 8 || cal_date.month == 10 || cal_date.month == 12 {
        return (1..32).collect()
    } else if cal_date.month == 2 && cal_date.leap {
        return (1..30).collect();
    } else if cal_date.month == 2 {
        return (1..29).collect();
    } else {
        panic!("wrong date there's nothing like 31 Feb or 5 sep 1752. Your date {:?}", cal_date);
    }
}

fn days_in_year(cal_date: CalendarDay) -> i64 {
    if cal_date.year == 1752 {
        return 366-12
    } else if cal_date.leap {
        return 366
    } else {
        return 365
    }
}



fn get_days_since_epoch(cal_date: CalendarDay) -> i64 {
    let cal_epoch =  CalendarDay { year: 1970, month: 1, day: 1, leap: false, weekday: 4};
    get_days_since_date(cal_date, cal_epoch)
}

fn get_days_since_date(cal_date: CalendarDay, since_date: CalendarDay) -> i64 {

    fn get_way_pivot(way: i64) -> i64 {
        match way {
            0 => 0,
            way => -1 * way/way.abs(),
        }
    }

    // 01.01.1970 thu
    let mut year_shift = since_date.year - cal_date.year;
    let mut months_shift = since_date.month - cal_date.month;
    let days_shift = since_date.day - cal_date.day;

    let mut days = 0;
    let mut pivot_date = cal_date; // copy;
    pivot_date.day = 1; // may fail for 1752

    let way_pivot =  get_way_pivot(year_shift);
    while year_shift != 0 {
        year_shift+=way_pivot;
        pivot_date.year+=way_pivot;
        days+=days_in_year(pivot_date) * way_pivot;
    }

    let way_pivot =  get_way_pivot(months_shift as i64);
    while months_shift != 0 {
        months_shift+=way_pivot as i8;
        pivot_date.month+=way_pivot as i8;
        pivot_date.month = match pivot_date.month {
            0 => 12,
            13 => 1,
            r => r,
        };
        days+=get_month_days(pivot_date).len() as i64 * way_pivot;
    }

    days-=days_shift as i64;

    if cal_date.year < 1752 || ( cal_date.year == 1752 && cal_date.month<9) || (cal_date.year == 1752 && cal_date.month==9 && cal_date.day<14) {
        days-=12;
    }

    days
}


fn main() {
    let stdout = stdout();
    let mut stdout = stdout.lock();
    let mut stderr = stderr();

    let mut parser = ArgParser::new(8)
        .add_flag(&["h", "help"])
        .add_flag(&["1"])
        .add_flag(&["3"])
        .add_flag(&["m", "monday"])
        .add_flag(&["s", "sunday"]);

    parser.parse(env::args());

    if parser.found("help") {
        stdout.write(MAN_PAGE.as_bytes()).try(&mut stderr);
        stdout.flush().try(&mut stderr);
        exit(0);
    }

    let time = SystemTime::now();
    let duration = time.duration_since(UNIX_EPOCH).try(&mut stderr);
    let ts = duration.as_secs() as i64;

    let tz_offset = 0;
    let (mut year, mut month, mut day, _, _, _) = get_time_tuple(ts, tz_offset);
    if parser.args.len() > 0 {
        match parser.args.len() {
            0 => {},
            1 => {
                 year = get_year_from_args(&parser.args[0]);
             },
             2 => {
                year = get_year_from_args(&parser.args[1]);
                month = get_month_from_args(&parser.args[0]);
             },
             3 => {
                year = get_year_from_args(&parser.args[2]);
                month = get_month_from_args(&parser.args[1]);
                day = get_day_from_args(&parser.args[0]);
             },
             _ => {
                panic!("To many arguments!.\nUse --help to read man");
             }
        }
    }

    let active_date = CalendarDay::new(year, month as i8, day as i8);

//    stdout.write(&format!("{:?}\ni {:?}", get_month_days(active_date),active_date).as_bytes()).try(&mut stderr);

    print_calendar(active_date, &mut stdout, &mut stderr);

}
