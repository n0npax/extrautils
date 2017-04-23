#![deny(warnings)]
extern crate coreutils;
extern crate extra;
extern crate termion;

#[derive(Debug, Copy, Clone)]
pub struct CalendarDay {
    pub year: i64,
    pub month: i64,
    pub day: i64,
    pub leap: bool,
    pub weekday: i64,
}

pub struct CalendarWeek {
    pub week_num: i64,
    pub days: Vec<CalendarDay>,
}
impl PartialEq for CalendarDay {
    fn eq(&self, other: &CalendarDay) -> bool {
        self.year == other.year && self.month == other.month && self.day == other.day && self.leap == other.leap && self.weekday == other.weekday
    }
}
impl CalendarDay {
    pub fn new(year: i64, month: i64, day: i64) -> CalendarDay {
        let mut this_date = CalendarDay {
            year: year,
            month: month,
            day: day,
            leap: false,
            weekday: -1,
        };
        this_date.update_leap();
        this_date.shift_month(0); //validate
        this_date.shift_day(0);
        this_date.update_weekday();
        this_date
    }

    pub fn update_leap(&mut self) {
        // https://en.wikipedia.org/wiki/Leap_year#Algorithm
        self.leap = (self.year % 4 == 0 && self.year % 400 == 0) || (self.year % 4 == 0 && self.year % 100 != 0);
    }

    pub fn shift_year(&mut self, shift: i64) {
        self.year += shift;
        self.update_weekday();
        self.update_leap();
    }

    pub fn shift_month(&mut self, shift: i64) {

        let new_month = self.month + shift;
        match new_month {
            13 => {
                self.month = 1;
                self.shift_year(1);
            }
            0 => {
                self.month = 12;
                self.shift_year(-1);
            }
            1...13 => {
                self.month = new_month;
            }
            _ => {
                self.shift_year(shift / 12);
                let nm = self.month + shift % 12;
                self.year += nm / 13;
                self.month = match nm % 12 {
                    0 => 12,
                    m => m,
                };
            }
        }
        self.update_weekday();
        self.update_leap();
    }


    pub fn shift_day(&mut self, shift: i64) {
        let days_in_month = self.get_month_days().len() as i64;

        self.day += shift;

        if days_in_month + 1 == self.day {
            self.shift_month(1);
            self.day = 1;
        } else if 0 == self.day {
            self.shift_month(-1);

            let dummy_active_date = CalendarDay::new(self.year, self.month, self.day);
            self.day = dummy_active_date.get_month_days().len() as i64;
        } else if self.day < 0 || self.day > days_in_month + 1 {
            // TODO need to implement if someone will need to shift 2+ days
        } else if self.day == -1 && shift == 0 {
            // hidden day
        } else if self.year == 1752 && self.month == 9 && self.day < 31 {
            if self.day > 2 && self.day < 14 {
                panic!("day error: {:?} (check gregor XIII reformation)", self);
            }
        } else if self.day < 0 || self.day > days_in_month {
            panic!("day error: {:?}", self);
        }
        self.update_weekday();
    }

    pub fn get_month_days(&self) -> Vec<i64> {
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
            return 366 - 12;
        } else if self.leap {
            return 366;
        }
        return 365;
    }

    pub fn update_weekday(&mut self) {
        // Sakamoto's methods -> https://en.wikipedia.org/wiki/Determination_of_the_day_of_the_week#Implementation-dependent_methods
        let (year, month, day) = (self.year, self.month, self.day);
        let sakamoto_array: [i64; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
        let mut m_year = year;
        m_year -= match month {
            0...3 => 1,
            _ => 0,
        };
        //Sakamoto method works fine for dates 1752+
        let mut george_shift = 0;
        if year < 1752 || year == 1752 && month < 10 {
            george_shift = 4;
        }
        self.weekday = ((m_year + m_year / 4 - m_year / 100 + m_year / 400 + sakamoto_array[(month - 1) as usize] + day + george_shift) % 7) as i64;
    }
}

impl CalendarWeek {
    pub fn new() -> CalendarWeek {
        CalendarWeek {
            week_num: -1,
            days: Vec::new(),
        }
    }
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
        let some_day = CalendarDay::new(1, month_num as i64 + 1, 1); // enumerate starts with 0, months with 1
        assert_eq!(some_day.get_month_days().len(), *days_num);
    }
}
#[test]
fn test_cal_days_in_year() {
    let george = CalendarDay::new(1752, 9, 1);
    assert_eq!(george.get_days_in_year(), 366 - 12);
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

    assert_eq!(before_george.weekday, 2);
    assert_eq!(after_george.weekday, 5);
    assert_eq!(epoch.weekday, 4);
    assert_eq!(warsaw_uprising.weekday, 2);
    assert_eq!(perestroika.weekday, 4);
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
    assert_ne!(some_flexible_day, some_day_12_5);
    some_flexible_day.shift_year(1);
    assert_eq!(some_flexible_day, some_day_12_5);

    some_flexible_day = CalendarDay::new(1998, 2, 5);
    let some_day_1999_8_5 = CalendarDay::new(1999, 8, 5);
    let some_day_2002_4_5 = CalendarDay::new(2004, 4, 5);

    some_flexible_day.shift_month(18);
    assert_eq!(some_flexible_day, some_day_1999_8_5);

    some_flexible_day.shift_month(12 * 5 - 4);
    assert_eq!(some_flexible_day, some_day_2002_4_5);
}
