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
        let leap = (year % 4 == 0 && year % 400 == 0) || (year % 4 == 0 && year % 100 != 0); // https://en.wikipedia.org/wiki/Leap_year#Algorithm
        let dummy_active_date = CalendarDay {
            year: year,
            month: month,
            day: day,
            leap: leap,
            weekday: -1,
        };
        let weekday = dummy_active_date.get_week_day();
        CalendarDay {
            year: year,
            month: month,
            day: day,
            leap: leap,
            weekday: weekday,
        }
    }

    pub fn shift_year(&mut self, shift: i64) {
        self.year += shift;
    }

    pub fn shift_month(&mut self, shift: i8) {
        self.month += shift;
        match self.month {
            13 => {
                self.month = 1;
                self.year += 1;
            }
            0 => {
                self.month = 12;
                self.year -= 1;
            }
            1...12 => {}
            _ => panic!("month shift failed"),
        }
    }


    pub fn shift_day(&mut self, shift: i8) {
        let days_in_month = self.get_month_days().len() as i8;

        self.day += shift;

        if days_in_month + 1 == self.day {
            self.shift_month(1);
            self.day = 1;
        } else if 0 == self.day {
            self.shift_month(-1);

            let dummy_active_date = CalendarDay {
                year: self.year,
                month: self.month,
                day: self.day,
                leap: self.leap,
                weekday: -1,
            };
            self.day = dummy_active_date.get_month_days().len() as i8;
        } else if self.day < 0 || self.day > days_in_month {
            panic!("day shift failed");
        }
    }

    pub fn get_month_days(&self) -> Vec<i64> {
        // range (1..3) produces 1,2, so months has ranges january has range 1..32
        if self.year == 1752 && self.month == 9 {
            return [1, 2, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30].to_vec();
        } else if self.month == 4 || self.month == 6 || self.month == 9 || self.month == 11 {
            return (1..31).collect();
        } else if self.month == 1 || self.month == 3 || self.month == 5 || self.month == 7 || self.month == 8 || self.month == 10 || self.month == 12 {
            return (1..32).collect();
        } else if self.month == 2 && self.leap {
            return (1..30).collect();
        } else if self.month == 2 {
            return (1..29).collect();
        } else {
            panic!("wrong date there's nothing like 31 Feb or 5 sep 1752. Your date {:?}", self);
        }
    }


    pub fn days_in_year(&self) -> i64 {
        if self.year == 1752 {
            return 365 - 12;
        } else if self.leap {
            return 366;
        }
        return 365;
    }

    pub fn get_week_day(&self) -> i8 {
        let days_since_epoch = self.get_days_since_epoch();
        if self.day == -1 {
            return -1
        }
        match ((days_since_epoch+3) % 7) as i8 {
            e @ 0...6 => e,
            _ => panic!("can't determine weekday ({}) \n-> {:?}", days_since_epoch, self),
        }
    }

    fn get_days_since_epoch(&self) -> i64 {
        let cal_epoch = CalendarDay {
            year: 1970,
            month: 1,
            day: 1,
            leap: false,
            weekday: 4,
        };
        self.get_days_since_date(cal_epoch)
    }

    fn get_days_since_date(&self, since_date: CalendarDay) -> i64 {

        fn get_way_pivot(way: i64) -> i64 {
            match way {
                0 => 0,
                way => -1 * way / way.abs(),
            }
        }

        let mut year_diff = self.year - since_date.year;
        let mut month_diff = self.month - since_date.month;
        let days_diff = self.day - since_date.day;

        let mut days = 0;
        let mut pivot_date = self.clone(); // copy;
        pivot_date.day = 1;

        let way_pivot = get_way_pivot(year_diff);
        while year_diff != 0 {
            year_diff += way_pivot;
            pivot_date.year += way_pivot;
            days += pivot_date.days_in_year() * way_pivot;
        }

        let way_pivot = get_way_pivot(month_diff as i64);
        while month_diff != 0 {
            month_diff += way_pivot as i8;
            pivot_date.shift_month(way_pivot as i8);
            days += pivot_date.get_month_days().len() as i64 * way_pivot;
        }

        days += days_diff as i64;

        if self.year < 1752 || (self.year == 1752 && self.month < 9) || (self.year == 1752 && self.month == 9 && self.day < 14) {
            days -= 12;
        }
        println!("{} || {:?}\n  || {:?}\n-----------",days, self, since_date);

        days
    }
}

impl CalendarWeek {
    pub fn new() -> CalendarWeek {
        CalendarWeek {
            month: "".to_owned(),
            mon_first: false,
            week_num: -1,
            days: Vec::new(),
        }
    }
}

fn print_calendar(active_date: CalendarDay, stdout: &mut StdoutLock, stderr: &mut Stderr) {

    let mut calendar_string = ["".to_owned(), "".to_owned(), "".to_owned(), "".to_owned(), "".to_owned(), "".to_owned()].to_vec();


    //for month_shift in -1..2 {
    for month_shift in 0..1 {
        let mut some_day = CalendarDay::new(active_date.year, active_date.month + month_shift, 1);

        let mut cw = CalendarWeek::new();
        for _ in 0..some_day.weekday {
            cw.days.push(CalendarDay::new(some_day.year, some_day.month, -1));
        }
        for day_num in some_day.get_month_days().iter() {
            cw.days.push(CalendarDay::new(some_day.year, some_day.month, (*day_num) as i8));
        }
        for _ in cw.days.len()..37 {
            cw.days.push(CalendarDay::new(some_day.year, some_day.month, -1));
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
            0 => {}
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

    let active_date = CalendarDay::new(year, month as i8, day as i8);

    //    stdout.write(&format!("{:?}\ni {:?}", get_month_days(active_date),active_date).as_bytes()).try(&mut stderr);

    print_calendar(active_date, &mut stdout, &mut stderr);

}
