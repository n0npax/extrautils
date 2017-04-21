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

   -n , --months number
        Display number of months, starting from the month containing the date.

   -y, --year
        Display a calendar for the whole year.

   -Y, --twelve
        Display a calendar for the next twelve months.

   -monday
       monday as first day of week

   -sunday
       sunday as first day of week

    -h
    --help
        display this help and exit
AUTHOR
    Marcin Niemira
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
impl PartialEq for CalendarDay {
    fn eq(&self, other: &CalendarDay) -> bool {
        self.year == other.year && self.month == other.month && self.day == other.day && self.leap == other.leap && self.weekday == other.weekday
    }
}
impl CalendarDay {
    pub fn new(year: i64, month: i8, day: i8) -> CalendarDay {
        let leap = (year % 4 == 0 && year % 400 == 0) || (year % 4 == 0 && year % 100 != 0); // https://en.wikipedia.org/wiki/Leap_year#Algorithm
        let mut some_date = CalendarDay {
            year: year,
            month: month,
            day: day,
            leap: leap,
            weekday: -1,
        };
        some_date.shift_month(0); //validate
        some_date.shift_day(0);
        some_date.update_weekday();
        some_date
    }

    pub fn shift_year(&mut self, shift: i64) {
        self.year += shift;
        self.update_weekday();
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
            _ => panic!("month error {:?}", self),
        }
        self.update_weekday();
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
        } else if self.day == -1 && shift == 0 {
            // hidden day
        } else if self.day < 0 || self.day > days_in_month {
            panic!("day error: {:?}", self);
        }
        self.update_weekday();
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


    pub fn get_days_in_year(&self) -> i64 {
        if self.year == 1752 {
            return 365 - 12;
        } else if self.leap {
            return 366;
        }
        return 365;
    }

    pub fn update_weekday(&mut self) {
        // Sakamoto's methods -> https://en.wikipedia.org/wiki/Determination_of_the_day_of_the_week#Implementation-dependent_methods
        let (y, m, d) = (self.year, self.month as i64, self.day as i64);
        let t: [i64; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
        let mut dy = y;
        dy -= match m {
            0 | 1 | 2 => 1,
            _ => 0,
        };
        // works fine for yar > 1752
        self.weekday = ((dy + dy / 4 - dy / 100 + dy / 400 + t[(m - 1) as usize] + d) % 7) as i8;
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

fn print_calendar(parser: &ArgParser, active_date: CalendarDay, stdout: &mut StdoutLock, stderr: &mut Stderr) {
    //let sunday_first = parser.found("sunday");
    let monday_first = parser.found("monday");

    let weekdays_names;
    let weekdays_complement;
    if monday_first {
        weekdays_complement = 1;
        weekdays_names = "Mo Tu We Th Fr Sa Su";
    } else {
        weekdays_complement = 0;
        weekdays_names = "Su Mo Tu We Th Fr Sa";
    }

    let mut months_range: Vec<_>;
    if parser.found("twelve") {
        months_range = (0..12).collect();
    } else if parser.found("year") {
        let active_month = active_date.month;
        months_range = (-active_month..(12 - active_month)).collect();
    } else if parser.found("three") {
        months_range = (-1..2).collect();
    } else if parser.found("months") {
        let n = parser.get_opt("n")
            .unwrap()
            .parse::<i8>()
            .unwrap();
        months_range = (0..n).collect();
    } else {
        months_range = (0..1).collect();
    }


    for months_range_max3 in months_range.chunks(3) {
        let mut calendar_string = ["".to_owned(), "".to_owned(), "".to_owned(), "".to_owned(), "".to_owned(), "".to_owned(), "".to_owned(), "".to_owned()].to_vec();
        for month_shift in months_range_max3 {
            //for month_shift in 0..1 {
            let mut some_day = CalendarDay::new(active_date.year, active_date.month, 1);
            some_day.shift_month(*month_shift);

            let mut cw = CalendarWeek::new();

            for _ in weekdays_complement..some_day.weekday {
                cw.days.push(CalendarDay::new(some_day.year, some_day.month, -1));
            }

            for day_num in some_day.get_month_days().iter() {
                cw.days.push(CalendarDay::new(some_day.year, some_day.month, (*day_num) as i8));
            }
            for _ in cw.days.len()..37 {
                cw.days.push(CalendarDay::new(some_day.year, some_day.month, -1));
            }


            let month_name = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
            calendar_string[0].push_str(&format!("{:>11} {:<12}", month_name[(some_day.month - 1) as usize], some_day.year));
            calendar_string[1].push_str(&format!("{:<23}", weekdays_names));


            for (week_num, week) in cw.days.chunks(7).enumerate() {
                let mut calendar_row = "".to_owned();

                for day in week {
                    if day.day > 0 {
                        if day.day == active_date.day && day.month == active_date.month {
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
        .add_flag(&["n", "months"])
        .add_flag(&["Y", "twelve"])
        .add_flag(&["y", "year"]);

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
                //    panic!("To many arguments!.\nUse --help to read man");
            }
        }
    }

    let active_date = CalendarDay::new(year, month as i8, day as i8);

    print_calendar(&parser, active_date, &mut stdout, &mut stderr);

}


#[test]
fn test_cal_shifts() {
    let mut some_flexible_day = CalendarDay::new(1998, 2, 5);
    let some_day_1_6 = CalendarDay::new(1998, 1, 5);
    let some_day_3_5 = CalendarDay::new(1998, 3, 5);
    let some_day_12_5 = CalendarDay::new(1998, 12, 5);
    assert_ne!(some_day_1_6, some_flexible_day);
    assert_ne!(some_flexible_day, some_day_3_5);
    assert_ne!(some_day_3_5, some_day_12_5);
    some_flexible_day.shift_month(1);
    assert_eq!(some_flexible_day, some_day_3_5);
    some_flexible_day.shift_month(-1);
    assert_ne!(some_day_1_6, some_flexible_day);
    some_flexible_day.shift_month(-1);
    assert_eq!(some_flexible_day, some_day_1_6);

    some_flexible_day.shift_month(-1);
    assert_ne!(some_flexible_day, some_day_12_5); // same day other year
    some_flexible_day.shift_year(1);
    assert_eq!(some_flexible_day, some_day_12_5);
}
#[test]
fn test_cal_days_shifts() {

    let mut some_flexible_day = CalendarDay::new(1999, 2, 5);
    let some_day_2_10 = CalendarDay::new(1999, 2, 10);
    let some_day_3_1 = CalendarDay::new(1999, 3, 1);
    some_flexible_day.shift_day(5);
    assert_eq!(some_day_2_10, some_flexible_day);
    some_flexible_day.shift_day(19);
    assert_eq!(some_day_3_1, some_flexible_day);
    // TODO shift from 22.02 -> 5.03

}

#[test]
fn test_cal_days_in_month() {
    let george = CalendarDay::new(1752, 9, 1);
    assert_eq!(george.get_month_days().len(), 31 - 12);
    let some_leap_year = CalendarDay::new(4, 2, 2);
    assert_eq!(some_leap_year.get_month_days().len(), 29);

    for (month_num, days_num) in [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31].into_iter().enumerate() {
        let some_day = CalendarDay::new(1, month_num as i8 + 1, 1); // enumerate starts with 0, months with 1
        assert_eq!(some_day.get_month_days().len(), *days_num);
    }
}
#[test]
fn test_cal_days_in_year() {
    let george = CalendarDay::new(1752, 9, 1);
    assert_eq!(george.get_days_in_year(), 365 - 12);
    let some_leap_year = CalendarDay::new(4, 4, 4);
    assert_eq!(some_leap_year.get_days_in_year(), 366);
    let some_non_leap_year = CalendarDay::new(3, 3, 3);
    assert_eq!(some_non_leap_year.get_days_in_year(), 365);
}

#[test]
fn test_weekdays() {
    let before_george = CalendarDay::new(1751, 1, 1);
    let after_george = CalendarDay::new(1753, 6, 8);
    let epoch = CalendarDay::new(1970, 1, 1);
    let warsaw_uprising = CalendarDay::new(1944, 8, 1);
    let perestroika = CalendarDay::new(1980, 5, 1);

    //assert_eq!(before_george.weekday, 2);
    //TODO
    assert_eq!(after_george.weekday, 5);
    assert_eq!(epoch.weekday, 4);
    assert_eq!(warsaw_uprising.weekday, 2);
    assert_eq!(perestroika.weekday, 4);
}
